// src/ybc_vm.rs — chạy trong Ring 3, no_std, KHÔNG truy cập trực tiếp phần cứng
// Mọi I/O đi qua sys_call() -> int 0x80, kernel bên kia validate lại tham số
use crate::ybc::{Op, SysCallId, YbcHeader, YBC_HEADER_SIZE};

const MAX_STACK: usize = 256;
const MAX_LOCALS: usize = 64;

pub struct YbcVm<'a> {
    code: &'a [u8],
    strings: &'a [u8],
    stack: [i64; MAX_STACK],
    sp: usize,
    locals: [i64; MAX_LOCALS],
    pc: usize,
}

pub enum VmError {
    StackOverflow,
    StackUnderflow,
    DivByZero,
    BadOpcode,
    Halted,
}

impl<'a> YbcVm<'a> {
    pub fn new(data: &'a [u8], header: &YbcHeader) -> Self {
        let code_start = YBC_HEADER_SIZE;
        let code_end = code_start + header.code_len as usize;
        let str_end = code_end + header.string_pool_len as usize;

        Self {
            code: &data[code_start..code_end],
            strings: &data[code_end..str_end],
            stack: [0; MAX_STACK],
            sp: 0,
            locals: [0; MAX_LOCALS],
            pc: header.entry_offset as usize,
        }
    }

    fn push(&mut self, v: i64) -> Result<(), VmError> {
        if self.sp >= MAX_STACK { return Err(VmError::StackOverflow); }
        self.stack[self.sp] = v;
        self.sp += 1;
        Ok(())
    }

    fn pop(&mut self) -> Result<i64, VmError> {
        if self.sp == 0 { return Err(VmError::StackUnderflow); }
        self.sp -= 1;
        Ok(self.stack[self.sp])
    }

    fn read_u8(&mut self) -> u8 {
        let v = self.code[self.pc];
        self.pc += 1;
        v
    }

    fn read_u16(&mut self) -> u16 {
        let v = u16::from_le_bytes([self.code[self.pc], self.code[self.pc + 1]]);
        self.pc += 2;
        v
    }

    fn read_u32(&mut self) -> u32 {
        let v = u32::from_le_bytes([
            self.code[self.pc], self.code[self.pc + 1],
            self.code[self.pc + 2], self.code[self.pc + 3],
        ]);
        self.pc += 4;
        v
    }

