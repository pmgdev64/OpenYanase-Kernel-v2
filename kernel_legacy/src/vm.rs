// src/vm.rs
// Stack-based bytecode VM tối giản, an toàn: mọi truy cập stack/memory/code
// đều bound-checked.

pub const MAX_STACK: usize = 256;
pub const MAX_CALLSTACK: usize = 64;
pub const MAX_MEM: usize = 16 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trap {
    StackOverflow,
    StackUnderflow,
    CallStackOverflow,
    CallStackUnderflow,
    MemOutOfBounds,
    CodeOutOfBounds,
    DivByZero,
    UnknownOpcode(u8),
    UnknownSyscall(u8),
    Halted,
}

// Opcode
pub const OP_NOP: u8 = 0x00;
pub const OP_PUSH_I32: u8 = 0x01;
pub const OP_POP: u8 = 0x02;
pub const OP_DUP: u8 = 0x03;
pub const OP_ADD: u8 = 0x04;
pub const OP_SUB: u8 = 0x05;
pub const OP_MUL: u8 = 0x06;
pub const OP_DIV: u8 = 0x07;
pub const OP_EQ: u8 = 0x08;
pub const OP_LT: u8 = 0x09;
pub const OP_GT: u8 = 0x0A;
pub const OP_JMP: u8 = 0x0B;
pub const OP_JZ: u8 = 0x0C;
pub const OP_CALL: u8 = 0x0D;
pub const OP_RET: u8 = 0x0E;
pub const OP_LOAD8: u8 = 0x0F;
pub const OP_STORE8: u8 = 0x10;
pub const OP_SYSCALL: u8 = 0x11;
pub const OP_HALT: u8 = 0x12;

// Syscall id - Standard (0-23)
pub const SYS_WRITE_BYTE: u8 = 0;
pub const SYS_WRITE_STR: u8 = 1;
pub const SYS_READ_KEY: u8 = 2;
pub const SYS_GET_TICKS: u8 = 3;
pub const SYS_EXIT: u8 = 4;
pub const SYS_READ_CHAR: u8 = 5;
pub const SYS_GET_PID: u8 = 6;
pub const SYS_SLEEP_MS: u8 = 7;
pub const SYS_RANDOM: u8 = 8;
pub const SYS_CLEAR_SCREEN: u8 = 9;
pub const SYS_GET_SCREEN_W: u8 = 10;
pub const SYS_GET_SCREEN_H: u8 = 11;
pub const SYS_GET_MOUSE_X: u8 = 12;
pub const SYS_GET_MOUSE_Y: u8 = 13;
pub const SYS_GET_MOUSE_BTN: u8 = 14;
pub const SYS_BEEP: u8 = 15;
pub const SYS_SET_GFX_MODE: u8 = 16;
pub const SYS_SET_TTY_MODE: u8 = 17;
pub const SYS_PUT_PIXEL: u8 = 18;
pub const SYS_DRAW_RECT: u8 = 19;
pub const SYS_DRAW_STR_GFX: u8 = 20;
pub const SYS_DRAW_CURSOR: u8 = 21;
pub const SYS_MOUSE_CLICKED: u8 = 22;
pub const SYS_MOUSE_RIGHT_CLICKED: u8 = 23;

// Syscall id - Driver (30-42)
pub const SYS_DRIVER_REGISTER: u8 = 30;
pub const SYS_DRIVER_READY: u8 = 31;
pub const SYS_DRIVER_IO_PORT: u8 = 32;
pub const SYS_DRIVER_CLAIM_IRQ: u8 = 33;
pub const SYS_DRIVER_RELEASE_IRQ: u8 = 34;
pub const SYS_DRIVER_INFO: u8 = 35;
pub const SYS_DRIVER_UNREGISTER: u8 = 36;
pub const SYS_DRIVER_SEND_EVENT: u8 = 37;
pub const SYS_DRIVER_WAIT_EVENT: u8 = 38;
pub const SYS_DRIVER_POLL_EVENT: u8 = 39;
pub const SYS_DRIVER_GET_STATE: u8 = 40;
pub const SYS_DRIVER_GET_COUNT: u8 = 41;
pub const SYS_DBGSERIAL: u8 = 42;

pub struct Vm<'a> {
    code: &'a [u8],
    pc: usize,
    stack: [i32; MAX_STACK],
    sp: usize,
    call_stack: [usize; MAX_CALLSTACK],
    csp: usize,
    memory: [u8; MAX_MEM],
    pub halted: bool,
    pub pid: u32,
    pub sleep_until: u64,
}

