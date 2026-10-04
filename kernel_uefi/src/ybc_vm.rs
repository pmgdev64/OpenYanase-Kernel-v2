// src/ybc_vm.rs
use alloc::vec::Vec;

const OP_NOP: u8 = 0;
const OP_PUSHINT: u8 = 1;
const OP_PUSHSTR: u8 = 2;
const OP_POP: u8 = 3;
const OP_ADD: u8 = 4;
const OP_SUB: u8 = 5;
const OP_MUL: u8 = 6;
const OP_DIV: u8 = 7;
const OP_LT: u8 = 8;
const OP_GT: u8 = 9;
const OP_EQ: u8 = 10;
const OP_NOT: u8 = 11;
const OP_JMPIFFALSE: u8 = 12;
const OP_JMP: u8 = 13;
const OP_CALLSYS: u8 = 14;
const OP_LOADLOCAL: u8 = 15;
const OP_STORELOCAL: u8 = 16;
const OP_DUP: u8 = 17;
const OP_HALT: u8 = 18;

const OP_NEWOBJECT: u8 = 19;
const OP_GETFIELD: u8 = 20;
const OP_SETFIELD: u8 = 21;
const OP_CALLMETHOD: u8 = 22;
const OP_RET: u8 = 23;

const OP_NEWARRAY: u8 = 24;
const OP_GETINDEX: u8 = 25;
const OP_SETINDEX: u8 = 26;
const OP_ARRAYLEN: u8 = 27;

const YBC_MAGIC: u32 = 0x59424331;

const MAX_STACK: usize = 512;
const MAX_LOCALS: usize = 32;
const MAX_CALL_DEPTH: usize = 16;
const MAX_OBJECTS: usize = 32;
const MAX_ARRAY_ELEMS: usize = 32;
const MAX_FIELDS: usize = 16;
const MAX_STRING_LEN: usize = 256;
const STRING_POOL_SLOTS: usize = 1024;
const STR_CACHE_SIZE: usize = 512;

/// Syscall 58 — system.exec_arr(array, len). VM đọc các phần tử của mảng,
/// marshalling thành C string null-terminated, rồi forward tới kernel
/// syscall 52 (system.exec cstr).
const SYS_EXEC_ARR: u64 = 58;
const SYS_EXEC_CSTR: u64 = 52;
const EXEC_SCRATCH_LEN: usize = 128;
const EXEC_MAX_NAME: usize = 120;

static mut STRING_POOL: [[u8; MAX_STRING_LEN]; STRING_POOL_SLOTS] =
    [[0; MAX_STRING_LEN]; STRING_POOL_SLOTS];
static mut STRING_POOL_NEXT: usize = 0;

static mut STR_CACHE_PC: [u32; STR_CACHE_SIZE] = [u32::MAX; STR_CACHE_SIZE];
static mut STR_CACHE_SLOT: [u16; STR_CACHE_SIZE] = [0; STR_CACHE_SIZE];

#[derive(Clone, Copy)]
pub enum GcObject {
    Instance { class_id: u16, fields: [i64; MAX_FIELDS] },
    Array { data: [i64; MAX_ARRAY_ELEMS], len: usize },
    Buffer { data: [u8; MAX_ARRAY_ELEMS], len: usize },
}

#[derive(Clone, Copy)]
pub struct CallFrame {
    pub return_pc: usize,
    pub locals: [i64; MAX_LOCALS],
    pub caller_sp: usize,
}

impl CallFrame {
    pub const fn empty() -> Self {
        Self {
            return_pc: 0,
            locals: [0; MAX_LOCALS],
            caller_sp: 0,
        }
    }
}

pub struct VM {
    pub code: Vec<u8>,
    pub strings: Vec<u8>,
    pub pc: usize,

    pub stack: [i64; MAX_STACK],
    pub sp: usize,

    pub call_stack: [CallFrame; MAX_CALL_DEPTH],
    pub fp: usize,

    pub locals: [i64; MAX_LOCALS],

    pub objects: [Option<GcObject>; MAX_OBJECTS],
    pub object_count: usize,

    exec_scratch: [u8; EXEC_SCRATCH_LEN],
}

pub type YbcVm = VM;

impl VM {
    pub fn new(bytecode: &[u8]) -> Result<Self, &'static str> {
        if bytecode.len() < 22 {
            return Err("Bytecode too short");
        }

        let magic = u32::from_le_bytes(bytecode[0..4].try_into().unwrap());
        if magic != YBC_MAGIC {
            return Err("Invalid YBC magic header");
        }

        let _ver = u16::from_le_bytes(bytecode[4..6].try_into().unwrap());
        let _num_locals = bytecode[6] as usize;
        let code_len = u32::from_le_bytes(bytecode[8..12].try_into().unwrap()) as usize;
        let str_len = u32::from_le_bytes(bytecode[12..16].try_into().unwrap()) as usize;
        let entry_pc = u32::from_le_bytes(bytecode[18..22].try_into().unwrap()) as usize;

