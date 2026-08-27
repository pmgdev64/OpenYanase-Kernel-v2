// src/ybc.rs — no_std, dùng chung cho loader (Ring 0) và interpreter (Ring 3)
#![allow(dead_code)]

pub const YBC_MAGIC: u32 = 0x59424331; // "YBC1"

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Op {
    Nop = 0,
    PushInt = 1,       // i64 operand (8 bytes, little-endian)
    PushStr = 2,        // u16 index vào string pool
    Pop = 3,
    Add = 4,
    Sub = 5,
    Mul = 6,
    Div = 7,
    Lt = 8,
    Gt = 9,
    Eq = 10,
    Not = 11,
    JmpIfFalse = 12,    // u32 địa chỉ tuyệt đối trong code
    Jmp = 13,           // u32
    CallSys = 14,       // u16 syscall_id, u8 argc
    LoadLocal = 15,     // u8 slot
    StoreLocal = 16,    // u8 slot
    Dup = 17,
    Halt = 18,
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
            _ => return None,
        })
    }
}

/// Header nằm ở đầu file .ybc, 24 bytes, repr(C) để đọc trực tiếp từ bytes
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

/// Danh sách syscall id được phép gọi từ sandbox — cố định, không cho mở rộng động
#[repr(u16)]
#[derive(Clone, Copy, PartialEq)]
pub enum SysCallId {
    Print = 1,      // arg: str index (đã push lên stack dạng PushStr trước đó)
    DrawRect = 2,   // arg: x, y, w, h, color (5 int trên stack)
    GetTick = 3,    // không arg, trả timer tick về stack
    Sleep = 4,      // arg: ms
    Exit = 5,       // arg: exit code
}

impl SysCallId {
    pub fn from_u16(v: u16) -> Option<SysCallId> {
        use SysCallId::*;
        Some(match v {
            1 => Print, 2 => DrawRect, 3 => GetTick, 4 => Sleep, 5 => Exit,
            _ => return None,
        })
    }
}

/// Parse header từ raw bytes, validate magic + kích thước hợp lệ trước khi dùng
pub fn parse_header(data: &[u8]) -> Option<&YbcHeader> {
    if data.len() < YBC_HEADER_SIZE {
        return None;
    }
    let header = unsafe { &*(data.as_ptr() as *const YbcHeader) };
    if header.magic != YBC_MAGIC {
        return None;
    }
    Some(header)
}

/// Validate toàn bộ layout file trước khi load vào Ring 3 —
/// bắt buộc chạy ở Ring 0 (loader), KHÔNG được bỏ qua bước này
pub fn validate_ybc(data: &[u8]) -> Result<(), &'static str> {
    let header = parse_header(data).ok_or("invalid header/magic")?;

    let code_len = header.code_len as usize;
    let str_len = header.string_pool_len as usize;
    let entry = header.entry_offset as usize;

    let expected_min = YBC_HEADER_SIZE
        .checked_add(code_len).ok_or("overflow")?
        .checked_add(str_len).ok_or("overflow")?;

    if data.len() < expected_min {
        return Err("file truncated vs header sizes");
    }
    if entry >= code_len {
        return Err("entry_offset out of code bounds");
    }
    if header.max_stack == 0 || header.max_stack > 4096 {
        return Err("max_stack out of allowed range");
    }
    if header.num_locals > 64 {
        return Err("too many locals");
    }

    // Duyệt toàn bộ opcode 1 lượt để chắc chắn không có opcode lạ / operand tràn biên
    // trước khi đưa cho Ring 3 thực thi — chặn crash do bytecode hỏng/độc hại
    let code = &data[YBC_HEADER_SIZE..YBC_HEADER_SIZE + code_len];
    let mut pc = 0usize;
    while pc < code.len() {
        let op = Op::from_u8(code[pc]).ok_or("unknown opcode")?;
        pc += 1;
        let operand_len = match op {
            Op::PushInt => 8,
            Op::PushStr | Op::JmpIfFalse | Op::Jmp => {
                if op == Op::PushStr { 2 } else { 4 }
            }
            Op::CallSys => 3, // u16 + u8
            Op::LoadLocal | Op::StoreLocal => 1,
            _ => 0,
        };
        if pc + operand_len > code.len() {
            return Err("operand truncated at end of code");
        }
        // Validate jump target nằm trong code
        if op == Op::Jmp || op == Op::JmpIfFalse {
            let target = u32::from_le_bytes([
                code[pc], code[pc + 1], code[pc + 2], code[pc + 3]
            ]) as usize;
            if target >= code_len {
                return Err("jump target out of bounds");
            }
        }
        if op == Op::LoadLocal || op == Op::StoreLocal {
            if code[pc] >= header.num_locals {
                return Err("local slot out of bounds");
            }
        }
        pc += operand_len;
    }

    Ok(())
}