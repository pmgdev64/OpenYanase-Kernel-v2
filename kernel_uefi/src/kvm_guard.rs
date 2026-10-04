// src/kvm_guard.rs
// Zero Trust: mọi entity đều có 1 Capability Token do kernel cấp.
// Token không thể tự tạo, không thể tự nâng quyền, chỉ kernel mint/revoke.

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CapToken(u64);

#[derive(Clone, Copy)]
pub struct Capabilities {
    pub can_io_port: bool,
    pub can_irq_register: bool,
    pub can_draw: bool,
    pub can_print: bool,
    pub max_steps_per_slice: u32,
    pub allowed_port_ranges: [(u16, u16); 4],
    pub allowed_port_count: u8,
}

impl Capabilities {
    pub const NONE: Self = Self {
        can_io_port: false, can_irq_register: false,
        can_draw: false, can_print: false,
        max_steps_per_slice: 1000,
        allowed_port_ranges: [(0, 0); 4], allowed_port_count: 0,
    };

    pub const APP_DEFAULT: Self = Self {
        can_io_port: false, can_irq_register: false,
        can_draw: true, can_print: true,
        max_steps_per_slice: 5000,
        allowed_port_ranges: [(0, 0); 4], allowed_port_count: 0,
    };

    pub const DRIVER_PS2: Self = Self {
        can_io_port: true, can_irq_register: true,
        can_draw: false, can_print: true,
        max_steps_per_slice: 2000,
        allowed_port_ranges: [(0x60, 0x64), (0, 0), (0, 0), (0, 0)],
        allowed_port_count: 1,
    };

    pub const DRIVER_TIMER: Self = Self {
        can_io_port: true, can_irq_register: true,
        can_draw: false, can_print: true,
        max_steps_per_slice: 1000,
        allowed_port_ranges: [(0x40, 0x43), (0, 0), (0, 0), (0, 0)],
        allowed_port_count: 1,
    };

    pub const DRIVER_BLOCK: Self = Self {
        can_io_port: true, can_irq_register: true,
        can_draw: false, can_print: true,
        max_steps_per_slice: 3000,
        allowed_port_ranges: [(0x1F0, 0x1F7), (0x170, 0x177), (0, 0), (0, 0)],
        allowed_port_count: 2,
    };
}

const MAX_TOKENS: usize = 32;
static mut TOKEN_TABLE: [Option<Capabilities>; MAX_TOKENS] = [None; MAX_TOKENS];
static mut NEXT_TOKEN_ID: u64 = 1;

fn mint_token(caps: Capabilities) -> Option<CapToken> {
    unsafe {
        for i in 0..MAX_TOKENS {
            if TOKEN_TABLE[i].is_none() {
                TOKEN_TABLE[i] = Some(caps);
                let id = (i as u64) | (NEXT_TOKEN_ID << 8);
                NEXT_TOKEN_ID = NEXT_TOKEN_ID.wrapping_add(1);
                return Some(CapToken(id));
            }
        }
        None
    }
}

pub fn mint_app_token() -> Option<CapToken> {
    mint_token(Capabilities::APP_DEFAULT)
}

pub fn mint_driver_token(preset: Capabilities) -> Option<CapToken> {
    mint_token(preset)
}

pub fn revoke(tok: CapToken) {
    let idx = (tok.0 & 0xFF) as usize;
    unsafe {
        if idx < MAX_TOKENS {
            TOKEN_TABLE[idx] = None;
        }
    }
}

pub fn check(tok: CapToken) -> Option<Capabilities> {
    let idx = (tok.0 & 0xFF) as usize;
    unsafe {
        if idx >= MAX_TOKENS { return None; }
        TOKEN_TABLE[idx]
    }
}