        let code_start = 22;
        let code_end = code_start + code_len;
        let str_end = code_end + str_len;

        if bytecode.len() < str_end {
            return Err("Truncated bytecode data");
        }

        Ok(Self {
            code: bytecode[code_start..code_end].to_vec(),
            strings: bytecode[code_end..str_end].to_vec(),
            pc: entry_pc,
            stack: [0; MAX_STACK],
            sp: 0,
            call_stack: [CallFrame::empty(); MAX_CALL_DEPTH],
            fp: 0,
            locals: [0; MAX_LOCALS],
            objects: [None; MAX_OBJECTS],
            object_count: 0,
            exec_scratch: [0; EXEC_SCRATCH_LEN],
        })
    }

    #[inline(always)]
    fn push(&mut self, v: i64) -> Result<(), &'static str> {
        if self.sp >= MAX_STACK {
            return Err("Stack overflow");
        }
        self.stack[self.sp] = v;
        self.sp += 1;
        Ok(())
    }

    #[inline(always)]
    fn pop(&mut self) -> Result<i64, &'static str> {
        if self.sp == 0 {
            return Err("Stack underflow");
        }
        self.sp -= 1;
        Ok(self.stack[self.sp])
    }

    fn alloc_object(&mut self, obj: GcObject) -> Result<i64, &'static str> {
        for i in 0..MAX_OBJECTS {
            if self.objects[i].is_none() {
                self.objects[i] = Some(obj);
                self.object_count += 1;
                return Ok((i + 1) as i64);
            }
        }
        Err("Object pool exhausted")
    }

    #[inline(always)]
    fn obj(&self, handle: i64) -> Result<&GcObject, &'static str> {
        if handle <= 0 || (handle as usize) > MAX_OBJECTS {
            return Err("Invalid GC handle");
        }
        self.objects[(handle - 1) as usize]
            .as_ref()
            .ok_or("Object slot empty")
    }

    #[inline(always)]
    fn obj_mut(&mut self, handle: i64) -> Result<&mut GcObject, &'static str> {
        if handle <= 0 || (handle as usize) > MAX_OBJECTS {
            return Err("Invalid GC handle");
        }
        self.objects[(handle - 1) as usize]
            .as_mut()
            .ok_or("Object slot empty")
    }

    #[inline(always)]
    fn read_u8(&mut self) -> u8 {
        let b = self.code[self.pc];
        self.pc += 1;
        b
    }

    #[inline(always)]
    fn read_u16(&mut self) -> u16 {
        let v = u16::from_le_bytes(self.code[self.pc..self.pc + 2].try_into().unwrap());
        self.pc += 2;
        v
    }

    #[inline(always)]
    fn read_u32(&mut self) -> u32 {
        let v = u32::from_le_bytes(self.code[self.pc..self.pc + 4].try_into().unwrap());
        self.pc += 4;
        v
    }

    #[inline(always)]
    fn read_i64(&mut self) -> i64 {
        let v = i64::from_le_bytes(self.code[self.pc..self.pc + 8].try_into().unwrap());
        self.pc += 8;
        v
    }

    fn execute_syscall(&mut self, sys_id: u64, a1: u64, a2: u64, a3: u64, a4: u64, a5: u64) -> u64 {
        crate::syscall::handle_syscall(sys_id, a1, a2, a3, a4, a5) as u64
    }

    /// Intercept syscall 58: marshal mảng các i64 (đóng vai trò char code)
    /// thành mảng byte null-terminated, rồi gọi kernel syscall 52 (exec cstr).
    fn sys_exec_arr(&mut self, arr_handle: i64, req_len: u64) -> i64 {
        let req_len = (req_len as usize).min(EXEC_MAX_NAME);

        // Đọc thông tin mảng an toàn. Không giữ borrow self.objects sau đây.
        let (arr_len, arr_ptr): (usize, *const i64) = if arr_handle <= 0
            || (arr_handle as usize) > MAX_OBJECTS
        {
            (0, core::ptr::null())
        } else {
            match self.objects[(arr_handle - 1) as usize].as_ref() {
                Some(GcObject::Array { data, len }) => (*len, data.as_ptr()),
                _ => (0, core::ptr::null()),
            }
        };

        if arr_ptr.is_null() || arr_len == 0 {
            return -1;
        }

        let max = arr_len
            .min(req_len)
            .min(EXEC_SCRATCH_LEN - 1);
        if max == 0 {
            return -1;
        }

        // Marshal i64 -> u8 vào scratch buffer.
        unsafe {
            for i in 0..max {
                self.exec_scratch[i] = *arr_ptr.add(i) as u8;
            }
        }
        self.exec_scratch[max] = 0;

        let ptr = self.exec_scratch.as_ptr() as u64;
        crate::syscall::handle_syscall(SYS_EXEC_CSTR, ptr, max as u64, 0, 0, 0)
    }

    pub fn run(&mut self) -> Result<i64, &'static str> {
        loop {
            if self.pc >= self.code.len() {
                return Ok(0);
            }

            let op_pc = self.pc;
            let op = self.read_u8();
            match op {
                OP_NOP => {}
                OP_PUSHINT => {
                    let v = self.read_i64();
                    self.push(v)?;
                }
                OP_PUSHSTR => {
                    let offset = self.read_u16() as usize;
                    if offset + 2 > self.strings.len() {
                        return Err("String offset OOB");
                    }
                    let len = u16::from_le_bytes(
                        self.strings[offset..offset + 2].try_into().unwrap(),
                    ) as usize;
                    if offset + 2 + len > self.strings.len() {
                        return Err("String length OOB");
                    }
                    let str_bytes = &self.strings[offset + 2..offset + 2 + len];

                    let cache_key = op_pc & (STR_CACHE_SIZE - 1);
                    let slot = unsafe {
                        if STR_CACHE_PC[cache_key] == op_pc as u32 {
                            STR_CACHE_SLOT[cache_key] as usize
                        } else if STRING_POOL_NEXT < STRING_POOL_SLOTS {
                            let s = STRING_POOL_NEXT;
                            STRING_POOL_NEXT += 1;
                            STR_CACHE_PC[cache_key] = op_pc as u32;
                            STR_CACHE_SLOT[cache_key] = s as u16;
                            s
                        } else {
                            0
                        }
                    };

                    unsafe {
                        let pool_base = core::ptr::addr_of_mut!(STRING_POOL) as *mut u8;
                        let dest = pool_base.add(slot * MAX_STRING_LEN);
                        let copy_len = len.min(MAX_STRING_LEN - 1);
                        core::ptr::copy_nonoverlapping(str_bytes.as_ptr(), dest, copy_len);
                        *dest.add(copy_len) = 0;
                        self.push(dest as i64)?;
                    }
                }
                OP_POP => { self.pop()?; }
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
                        return Err("Division by zero");
                    }
                    self.push(a / b)?;
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
                OP_EQ => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.push(if a == b { 1 } else { 0 })?;
                }
                OP_NOT => {
                    let a = self.pop()?;
                    self.push(if a == 0 { 1 } else { 0 })?;
                }
                OP_JMPIFFALSE => {
                    let target = self.read_u32() as usize;
                    let cond = self.pop()?;
                    if cond == 0 {
                        if target >= self.code.len() {
                            return Err("Jump target OOB");
                        }
                        self.pc = target;
                    }
                }
                OP_JMP => {
                    let target = self.read_u32() as usize;
                    if target >= self.code.len() {
                        return Err("Jump target OOB");
                    }
                    self.pc = target;
                }
                OP_LOADLOCAL => {
                    let slot = self.read_u8() as usize;
                    if slot >= MAX_LOCALS {
                        return Err("Local slot OOB");
                    }
                    let val = self.locals[slot];
                    self.push(val)?;
                }
                OP_STORELOCAL => {
                    let slot = self.read_u8() as usize;
                    if slot >= MAX_LOCALS {
                        return Err("Local slot OOB");
                    }
                    let val = self.pop()?;
                    self.locals[slot] = val;
                }
                OP_DUP => {
                    if self.sp == 0 {
                        return Err("Stack underflow on dup");
                    }
                    let val = self.stack[self.sp - 1];
                    self.push(val)?;
                }
                OP_CALLSYS => {
                    let sys_id = self.read_u16() as u64;
                    let argc = self.read_u8() as usize;
                    let mut args = [0u64; 5];

                    for i in (0..argc).rev() {
                        if i < 5 {
                            args[i] = self.pop()? as u64;
                        } else {
                            self.pop()?;
                        }
                    }

                    let res: i64 = if sys_id == SYS_EXEC_ARR {
                        self.sys_exec_arr(args[0] as i64, args[1])
                    } else {
                        self.execute_syscall(
                            sys_id, args[0], args[1], args[2], args[3], args[4],
                        ) as i64
                    };
                    self.push(res)?;
                }
                OP_NEWOBJECT => {
                    let class_id = self.read_u16();
                    let handle = self.alloc_object(GcObject::Instance {
                        class_id,
                        fields: [0; MAX_FIELDS],
                    })?;
                    self.push(handle)?;
                }
                OP_GETFIELD => {
                    let slot = self.read_u8() as usize;
                    if slot >= MAX_FIELDS {
                        return Err("Field slot OOB");
                    }
                    let handle = self.pop()?;

                    let val = match self.obj(handle)? {
                        GcObject::Instance { fields, .. } => fields[slot],
                        _ => return Err("Target is not an instance"),
                    };

                    self.push(val)?;
                }
                OP_SETFIELD => {
                    let slot = self.read_u8() as usize;
                    if slot >= MAX_FIELDS {
                        return Err("Field slot OOB");
                    }
                    let val = self.pop()?;
                    let handle = self.pop()?;

                    match self.obj_mut(handle)? {
                        GcObject::Instance { fields, .. } => {
                            fields[slot] = val;
                        }
                        _ => return Err("Target is not an instance"),
                    }
                }
                OP_CALLMETHOD => {
                    let target_pc = self.read_u32() as usize;
                    if target_pc >= self.code.len() {
                        return Err("Call target OOB");
                    }
                    let argc = self.read_u8() as usize;

                    if self.fp >= MAX_CALL_DEPTH {
                        return Err("Call stack overflow");
                    }

                    let saved_locals = self.locals;
                    let return_pc = self.pc;

                    self.locals = [0; MAX_LOCALS];

                    for i in (0..argc).rev() {
                        if i + 1 < MAX_LOCALS {
                            self.locals[i + 1] = self.pop()?;
                        } else {
                            self.pop()?;
                        }
                    }
                    let this_obj = self.pop()?;
                    self.locals[0] = this_obj;

                    let caller_sp = self.sp;

                    self.call_stack[self.fp] = CallFrame {
                        return_pc,
                        locals: saved_locals,
                        caller_sp,
                    };
                    self.fp += 1;

                    self.pc = target_pc;
                }
                OP_RET => {
                    let ret_val = if self.sp > 0 {
                        self.pop().unwrap_or(0)
                    } else {
                        0
                    };

                    if self.fp > 0 {
                        self.fp -= 1;
                        let frame = self.call_stack[self.fp];
                        self.pc = frame.return_pc;
                        self.locals = frame.locals;
                        self.sp = frame.caller_sp;
                        self.push(ret_val)?;
                    } else {
                        return Ok(ret_val);
                    }
                }
                OP_NEWARRAY => {
                    let size = self.pop()? as usize;
                    if size > MAX_ARRAY_ELEMS {
                        return Err("Array size exceeds limit");
                    }
                    let handle = self.alloc_object(GcObject::Array {
                        data: [0; MAX_ARRAY_ELEMS],
                        len: size,
                    })?;
                    self.push(handle)?;
                }
                OP_GETINDEX => {
                    let index = self.pop()? as usize;
                    let handle = self.pop()?;

                    let val = match self.obj(handle)? {
                        GcObject::Array { data, len } => {
                            if index >= *len {
                                return Err("Array index OOB");
                            }
                            data[index]
                        }
                        GcObject::Buffer { data, len } => {
                            if index >= *len {
                                return Err("Buffer index OOB");
                            }
                            data[index] as i64
                        }
                        _ => return Err("Not indexable"),
                    };

                    self.push(val)?;
                }
                OP_SETINDEX => {
                    let val = self.pop()?;
                    let index = self.pop()? as usize;
                    let handle = self.pop()?;

                    match self.obj_mut(handle)? {
                        GcObject::Array { data, len } => {
                            if index >= *len {
                                return Err("Array index OOB");
                            }
                            data[index] = val;
                        }
                        GcObject::Buffer { data, len } => {
                            if index >= *len {
                                return Err("Buffer index OOB");
                            }
                            data[index] = val as u8;
                        }
                        _ => return Err("Not mutable indexable"),
                    }
                }
                OP_ARRAYLEN => {
                    let handle = self.pop()?;
                    let len = match self.obj(handle)? {
                        GcObject::Array { len, .. } => *len as i64,
                        GcObject::Buffer { len, .. } => *len as i64,
                        _ => return Err("Not sized"),
                    };
                    self.push(len)?;
                }
                OP_HALT => {
                    let res = if self.sp > 0 { self.pop().unwrap_or(0) } else { 0 };
                    return Ok(res);
                }
                _ => return Err("Unknown opcode"),
            }
        }
    }

    pub fn attach_shm_buffer(&mut self, shm_id: u64, size: usize) -> Result<i64, &'static str> {
        let size = size.min(MAX_ARRAY_ELEMS);
        let handle = self.alloc_object(GcObject::Buffer {
            data: [0; MAX_ARRAY_ELEMS],
            len: size,
        })?;

        let ptr = match &mut self.objects[(handle - 1) as usize] {
            Some(GcObject::Buffer { data, .. }) => data.as_mut_ptr() as u64,
            _ => return Err("Failed to init buffer"),
        };

        self.execute_syscall(126, shm_id, ptr, size as u64, 0, 0);
        Ok(handle)
    }
}