    fn read_i64(&mut self) -> i64 {
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&self.code[self.pc..self.pc + 8]);
        self.pc += 8;
        i64::from_le_bytes(buf)
    }

    /// Chạy tối đa `max_steps` instruction rồi trả về — chống vòng lặp vô hạn
    /// chiếm CPU: kernel gọi run() theo timeslice, không chạy tới khi Halt bất chấp
    pub fn run(&mut self, max_steps: u32) -> Result<bool, VmError> {
        let mut steps = 0;
        while steps < max_steps {
            if self.pc >= self.code.len() {
                return Err(VmError::BadOpcode);
            }
            let opcode = self.read_u8();
            let op = Op::from_u8(opcode).ok_or(VmError::BadOpcode)?;

            match op {
                Op::Nop => {}
                Op::PushInt => {
                    let v = self.read_i64();
                    self.push(v)?;
                }
                Op::PushStr => {
                    let idx = self.read_u16();
                    self.push(idx as i64)?; // string index, syscall Print sẽ resolve
                }
                Op::Pop => { self.pop()?; }
                Op::Dup => {
                    let v = self.stack[self.sp - 1];
                    self.push(v)?;
                }
                Op::Add => { let b = self.pop()?; let a = self.pop()?; self.push(a + b)?; }
                Op::Sub => { let b = self.pop()?; let a = self.pop()?; self.push(a - b)?; }
                Op::Mul => { let b = self.pop()?; let a = self.pop()?; self.push(a * b)?; }
                Op::Div => {
                    let b = self.pop()?; let a = self.pop()?;
                    if b == 0 { return Err(VmError::DivByZero); }
                    self.push(a / b)?;
                }
                Op::Lt => { let b = self.pop()?; let a = self.pop()?; self.push((a < b) as i64)?; }
                Op::Gt => { let b = self.pop()?; let a = self.pop()?; self.push((a > b) as i64)?; }
                Op::Eq => { let b = self.pop()?; let a = self.pop()?; self.push((a == b) as i64)?; }
                Op::Not => { let a = self.pop()?; self.push((a == 0) as i64)?; }
                Op::Jmp => {
                    let target = self.read_u32();
                    self.pc = target as usize;
                }
                Op::JmpIfFalse => {
                    let target = self.read_u32();
                    let cond = self.pop()?;
                    if cond == 0 { self.pc = target as usize; }
                }
                Op::LoadLocal => {
                    let slot = self.read_u8();
                    self.push(self.locals[slot as usize])?;
                }
                Op::StoreLocal => {
                    let slot = self.read_u8();
                    let v = self.pop()?;
                    self.locals[slot as usize] = v;
                }
                Op::CallSys => {
                    let sys_id = self.read_u16();
                    let argc = self.read_u8();
                    self.dispatch_syscall(sys_id, argc)?;
                }
                Op::Halt => return Ok(true),
            }
            steps += 1;
        }
        Ok(false) // hết timeslice, chưa Halt -> kernel gọi lại run() sau
    }

    fn dispatch_syscall(&mut self, sys_id: u16, argc: u8) -> Result<(), VmError> {
        let id = SysCallId::from_u16(sys_id).ok_or(VmError::BadOpcode)?;

        // Lấy args ra khỏi VM stack, gói lại rồi gọi thẳng int 0x80 —
        // KHÔNG bao giờ truy cập phần cứng trực tiếp từ đây (đây vẫn là code Ring 3)
        let mut args = [0i64; 5];
        for i in (0..argc as usize).rev() {
            args[i] = self.pop()?;
        }

        match id {
            SysCallId::Print => {
                let str_idx = args[0] as usize;
                sys_print_str_from_pool(self.strings, str_idx);
            }
            SysCallId::DrawRect => {
                sys_draw_rect(args[0], args[1], args[2], args[3], args[4]);
            }
            SysCallId::GetTick => {
                let t = sys_get_tick();
                self.push(t)?;
            }
            SysCallId::Sleep => {
                sys_sleep(args[0] as u64);
            }
            SysCallId::Exit => {
                sys_exit(args[0] as i32);
            }
        }
        Ok(())
    }
}

// === Syscall wrappers — mỗi hàm chỉ làm 1 việc: gói tham số, gọi int 0x80 ===
// Đây là code chạy Ring 3, tuyệt đối không có asm outb/inb ở tầng này

#[inline(always)]
fn syscall(id: u64, a1: u64, a2: u64, a3: u64, a4: u64, a5: u64) -> i64 {
    let ret: i64;
    unsafe {
        core::arch::asm!(
            "int 0x80",
            inlateout("rax") id => ret,
            in("rdi") a1, in("rsi") a2, in("rdx") a3,
            in("r10") a4, in("r8") a5,
            options(nostack)
        );
    }
    ret
}

fn sys_print_str_from_pool(strings: &[u8], idx: usize) {
    // Chuỗi trong pool được kernel validate biên khi load .ybc,
    // nhưng vẫn kiểm tra lại idx phòng lỗi logic VM
    if idx >= strings.len() { return; }
    // format pool: [u16 len][bytes...] liên tiếp theo idx offset trực tiếp
    // (compiler host-side đảm bảo idx là offset hợp lệ trỏ đúng đầu 1 entry)
    if idx + 2 > strings.len() { return; }
    let len = u16::from_le_bytes([strings[idx], strings[idx + 1]]) as usize;
    let start = idx + 2;
    if start + len > strings.len() { return; }
    let ptr = strings[start..start + len].as_ptr() as u64;
    syscall(1 /* SYS_PRINT */, ptr, len as u64, 0, 0, 0);
}

fn sys_draw_rect(x: i64, y: i64, w: i64, h: i64, color: i64) {
    syscall(2, x as u64, y as u64, w as u64, h as u64, color as u64);
}

fn sys_get_tick() -> i64 {
    syscall(3, 0, 0, 0, 0, 0)
}

fn sys_sleep(ms: u64) {
    syscall(4, ms, 0, 0, 0, 0);
}

fn sys_exit(code: i32) -> ! {
    syscall(5, code as u64, 0, 0, 0, 0);
    loop { unsafe { core::arch::asm!("hlt"); } }
}