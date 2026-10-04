// src/process.rs
use crate::ybc_vm::YbcVm;
use crate::ybc;

pub const MAX_PROCESSES: usize = 8;
static mut EXIT_REQUESTED: Option<i32> = None;

#[allow(dead_code)]
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
    pub data: [u8; 65536],
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
pub static mut CURRENT_PID: Option<usize> = None;

pub fn validate_user_range(ptr: u64, len: u64) -> bool {
    unsafe {
        let pid = match CURRENT_PID {
            Some(p) => p,
            None => return false,
        };

        if pid >= MAX_PROCESSES {
            return false;
        }

        let proc = &PROCESSES[pid];
        let base = proc.data.as_ptr() as u64;
        let end = base + proc.data_len as u64;

        if ptr == 0 {
            return false;
        }

        if ptr < base || ptr >= end {
            return false;
        }

        if ptr + len > end {
            return false;
        }

        true
    }
}

/// Process đang chạy có đúng tên package này không (dùng cho kiểm tra quyền).
pub fn current_is(name: &str) -> bool {
    unsafe {
        match CURRENT_PID {
            Some(p) if p < MAX_PROCESSES => {
                let pr = &PROCESSES[p];
                &pr.name[..pr.name_len] == name.as_bytes()
            }
            _ => false,
        }
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

pub fn request_exit(code: i32) {
    unsafe { EXIT_REQUESTED = Some(code); }
}

/// App thoát (bình thường hoặc lỗi) -> trả màn hình về vtty.
/// Không bật demo của kernel: demo chỉ dành cho lệnh `gfx` / F1.
unsafe fn return_to_console() {
    crate::graphics::gfx::set_demo_mode(false);
    if crate::console::get_display_mode() == crate::console::DisplayMode::Graphics {
        crate::graphics::gfx::clear_buffers();
        crate::graphics::gfx::exit_graphics_mode(); // restore console + set Console mode
    }
}

pub fn run_to_completion(pid: usize) -> Result<(), &'static str> {
    unsafe {
        EXIT_REQUESTED = None;
        let proc_ptr = &mut PROCESSES[pid] as *mut Process;
        let proc = &mut *proc_ptr;
        if proc.state != ProcState::Running {
            return Err("process not running");
        }

        let _header = ybc::parse_header(&proc.data[..proc.data_len])
            .ok_or("header parse failed")?;

        // Cho phép exec lồng nhau (shell app chạy app khác): nhớ process cha
        // và chế độ màn hình lúc bắt đầu để trả lại đúng khi kết thúc.
        let prev_pid = CURRENT_PID;
        let start_mode = crate::console::get_display_mode();

        CURRENT_PID = Some(pid);

        let mut vm = match YbcVm::new(&proc.data[..proc.data_len]) {
            Ok(v) => v,
            Err(e) => {
                proc.state = ProcState::Unused;
                CURRENT_PID = prev_pid;
                return Err(e);
            }
        };

        // FIX: log VM error thay vì nuốt. App chết phải để lại dấu vết.
        match vm.run() {
            Ok(_exit_code) => {
                crate::serial::serial_write_str("PROC: app exited normally\r\n");
            }
            Err(e) => {
                crate::serial::serial_write_str("PROC: VM ERROR: ");
                crate::serial::serial_write_str(e);
                crate::serial::serial_write_str("\r\n");
            }
        }

        // Giải phóng slot, nếu không sau 8 lần chạy sẽ hết process slot.
        proc.state = ProcState::Unused;
        CURRENT_PID = prev_pid;

        // Thoát app -> về vtty, nhưng chỉ khi app được chạy từ vtty. Nếu cha đang
        // ở Graphics (ví dụ desktop chạy app con) thì không đụng vào màn hình.
        if start_mode == crate::console::DisplayMode::Console {
            return_to_console();
        }

        Ok(())
    }
}