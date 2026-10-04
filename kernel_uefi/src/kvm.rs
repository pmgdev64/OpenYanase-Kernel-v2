// src/kvm.rs
// Kernel VM — điểm thực thi DUY NHẤT cho mọi bytecode driver.
// Ring-1 sandbox: driver không thể outb/inb trực tiếp, mọi I/O qua capability
// whitelist. Bytecode phải là &'static — initrd là vùng nhớ vĩnh viễn.

use crate::ybc::{Op, YbcHeader, YBC_HEADER_SIZE};
use crate::kvm_guard::{CapToken, Capabilities, check};

pub struct KernelVm {
    code: &'static [u8],
    strings: &'static [u8],
    stack: [i64; 256],
    sp: usize,
    locals: [i64; 64],
    pc: usize,
    token: CapToken,
    caps: Capabilities,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KvmError {
    StackFault,
    BadOpcode,
    CapabilityDenied,
    PortNotWhitelisted,
    Halted,
    TokenRevoked,
}

impl KernelVm {
    pub fn new(
        data: &'static [u8],
        header: &YbcHeader,
        token: CapToken,
    ) -> Result<Self, KvmError> {
        let caps = check(token).ok_or(KvmError::CapabilityDenied)?;

        let code_start = YBC_HEADER_SIZE;
        let code_end = code_start + header.code_len as usize;
        let str_end = code_end + header.string_pool_len as usize;

        if data.len() < str_end {
            return Err(KvmError::BadOpcode);
        }

        Ok(Self {
            code: &data[code_start..code_end],
            strings: &data[code_end..str_end],
            stack: [0; 256],
            sp: 0,
            locals: [0; 64],
            pc: header.entry_offset as usize,
            token,
            caps,
        })
    }

    /// Public accessor — dùng cho telemetry nếu cần.
    pub fn pc(&self) -> usize { self.pc }
    pub fn sp(&self) -> usize { self.sp }

    fn push(&mut self, v: i64) -> Result<(), KvmError> {
        if self.sp >= 256 { return Err(KvmError::StackFault); }
        self.stack[self.sp] = v;
        self.sp += 1;
        Ok(())
    }

    fn pop(&mut self) -> Result<i64, KvmError> {
        if self.sp == 0 { return Err(KvmError::StackFault); }
        self.sp -= 1;
        Ok(self.stack[self.sp])
    }

    fn do_io_port(&mut self, is_write: bool, port: u16, value: u8) -> Result<u8, KvmError> {
        if !self.caps.can_io_port {
            return Err(KvmError::CapabilityDenied);
        }
        let mut allowed = false;
        for i in 0..self.caps.allowed_port_count as usize {
            let (lo, hi) = self.caps.allowed_port_ranges[i];
            if port >= lo && port <= hi { allowed = true; break; }
        }
        if !allowed {
            return Err(KvmError::PortNotWhitelisted);
        }
        unsafe {
            if is_write {
                crate::cpu::outb(port, value);
                Ok(0)
            } else {
                Ok(crate::cpu::inb(port))
            }
        }
    }

    fn do_draw(&mut self, x: i64, y: i64, w: i64, h: i64, color: i64) -> Result<(), KvmError> {
        if !self.caps.can_draw { return Err(KvmError::CapabilityDenied); }
        let x = x.clamp(0, 4096) as u32;
        let y = y.clamp(0, 4096) as u32;
        let w = w.clamp(0, 4096) as u32;
        let h = h.clamp(0, 4096) as u32;
        unsafe {
            if let Some(surface) = crate::graphics::gfx::get_back_surface() {
                surface.fill_rect(x, y, w, h, crate::gop::Color(color as u32));
                crate::graphics::gfx::mark_dirty();
            }
        }
        Ok(())
    }

    fn do_print(&mut self, str_idx: usize) -> Result<(), KvmError> {
        if !self.caps.can_print { return Err(KvmError::CapabilityDenied); }
        if str_idx + 2 > self.strings.len() { return Ok(()); }
        let len = u16::from_le_bytes([self.strings[str_idx], self.strings[str_idx + 1]]) as usize;
        let start = str_idx + 2;
        if start + len > self.strings.len() { return Ok(()); }
        if let Ok(s) = core::str::from_utf8(&self.strings[start..start + len]) {
            use core::fmt::Write;
            unsafe { let _ = crate::console::CONSOLE.write_str(s); }
        }
        Ok(())
    }

