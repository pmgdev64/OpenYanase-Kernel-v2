#![allow(dead_code)]

pub const YBC_MAGIC: u32 = 0x59424331;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Op {
    Nop = 0, PushInt = 1, PushStr = 2, Pop = 3,
    Add = 4, Sub = 5, Mul = 6, Div = 7,
    Lt = 8, Gt = 9, Eq = 10, Not = 11,
    JmpIfFalse = 12, Jmp = 13, CallSys = 14,
    LoadLocal = 15, StoreLocal = 16, Dup = 17, Halt = 18,
    NewObject = 19, GetField = 20, SetField = 21, CallMethod = 22, Ret = 23,
    NewArray = 24, GetIndex = 25, SetIndex = 26, ArrayLen = 27,
}

impl Op {
    pub fn from_u8(b: u8) -> Option<Op> {
        use Op::*;
        Some(match b {
            0 => Nop, 1 => PushInt, 2 => PushStr, 3 => Pop,
            4 => Add, 5 => Sub, 6 => Mul, 7 => Div,
            8 => Lt, 9 => Gt, 10 => Eq, 11 => Not,
            12 => JmpIfFalse, 13 => Jmp, 14 => CallSys,
            15 => LoadLocal, 16 => StoreLocal, 17 => Dup, 18 => Halt,
            19 => NewObject, 20 => GetField, 21 => SetField,
            22 => CallMethod, 23 => Ret,
            24 => NewArray, 25 => GetIndex, 26 => SetIndex, 27 => ArrayLen,
            _ => return None,
        })
    }

    fn operand_len(self) -> usize {
        match self {
            Op::PushInt => 8,
            Op::PushStr => 2,
            Op::JmpIfFalse | Op::Jmp => 4,
            Op::CallSys => 3,
            Op::LoadLocal | Op::StoreLocal => 1,
            Op::NewObject => 2,
            Op::GetField | Op::SetField => 1,
            Op::CallMethod => 5,
            _ => 0,
        }
    }
}

#[repr(C, packed)]
pub struct YbcHeader {
    pub magic: u32,
    pub version: u16,
    pub num_locals: u8,
    pub _pad: u8,
    pub code_len: u32,
    pub string_pool_len: u32,
    pub max_stack: u16,
    pub entry_offset: u32,
}

pub const YBC_HEADER_SIZE: usize = core::mem::size_of::<YbcHeader>();

pub fn parse_header(data: &[u8]) -> Option<&YbcHeader> {
    if data.len() < YBC_HEADER_SIZE { return None; }
    let header = unsafe { &*(data.as_ptr() as *const YbcHeader) };
    if header.magic != YBC_MAGIC { return None; }
    Some(header)
}

pub fn validate_ybc(data: &[u8]) -> Result<(), &'static str> {
    let header = parse_header(data).ok_or("invalid header/magic")?;

    let code_len = header.code_len as usize;
    let str_len = header.string_pool_len as usize;
    let entry = header.entry_offset as usize;

    let expected_min = YBC_HEADER_SIZE
        .checked_add(code_len).ok_or("overflow")?
        .checked_add(str_len).ok_or("overflow")?;

    if data.len() < expected_min { return Err("file truncated vs header sizes"); }
    if entry >= code_len { return Err("entry_offset out of code bounds"); }
    if header.max_stack == 0 || header.max_stack > 4096 { return Err("max_stack out of allowed range"); }

    let code = &data[YBC_HEADER_SIZE..YBC_HEADER_SIZE + code_len];
    let strings = &data[YBC_HEADER_SIZE + code_len..YBC_HEADER_SIZE + code_len + str_len];
    
    let mut pc = 0usize;
    while pc < code.len() {
        let op = Op::from_u8(code[pc]).ok_or("unknown opcode")?;
        pc += 1;
        let operand_len = op.operand_len();
        if pc + operand_len > code.len() {
            return Err("operand truncated at end of code");
        }

        match op {
            Op::Jmp | Op::JmpIfFalse | Op::CallMethod => {
                let target = u32::from_le_bytes([
                    code[pc], code[pc + 1], code[pc + 2], code[pc + 3]
                ]) as usize;
                if target >= code_len { return Err("jump/call target out of bounds"); }
            }
            Op::LoadLocal | Op::StoreLocal => {
                // REMOVED: bounds check
            }
            Op::GetField | Op::SetField => {
                if code[pc] >= 16 { return Err("field slot out of bounds (max 16)"); }
            }
            Op::PushStr => {
                let str_idx = u16::from_le_bytes([code[pc], code[pc + 1]]) as usize;
                if str_idx + 2 > str_len { return Err("string index out of pool bounds"); }
                let s_len = u16::from_le_bytes([strings[str_idx], strings[str_idx + 1]]) as usize;
                if str_idx + 2 + s_len > str_len { return Err("string length exceeds pool"); }
            }
            Op::CallSys => {
                // REMOVED: syscall ID validation
                // Only validate argc
                let argc = code[pc + 2];
                if argc > 5 { return Err("syscall argc too large"); }
            }
            _ => {}
        }
        pc += operand_len;
    }
    Ok(())
}