// src/process.rs
use crate::ybc_vm::YbcVm;
use crate::ybc::{self, YbcHeader};

pub const MAX_PROCESSES: usize = 8;

#[derive(Clone, Copy, PartialEq)]
pub enum ProcState {
    Unused,
    Running,
    Exited,
}

pub struct Process {
    pub state: ProcState,
    pub name: [u8; 32],
    pub name_len: usize,
    pub data: [u8; 65536],   // buffer chứa toàn bộ .ybc đã copy vào (đơn giản hoá, chưa paging riêng)
    pub data_len: usize,
    pub pc_saved: usize,
    pub steps_budget: u32,
}

impl Process {
    const fn new() -> Self {
        Self {
            state: ProcState::Unused,
            name: [0; 32],
            name_len: 0,
            data: [0; 65536],
            data_len: 0,
            pc_saved: 0,
            steps_budget: 2000,
        }
    }
}

static mut PROCESSES: [Process; MAX_PROCESSES] = [const { Process::new() }; MAX_PROCESSES];
static mut CURRENT_PID: Option<usize> = None;

/// Validate 1 vùng con trỏ nằm trong buffer .ybc của process hiện tại đang chạy syscall.
/// Placeholder cho tới khi có paging per-process thật; hiện dùng bounds-check trên buffer copy.
pub fn validate_user_range(ptr: u64, len: u64) -> bool {
    unsafe {
        let pid = match CURRENT_PID {
            Some(p) => p,
            None => return false,
        };
        let proc = &PROCESSES[pid];
        let base = proc.data.as_ptr() as u64;
        let end = base + proc.data_len as u64;
        ptr >= base && ptr.checked_add(len).map_or(false, |e| e <= end)
    }
}

pub fn spawn_ybc(name: &str, ybc_bytes: &[u8]) -> Result<usize, &'static str> {
    if ybc_bytes.len() > 65536 {
        return Err("file too large for process buffer");
    }

    ybc::validate_ybc(ybc_bytes)?;

    unsafe {
        for i in 0..MAX_PROCESSES {
            if PROCESSES[i].state == ProcState::Unused {
                let proc = &mut PROCESSES[i];
                let nb = name.as_bytes();
                let nlen = nb.len().min(31);
                proc.name[..nlen].copy_from_slice(&nb[..nlen]);
                proc.name_len = nlen;

                proc.data[..ybc_bytes.len()].copy_from_slice(ybc_bytes);
                proc.data_len = ybc_bytes.len();
                proc.pc_saved = 0;
                proc.state = ProcState::Running;

                return Ok(i);
            }
        }
    }
    Err("no free process slot")
}

/// Chạy 1 process cho tới khi Halt hoặc lỗi — chạy đồng bộ (blocking) trong bản đầu tiên,
/// chưa có scheduler đa nhiệm thật; đây là bước "chạy được trước, cô lập/đa nhiệm sau"
pub fn run_to_completion(pid: usize) -> Result<(), &'static str> {
    unsafe {
        let proc_ptr = &mut PROCESSES[pid] as *mut Process;
        let proc = &mut *proc_ptr;
        if proc.state != ProcState::Running {
            return Err("process not running");
        }

        let header = ybc::parse_header(&proc.data[..proc.data_len])
            .ok_or("header parse failed after validate (unexpected)")?;

        CURRENT_PID = Some(pid);

        let mut vm = YbcVm::new(&proc.data[..proc.data_len], header);

        loop {
            match vm.run(5000) {
                Ok(true) => break,   // Halt
                Ok(false) => continue, // hết timeslice, chạy tiếp (chưa có scheduler nhường CPU thật)
                Err(_) => {
                    crate::println!("Process '{}' crashed (VM error)", core::str::from_utf8(&proc.name[..proc.name_len]).unwrap_or("?"));
                    break;
                }
            }
        }

        proc.state = ProcState::Exited;
        CURRENT_PID = None;
        Ok(())
    }
}