    /// Chạy tối đa `max_steps` opcode (bị clamp bởi caps.max_steps_per_slice).
    ///   Ok(true)  — Halt: driver tự shutdown
    ///   Ok(false) — hết budget, còn chạy được
    ///   Err(...)  — trap / vi phạm capability / token bị revoke
    pub fn run(&mut self, max_steps: u32) -> Result<bool, KvmError> {
        let budget = max_steps.min(self.caps.max_steps_per_slice);
        let mut steps = 0;

        while steps < budget {
            if self.pc >= self.code.len() {
                return Err(KvmError::BadOpcode);
            }

            // Runtime token check — cho phép revoke từ ngoài
            if check(self.token).is_none() {
                return Err(KvmError::TokenRevoked);
            }

            let opcode = self.code[self.pc];
            self.pc += 1;
            let op = Op::from_u8(opcode).ok_or(KvmError::BadOpcode)?;

            match op {
                Op::Halt => return Ok(true),
                Op::PushInt => {
                    let mut buf = [0u8; 8];
                    buf.copy_from_slice(&self.code[self.pc..self.pc + 8]);
                    self.pc += 8;
                    self.push(i64::from_le_bytes(buf))?;
                }
                Op::Pop => { let _ = self.pop(); }
                Op::Add => { let b = self.pop()?; let a = self.pop()?; self.push(a + b)?; }
                Op::Sub => { let b = self.pop()?; let a = self.pop()?; self.push(a - b)?; }
                Op::Mul => { let b = self.pop()?; let a = self.pop()?; self.push(a.wrapping_mul(b))?; }
                Op::Div => {
                    let b = self.pop()?; let a = self.pop()?;
                    if b == 0 { return Err(KvmError::BadOpcode); }
                    self.push(a / b)?;
                }
                Op::Lt => { let b = self.pop()?; let a = self.pop()?; self.push(if a < b { 1 } else { 0 })?; }
                Op::Gt => { let b = self.pop()?; let a = self.pop()?; self.push(if a > b { 1 } else { 0 })?; }
                Op::Eq => { let b = self.pop()?; let a = self.pop()?; self.push(if a == b { 1 } else { 0 })?; }
                Op::Not => { let a = self.pop()?; self.push(if a == 0 { 1 } else { 0 })?; }
                Op::JmpIfFalse => {
                    let target = u32::from_le_bytes(
                        self.code[self.pc..self.pc + 4].try_into().unwrap(),
                    ) as usize;
                    self.pc += 4;
                    let cond = self.pop()?;
                    if cond == 0 {
                        if target >= self.code.len() { return Err(KvmError::BadOpcode); }
                        self.pc = target;
                    }
                }
                Op::Jmp => {
                    let target = u32::from_le_bytes(
                        self.code[self.pc..self.pc + 4].try_into().unwrap(),
                    ) as usize;
                    self.pc += 4;
                    if target >= self.code.len() { return Err(KvmError::BadOpcode); }
                    self.pc = target;
                }
                Op::CallSys => {
                    if self.pc + 3 > self.code.len() { return Err(KvmError::BadOpcode); }
                    let sys_id = u16::from_le_bytes(
                        [self.code[self.pc], self.code[self.pc + 1]],
                    );
                    self.pc += 2;
                    let argc = self.code[self.pc];
                    self.pc += 1;

                    let mut args = [0i64; 5];
                    for i in (0..argc as usize).rev() {
                        if i < 5 { args[i] = self.pop()?; } else { self.pop()?; }
                    }

                    match sys_id {
                        1 => self.do_print(args[0] as usize)?,
                        2 => self.do_draw(args[0], args[1], args[2], args[3], args[4])?,
                        100 => { self.do_io_port(true, args[0] as u16, args[1] as u8)?; }
                        101 => { let v = self.do_io_port(false, args[0] as u16, 0)?; self.push(v as i64)?; }
                        102 => { self.push(crate::timer::get_ticks() as i64)?; }
                        103 => { self.push(0)?; }
                        _ => return Err(KvmError::CapabilityDenied),
                    }
                }
                _ => { /* opcode chưa support — bỏ qua an toàn */ }
            }
            steps += 1;
        }
        Ok(false)
    }
}