impl<'a> Vm<'a> {
    pub fn new(code: &'a [u8]) -> Self {
        Vm {
            code,
            pc: 0,
            stack: [0; MAX_STACK],
            sp: 0,
            call_stack: [0; MAX_CALLSTACK],
            csp: 0,
            memory: [0; MAX_MEM],
            halted: false,
            pid: 0,
            sleep_until: 0,
        }
    }

    pub fn pc(&self) -> usize {
        self.pc
    }

    pub fn rewind_pc(&mut self, bytes: usize) -> Result<(), Trap> {
        if self.pc >= bytes {
            self.pc -= bytes;
            Ok(())
        } else {
            Err(Trap::CodeOutOfBounds)
        }
    }

    pub fn set_pc(&mut self, new_pc: usize) -> Result<(), Trap> {
        if new_pc >= self.code.len() {
            return Err(Trap::CodeOutOfBounds);
        }
        self.pc = new_pc;
        Ok(())
    }

    pub fn set_pid(&mut self, pid: u32) {
        self.pid = pid;
    }

    pub fn push(&mut self, v: i32) -> Result<(), Trap> {
        if self.sp >= MAX_STACK {
            return Err(Trap::StackOverflow);
        }
        self.stack[self.sp] = v;
        self.sp += 1;
        Ok(())
    }

    pub fn pop(&mut self) -> Result<i32, Trap> {
        if self.sp == 0 {
            return Err(Trap::StackUnderflow);
        }
        self.sp -= 1;
        Ok(self.stack[self.sp])
    }

    pub fn memory(&self) -> &[u8; MAX_MEM] {
        &self.memory
    }

    pub fn memory_mut(&mut self) -> &mut [u8; MAX_MEM] {
        &mut self.memory
    }

    fn fetch_u8(&mut self) -> Result<u8, Trap> {
        if self.pc >= self.code.len() {
            self.halted = true;
            return Err(Trap::Halted);
        }
        let b = self.code[self.pc];
        self.pc += 1;
        Ok(b)
    }

    fn fetch_i32(&mut self) -> Result<i32, Trap> {
        if self.pc + 4 > self.code.len() {
            return Err(Trap::CodeOutOfBounds);
        }
        let b = [
            self.code[self.pc],
            self.code[self.pc + 1],
            self.code[self.pc + 2],
            self.code[self.pc + 3],
        ];
        self.pc += 4;
        Ok(i32::from_le_bytes(b))
    }

    fn mem_read(&self, addr: i32) -> Result<u8, Trap> {
        if addr < 0 || addr as usize >= MAX_MEM {
            return Err(Trap::MemOutOfBounds);
        }
        Ok(self.memory[addr as usize])
    }

    fn mem_write(&mut self, addr: i32, val: u8) -> Result<(), Trap> {
        if addr < 0 || addr as usize >= MAX_MEM {
            return Err(Trap::MemOutOfBounds);
        }
        self.memory[addr as usize] = val;
        Ok(())
    }

    pub fn load_initial_memory(&mut self, data: &[u8]) -> bool {
        if data.len() > MAX_MEM {
            return false;
        }
        self.memory[..data.len()].copy_from_slice(data);
        true
    }

    pub fn run(&mut self, max_steps: u32) -> Result<u32, Trap> {
        let current_ticks = crate::timer::get_ticks();

        if current_ticks < self.sleep_until {
            return Ok(0);
        } else if self.sleep_until > 0 {
            unsafe {
                let tmp = crate::cpu::inb(0x61);
                crate::cpu::io_wait();
                crate::cpu::outb(0x61, tmp & !0x03);
            }
            self.sleep_until = 0;
        }

        let mut steps: u32 = 0;
        while !self.halted && steps < max_steps {
            if self.pc >= self.code.len() {
                self.halted = true;
                break;
            }
            steps += 1;
            self.step()?;

            let now = crate::timer::get_ticks();
            if self.sleep_until > now {
                return Ok(steps);
            }
        }

        if self.sleep_until > 0 {
            let now = crate::timer::get_ticks();
            if now < self.sleep_until {
                return Ok(steps);
            }
            self.sleep_until = 0;
            unsafe {
                let tmp = crate::cpu::inb(0x61);
                crate::cpu::io_wait();
                crate::cpu::outb(0x61, tmp & !0x03);
            }
        }

        Ok(steps)
    }

    fn step(&mut self) -> Result<(), Trap> {
        let op = self.fetch_u8()?;
        match op {
            OP_NOP => {}
            OP_PUSH_I32 => {
                let v = self.fetch_i32()?;
                self.push(v)?;
            }
            OP_POP => {
                self.pop()?;
            }
            OP_DUP => {
                let v = self.pop()?;
                self.push(v)?;
                self.push(v)?;
            }
            OP_ADD => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(a.wrapping_add(b))?;
            }
            OP_SUB => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(a.wrapping_sub(b))?;
            }
            OP_MUL => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(a.wrapping_mul(b))?;
            }
            OP_DIV => {
                let b = self.pop()?;
                let a = self.pop()?;
                if b == 0 {
                    return Err(Trap::DivByZero);
                }
                self.push(a.wrapping_div(b))?;
            }
            OP_EQ => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(if a == b { 1 } else { 0 })?;
            }
            OP_LT => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(if a < b { 1 } else { 0 })?;
            }
            OP_GT => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(if a > b { 1 } else { 0 })?;
            }
            OP_JMP => {
                let target = self.fetch_i32()?;
                if target < 0 || target as usize > self.code.len() {
                    return Err(Trap::CodeOutOfBounds);
                }
                self.pc = target as usize;
            }
            OP_JZ => {
                let target = self.fetch_i32()?;
                let cond = self.pop()?;
                if cond == 0 {
                    if target < 0 || target as usize > self.code.len() {
                        return Err(Trap::CodeOutOfBounds);
                    }
                    self.pc = target as usize;
                }
            }
            OP_CALL => {
                let target = self.fetch_i32()?;
                if self.csp >= MAX_CALLSTACK {
                    return Err(Trap::CallStackOverflow);
                }
                if target < 0 || target as usize > self.code.len() {
                    return Err(Trap::CodeOutOfBounds);
                }
                self.call_stack[self.csp] = self.pc;
                self.csp += 1;
                self.pc = target as usize;
            }
            OP_RET => {
                if self.csp == 0 {
                    self.halted = true;
                } else {
                    self.csp -= 1;
                    self.pc = self.call_stack[self.csp];
                }
            }
            OP_LOAD8 => {
                let addr = self.pop()?;
                let v = self.mem_read(addr)?;
                self.push(v as i32)?;
            }
            OP_STORE8 => {
                let addr = self.pop()?;
                let val = self.pop()?; // FIXED: Lấy đúng val ra khỏi stack
                self.mem_write(addr, val as u8)?;
            }
            OP_SYSCALL => {
                let id = self.fetch_u8()?;
                self.do_syscall(id)?;
            }
            OP_HALT => {
                self.halted = true;
            }
            other => {
                return Err(Trap::UnknownOpcode(other));
            }
        }
        Ok(())
    }

    fn do_syscall(&mut self, id: u8) -> Result<(), Trap> {
        match id {
            SYS_WRITE_BYTE => {
                let v = self.pop()?;
                crate::serial::serial_write_byte(v as u8);
            }
            SYS_WRITE_STR | SYS_DBGSERIAL => {
                let len = self.pop()?;
                let addr = self.pop()?;
                if len < 0 || addr < 0 {
                    return Err(Trap::MemOutOfBounds);
                }
                let (addr, len) = (addr as usize, len as usize);
                if addr + len > MAX_MEM {
                    return Err(Trap::MemOutOfBounds);
                }

                let bytes = &self.memory[addr..addr + len];

                let is_driver = unsafe { crate::driver::DRIVER_MANAGER.is_driver(self.pid) };
                if is_driver {
                    crate::serial::serial_write("[DRIVER] ");
                } else {
                    crate::serial::serial_write("[APP] ");
                }

                if let Ok(s) = core::str::from_utf8(bytes) {
                    crate::serial::serial_write(s);
                } else {
                    for &b in bytes {
                        crate::serial::serial_write_byte(b);
                    }
                }
            }
            SYS_READ_KEY => {
                let scancode = crate::keyboard::poll_scancode();
                if let Some(code) = scancode {
                    self.push(code as i32)?;
                } else {
                    self.push(0)?;
                }
            }
            SYS_GET_TICKS => {
                let t = crate::timer::get_ticks();
                self.push((t & 0xFFFF_FFFF) as u32 as i32)?;
            }
            SYS_EXIT => {
                self.halted = true;
            }
            SYS_READ_CHAR => {
                loop {
                    let scancode = crate::keyboard::poll_scancode();
                    if let Some(code) = scancode {
                        if let Some(ch) = crate::keyboard::scancode_to_ascii(code) {
                            self.push(ch as u8 as i32)?;
                            break;
                        }
                    }
                }
            }
            SYS_GET_PID => {
                self.push(self.pid as i32)?;
            }
            SYS_SLEEP_MS => {
                let ms = self.pop()?;
                if ms > 0 {
                    let current_ticks = crate::timer::get_ticks();
                    self.sleep_until = current_ticks + (ms as u64);
                }
            }
            SYS_RANDOM => {
                let ticks = crate::timer::get_ticks();
                let seed = (ticks ^ (ticks >> 13) ^ (ticks << 7)) as u32;
                let rng = (seed.wrapping_mul(1103515245).wrapping_add(12345)) & 0x7FFFFFFF;
                self.push(rng as i32)?;
            }
            SYS_CLEAR_SCREEN => {
                unsafe {
                    crate::console::CONSOLE.clear();
                }
            }
            SYS_GET_SCREEN_W => {
                unsafe {
                    if !crate::console::CONSOLE.fb.is_null() {
                        let fb = &*crate::console::CONSOLE.fb;
                        self.push(fb.framebuffer_width as i32)?;
                    } else {
                        self.push(800)?;
                    }
                }
            }
            SYS_GET_SCREEN_H => {
                unsafe {
                    if !crate::console::CONSOLE.fb.is_null() {
                        let fb = &*crate::console::CONSOLE.fb;
                        self.push(fb.framebuffer_height as i32)?;
                    } else {
                        self.push(600)?;
                    }
                }
            }
            SYS_GET_MOUSE_X => {
                let (x, _, _, _) = crate::mouse::get_mouse_state();
                self.push(x)?;
            }
            SYS_GET_MOUSE_Y => {
                let (_, y, _, _) = crate::mouse::get_mouse_state();
                self.push(y)?;
            }
            SYS_GET_MOUSE_BTN => {
                let (_, _, left, right) = crate::mouse::get_mouse_state();
                let btn = (if left { 1 } else { 0 }) | (if right { 2 } else { 0 });
                self.push(btn)?;
            }
            SYS_BEEP => {
                let freq = self.pop()?;
                let duration = self.pop()?;
                if freq > 0 && duration > 0 {
                    unsafe {
                        let divisor = (1193182 / freq) as u16;
                        if divisor > 0 {
                            crate::cpu::outb(0x43, 0xB6);
                            crate::cpu::io_wait();
                            crate::cpu::outb(0x42, (divisor & 0xFF) as u8);
                            crate::cpu::io_wait();
                            crate::cpu::outb(0x42, (divisor >> 8) as u8);
                            crate::cpu::io_wait();

                            let tmp = crate::cpu::inb(0x61);
                            crate::cpu::io_wait();
                            crate::cpu::outb(0x61, tmp | 0x03);
                        }
                    }
                    let current_ticks = crate::timer::get_ticks();
                    self.sleep_until = current_ticks + (duration as u64);
                }
            }
            SYS_SET_GFX_MODE => {
                crate::console::set_gfx_mode(true);
                unsafe {
                    if !crate::console::CONSOLE.fb.is_null() {
                        let fb = &*crate::console::CONSOLE.fb;
                        crate::vbe::clear_screen(fb, 0x00000000);
                        crate::console::CONSOLE.draw_cursor(false);
                    }
                }
            }
            SYS_SET_TTY_MODE => {
                crate::console::set_gfx_mode(false);
                unsafe {
                    crate::console::CONSOLE.clear();
                    crate::print!("> ");
                }
            }
            SYS_PUT_PIXEL => {
                let color = self.pop()? as u32;
                let y = self.pop()?;
                let x = self.pop()?;
                unsafe {
                    if !crate::console::CONSOLE.fb.is_null() {
                        let fb = &*crate::console::CONSOLE.fb;
                        crate::vbe::put_pixel(fb, x as u32, y as u32, color);
                    }
                }
            }
            SYS_DRAW_RECT => {
                let color = self.pop()? as u32;
                let h = self.pop()?;
                let w = self.pop()?;
                let y = self.pop()?;
                let x = self.pop()?;

                if w > 0 && h > 0 {
                    crate::desktop::draw_rect(x, y, w as u32, h as u32, color);
                    crate::desktop::redraw();
                }
            }
            SYS_DRAW_STR_GFX => {
                let len = self.pop()?;
                let addr = self.pop()?;
                let color = self.pop()? as u32;
                let y = self.pop()?;
                let x = self.pop()?;
                if len > 0 && addr >= 0 {
                    let (addr, len) = (addr as usize, len as usize);
                    if addr + len <= MAX_MEM {
                        let bytes = &self.memory[addr..addr + len];
                        if let Ok(s) = core::str::from_utf8(bytes) {
                            crate::desktop::draw_string(x, y, s, color);
                        }
                    }
                }
            }
            SYS_DRAW_CURSOR => {
                let (mx, my, _, _) = crate::mouse::get_mouse_state();
                const CURSOR: [&str; 12] = [
                    "X           ",
                    "XX          ",
                    "X.X         ",
                    "X..X        ",
                    "X...X       ",
                    "X....X      ",
                    "X.....X     ",
                    "X......X    ",
                    "X.......X   ",
                    "X........X  ",
                    "X.....XXXX  ",
                    "XX   X..X   ",
                ];
                unsafe {
                    if !crate::console::CONSOLE.fb.is_null() {
                        let fb = &*crate::console::CONSOLE.fb;
                        for (dy, row) in CURSOR.iter().enumerate() {
                            for (dx, ch) in row.chars().enumerate() {
                                let px = mx + dx as i32;
                                let py = my + dy as i32;
                                if px < 0 || py < 0 { continue; }
                                match ch {
                                    'X' => crate::vbe::put_pixel(fb, px as u32, py as u32, 0x000000),
                                    '.' => crate::vbe::put_pixel(fb, px as u32, py as u32, 0xFFFFFF),
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
            SYS_MOUSE_CLICKED => {
                let clicked = crate::mouse::take_left_click();
                self.push(if clicked { 1 } else { 0 })?;
            }
            SYS_MOUSE_RIGHT_CLICKED => {
                let clicked = crate::mouse::take_right_click();
                self.push(if clicked { 1 } else { 0 })?;
            }

            // Driver syscalls
            SYS_DRIVER_REGISTER => {
                crate::driver::driver_syscalls::sys_driver_register(self)?;
            }
            SYS_DRIVER_READY => {
                crate::driver::driver_syscalls::sys_driver_ready(self)?;
            }
            SYS_DRIVER_IO_PORT => {
                crate::driver::driver_syscalls::sys_driver_io_port(self)?;
            }
            SYS_DRIVER_CLAIM_IRQ => {
                crate::driver::driver_syscalls::sys_driver_claim_irq(self)?;
            }
            SYS_DRIVER_RELEASE_IRQ => {
                crate::driver::driver_syscalls::sys_driver_release_irq(self)?;
            }
            SYS_DRIVER_INFO => {
                crate::driver::driver_syscalls::sys_driver_info(self)?;
            }
            SYS_DRIVER_UNREGISTER => {
                crate::driver::driver_syscalls::sys_driver_unregister(self)?;
            }
            SYS_DRIVER_SEND_EVENT => {
                crate::driver::driver_syscalls::sys_driver_send_event(self)?;
            }
            SYS_DRIVER_WAIT_EVENT => {
                crate::driver::driver_syscalls::sys_driver_wait_event(self)?;
            }
            SYS_DRIVER_POLL_EVENT => {
                crate::driver::driver_syscalls::sys_driver_poll_event(self)?;
            }
            SYS_DRIVER_GET_STATE => {
                crate::driver::driver_syscalls::sys_driver_get_state(self)?;
            }
            SYS_DRIVER_GET_COUNT => {
                crate::driver::driver_syscalls::sys_driver_get_count(self)?;
            }

            other => {
                return Err(Trap::UnknownSyscall(other));
            }
        }
        Ok(())
    }
}

pub fn trap_name(t: Trap) -> &'static str {
    match t {
        Trap::StackOverflow => "stack overflow",
        Trap::StackUnderflow => "stack underflow",
        Trap::CallStackOverflow => "call stack overflow",
        Trap::CallStackUnderflow => "call stack underflow",
        Trap::MemOutOfBounds => "memory access out of bounds",
        Trap::CodeOutOfBounds => "code access out of bounds",
        Trap::DivByZero => "division by zero",
        Trap::UnknownOpcode(_) => "unknown opcode",
        Trap::UnknownSyscall(_) => "unknown syscall",
        Trap::Halted => "halted",
    }
}