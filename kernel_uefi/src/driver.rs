// src/driver.rs
// OpenYanase Kernel Driver System v2.0
//
// RING-1 ISOLATION MODEL:
//   Mỗi driver là 1 gói .drv (tar chứa .ybc) chạy trong KernelVM. VM giữ
//   state (pc/sp/stack/locals) xuyên tick → driver có thể là daemon thực sự,
//   không chỉ init-pass. Token sống suốt vòng đời driver; chỉ bị revoke khi
//   unload hoặc fault.
//
//   Watchdog: 3 trap liên tiếp → Faulted → cleanup → revoke.

use core::sync::atomic::{AtomicU32, Ordering};
use crate::kvm::KernelVm;
use crate::kvm_guard::CapToken;

pub const MAX_DRIVERS: usize = 32;
pub const MAX_DRIVER_NAME: usize = 32;
pub const MAX_DRIVER_EVENTS: usize = 64;
pub const MAX_DRIVER_IRQS: usize = 16;
pub const WATCHDOG_STRIKES: u32 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DriverType {
    Block = 0, Net = 1, Input = 2, Display = 3,
    Audio = 4, Hid = 5, Bus = 6, Char = 7, Unknown = 255,
}

impl DriverType {
    pub const fn as_str(&self) -> &'static str {
        match self {
            DriverType::Block => "BLOCK",
            DriverType::Net => "NET",
            DriverType::Input => "INPUT",
            DriverType::Display => "DISPLAY",
            DriverType::Audio => "AUDIO",
            DriverType::Hid => "HID",
            DriverType::Bus => "BUS",
            DriverType::Char => "CHAR",
            DriverType::Unknown => "UNKNOWN",
        }
    }

    pub const fn from_u8(v: u8) -> Self {
        match v {
            0 => DriverType::Block, 1 => DriverType::Net, 2 => DriverType::Input,
            3 => DriverType::Display, 4 => DriverType::Audio, 5 => DriverType::Hid,
            6 => DriverType::Bus, 7 => DriverType::Char, _ => DriverType::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DriverState {
    Unloaded = 0, Loaded = 1, Initializing = 2,
    Running = 3, Faulted = 4, Unloading = 5,
}

impl DriverState {
    pub const fn as_str(&self) -> &'static str {
        match self {
            DriverState::Unloaded => "UNLOADED",
            DriverState::Loaded => "LOADED",
            DriverState::Initializing => "INIT",
            DriverState::Running => "RUNNING",
            DriverState::Faulted => "FAULTED",
            DriverState::Unloading => "UNLOAD",
        }
    }

    pub const fn from_u8(v: u8) -> Self {
        match v {
            0 => DriverState::Unloaded, 1 => DriverState::Loaded,
            2 => DriverState::Initializing, 3 => DriverState::Running,
            4 => DriverState::Faulted, 5 => DriverState::Unloading,
            _ => DriverState::Unloaded,
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct DriverEvent {
    pub event_type: u32,
    pub data1: u32,
    pub data2: u32,
    pub data3: u32,
}

impl DriverEvent {
    pub const fn new(event_type: u32, d1: u32, d2: u32, d3: u32) -> Self {
        DriverEvent { event_type, data1: d1, data2: d2, data3: d3 }
    }
    pub const fn empty() -> Self {
        DriverEvent { event_type: 0, data1: 0, data2: 0, data3: 0 }
    }
}

pub const EVENT_KEY:    u32 = 1;
pub const EVENT_MOUSE:  u32 = 2;
pub const EVENT_IRQ:    u32 = 3;
pub const EVENT_TIMER:  u32 = 4;
pub const EVENT_IO:     u32 = 5;
pub const EVENT_DEVICE: u32 = 6;
pub const EVENT_POWER:  u32 = 7;

// ============================================================
// DriverInfo — persistent VM + telemetry
// ============================================================
pub struct DriverInfo {
    // Identity
    pub pid: u32,
    pub slot: usize,
    pub name: [u8; MAX_DRIVER_NAME],
    pub driver_type: u8,
    pub state: u8,
    pub priority: u8,

    // Event queue
    pub events: [DriverEvent; MAX_DRIVER_EVENTS],
    pub event_count: usize,
    pub event_read: usize,

    // IRQ
    pub claimed_irqs: [u8; MAX_DRIVER_IRQS],
    pub irq_count: usize,

    // Telemetry
    pub steps_total: u64,
    pub slice_count: u64,
    pub fault_count: u32,
    pub last_irq_tick: u64,
    pub watchdog_strikes: u32,

    // Runtime — token + VM state
    pub token: Option<CapToken>,
    pub vm: Option<KernelVm>,
}

impl DriverInfo {
    pub const fn new() -> Self {
        DriverInfo {
            pid: 0,
            slot: 0,
            name: [0; MAX_DRIVER_NAME],
            driver_type: 0,
            state: 0,
            priority: 0,
            events: [DriverEvent::empty(); MAX_DRIVER_EVENTS],
            event_count: 0,
            event_read: 0,
            claimed_irqs: [0; MAX_DRIVER_IRQS],
            irq_count: 0,
            steps_total: 0,
            slice_count: 0,
            fault_count: 0,
            last_irq_tick: 0,
            watchdog_strikes: 0,
            token: None,
            vm: None,
        }
    }

    pub fn name_str(&self) -> &str {
        let len = self.name.iter().position(|&b| b == 0).unwrap_or(MAX_DRIVER_NAME);
        core::str::from_utf8(&self.name[..len]).unwrap_or("invalid")
    }

    pub fn driver_type_enum(&self) -> DriverType { DriverType::from_u8(self.driver_type) }
    pub fn state_enum(&self) -> DriverState { DriverState::from_u8(self.state) }

    pub fn push_event(&mut self, event: DriverEvent) -> bool {
        if self.event_count < MAX_DRIVER_EVENTS {
            self.events[self.event_count] = event;
            self.event_count += 1;
            true
        } else {
            false
        }
    }

    pub fn pop_event(&mut self) -> Option<DriverEvent> {
        if self.event_read < self.event_count {
            let event = self.events[self.event_read];
            self.event_read += 1;
            Some(event)
        } else {
            if self.event_read == self.event_count {
                self.event_count = 0;
                self.event_read = 0;
            }
            None
        }
    }

    pub fn claim_irq(&mut self, irq: u8) -> bool {
        if self.irq_count >= MAX_DRIVER_IRQS { return false; }
        for i in 0..self.irq_count {
            if self.claimed_irqs[i] == irq { return true; }
        }
        self.claimed_irqs[self.irq_count] = irq;
        self.irq_count += 1;
        true
    }

    pub fn has_irq(&self, irq: u8) -> bool {
        for i in 0..self.irq_count {
            if self.claimed_irqs[i] == irq { return true; }
        }
        false
    }
}

// ============================================================
// DriverManager
// ============================================================
pub struct DriverManager {
    pub drivers: [Option<DriverInfo>; MAX_DRIVERS],
    count: usize,
    irq_routing: [i32; 16],
    next_pid: u32,
}

impl DriverManager {
    pub const fn new() -> Self {
        DriverManager {
            drivers: [const { None }; MAX_DRIVERS],
            count: 0,
            irq_routing: [-1; 16],
            next_pid: 1000,
        }
    }

    pub fn allocate_pid(&mut self) -> u32 {
        let p = self.next_pid;
        self.next_pid += 1;
        p
    }

    /// Register driver with persistent VM + capability token.
    /// Token KHÔNG bị revoke — sẽ revoke lúc unload hoặc fault.
    pub fn register(
        &mut self,
        name: &str,
        dtype: DriverType,
        priority: u8,
        token: CapToken,
        vm: KernelVm,
    ) -> Option<u32> {
        // Re-registration: name trùng → ghi đè VM/token (hot reload)
        for i in 0..MAX_DRIVERS {
            if let Some(info) = self.drivers[i].as_mut() {
                if info.name_str() == name {
                    if let Some(old) = info.token.take() {
                        crate::kvm_guard::revoke(old);
                    }
                    info.driver_type = dtype as u8;
                    info.state = DriverState::Running as u8;
                    info.priority = priority.min(10);
                    info.token = Some(token);
                    info.vm = Some(vm);
                    info.watchdog_strikes = 0;
                    return Some(info.pid);
                }
            }
        }

        // Find slot
        let slot = (0..MAX_DRIVERS).find(|&i| self.drivers[i].is_none())?;
        let pid = self.allocate_pid();

        let mut name_buf = [0u8; MAX_DRIVER_NAME];
        let bytes = name.as_bytes();
        let n = bytes.len().min(MAX_DRIVER_NAME - 1);
        name_buf[..n].copy_from_slice(&bytes[..n]);

        let mut info = DriverInfo::new();
        info.pid = pid;
        info.slot = slot;
        info.name = name_buf;
        info.driver_type = dtype as u8;
        info.state = DriverState::Running as u8;
        info.priority = priority.min(10);
        info.token = Some(token);
        info.vm = Some(vm);

        self.drivers[slot] = Some(info);
        self.count += 1;
        Some(pid)
    }

    pub fn unload_driver(&mut self, pid: u32) -> bool {
        for i in 0..MAX_DRIVERS {
            let matches = matches!(self.drivers[i].as_ref(), Some(info) if info.pid == pid);
            if matches {
                // Revoke token + clear IRQ routing
                if let Some(info) = self.drivers[i].take() {
                    if let Some(tok) = info.token {
                        crate::kvm_guard::revoke(tok);
                    }
                }
                for irq in 0..16 {
                    if self.irq_routing[irq] == i as i32 {
                        self.irq_routing[irq] = -1;
                    }
                }
                if self.count > 0 { self.count -= 1; }
                return true;
            }
        }
        false
    }

    pub fn send_event(&mut self, pid: u32, event: DriverEvent) -> bool {
        for i in 0..MAX_DRIVERS {
            if let Some(info) = self.drivers[i].as_mut() {
                if info.pid == pid {
                    return info.push_event(event);
                }
            }
        }
        false
    }

    pub fn claim_irq(&mut self, pid: u32, irq: u8) -> bool {
        if irq as usize >= 16 { return false; }
        if self.irq_routing[irq as usize] >= 0 {
            for i in 0..MAX_DRIVERS {
                if let Some(info) = &self.drivers[i] {
                    if info.pid == pid && info.has_irq(irq) { return true; }
                }
            }
            return false;
        }
        for i in 0..MAX_DRIVERS {
            if let Some(info) = self.drivers[i].as_mut() {
                if info.pid == pid && info.claim_irq(irq) {
                    self.irq_routing[irq as usize] = i as i32;
                    return true;
                }
            }
        }
        false
    }

    pub fn release_irq(&mut self, pid: u32, irq: u8) -> bool {
        if irq as usize >= 16 { return false; }
        for i in 0..MAX_DRIVERS {
            if let Some(info) = self.drivers[i].as_mut() {
                if info.pid == pid && info.has_irq(irq) {
                    self.irq_routing[irq as usize] = -1;
                    let mut nc = 0;
                    for j in 0..info.irq_count {
                        if info.claimed_irqs[j] != irq {
                            info.claimed_irqs[nc] = info.claimed_irqs[j];
                            nc += 1;
                        }
                    }
                    info.irq_count = nc;
                    return true;
                }
            }
        }
        false
    }

    pub fn get_irq_driver(&self, irq: u8) -> Option<u32> {
        if irq as usize >= 16 { return None; }
        let slot = self.irq_routing[irq as usize];
        if slot < 0 { return None; }
        self.drivers[slot as usize].as_ref().map(|info| info.pid)
    }

    pub fn get_driver(&self, pid: u32) -> Option<&DriverInfo> {
        for i in 0..MAX_DRIVERS {
            if let Some(info) = &self.drivers[i] {
                if info.pid == pid { return Some(info); }
            }
        }
        None
    }

    pub fn get_driver_mut(&mut self, pid: u32) -> Option<&mut DriverInfo> {
        for i in 0..MAX_DRIVERS {
            if let Some(info) = self.drivers[i].as_mut() {
                if info.pid == pid { return Some(info); }
            }
        }
        None
    }

    pub fn is_driver(&self, pid: u32) -> bool { self.get_driver(pid).is_some() }
    pub fn count(&self) -> usize { self.count }

    pub fn cleanup_faulted(&mut self) {
        let mut to_remove = [0u32; MAX_DRIVERS];
        let mut n = 0usize;
        for i in 0..MAX_DRIVERS {
            if let Some(info) = &self.drivers[i] {
                let st = info.state;
                if st == DriverState::Faulted as u8 || st == DriverState::Unloading as u8 {
                    if n < MAX_DRIVERS {
                        to_remove[n] = info.pid;
                        n += 1;
                    }
                }
            }
        }
        for k in 0..n {
            self.unload_driver(to_remove[k]);
        }
    }
}

pub static mut DRIVER_MANAGER: DriverManager = DriverManager::new();

// ============================================================
// IRQ pending bitmask — race-free giữa IRQ handler và main loop
// ============================================================
pub static IRQ_PENDING: AtomicU32 = AtomicU32::new(0);

/// Gọi từ IRQ handler (bất kỳ context). Chỉ set bit — không lock, không
/// cấp phát, không chạm DriverManager.
pub fn mark_irq_pending(irq: u8) {
    if irq < 32 {
        IRQ_PENDING.fetch_or(1u32 << irq, Ordering::Relaxed);
    }
}

/// Gọi từ main loop. Đọc + clear bitmask một lần, rồi dispatch từng IRQ.
pub fn process_pending_irqs() {
    let mask = IRQ_PENDING.swap(0, Ordering::AcqRel);
    if mask == 0 { return; }
    for irq in 0..32u8 {
        if mask & (1u32 << irq) != 0 {
            dispatch_irq(irq);
        }
    }
}

/// Push event IRQ tới driver đang giữ IRQ này.
pub fn dispatch_irq(irq: u8) {
    unsafe {
        let pid = match DRIVER_MANAGER.get_irq_driver(irq) {
            Some(p) => p,
            None => return,
        };
        let tick = crate::timer::get_ticks();
        if let Some(info) = DRIVER_MANAGER.get_driver_mut(pid) {
            let ev = DriverEvent::new(EVENT_IRQ, irq as u32, 0, 0);
            info.push_event(ev);
            info.last_irq_tick = tick;
        }
    }
}

// ============================================================
// Scheduler — tick_all()
// ============================================================
/// Chạy mỗi driver Running một slice. Gọi từ main loop định kỳ (10ms).
/// Watchdog: 3 trap liên tiếp → Faulted. cleanup_faulted() dọn cuối tick.
pub fn tick_all(default_budget: u32) {
    unsafe {
        for i in 0..MAX_DRIVERS {
            let info = match DRIVER_MANAGER.drivers[i].as_mut() {
                Some(x) => x,
                None => continue,
            };
            if info.state != DriverState::Running as u8 { continue; }

            let vm = match info.vm.as_mut() {
                Some(v) => v,
                None => continue,
            };

            match vm.run(default_budget) {
                Ok(true) => {
                    // Halt: driver tự shutdown
                    info.steps_total = info.steps_total.saturating_add(default_budget as u64);
                    info.state = DriverState::Unloading as u8;
                }
                Ok(false) => {
                    // Yield bình thường — hết budget
                    info.steps_total = info.steps_total.saturating_add(default_budget as u64);
                    info.slice_count = info.slice_count.saturating_add(1);
                    info.watchdog_strikes = 0;
                }
                Err(_) => {
                    info.fault_count += 1;
                    info.watchdog_strikes += 1;
                    if info.watchdog_strikes >= WATCHDOG_STRIKES {
                        info.state = DriverState::Faulted as u8;
                    }
                }
            }
        }
        DRIVER_MANAGER.cleanup_faulted();
    }
}

// ============================================================
// Helpers
// ============================================================

pub fn detect_driver_type(name: &str) -> DriverType {
    let mut buf = [0u8; 64];
    let bytes = name.as_bytes();
    let len = bytes.len().min(63);
    for i in 0..len {
        let b = bytes[i];
        buf[i] = if b >= b'A' && b <= b'Z' { b + 32 } else { b };
    }
    let lower = core::str::from_utf8(&buf[..len]).unwrap_or(name);

    if lower.contains("kbd") || lower.contains("keyboard") { DriverType::Input }
    else if lower.contains("mouse") || lower.contains("hid") { DriverType::Hid }
    else if lower.contains("fb") || lower.contains("vesa") || lower.contains("gpu") { DriverType::Display }
    else if lower.contains("audio") || lower.contains("sound") || lower.contains("beep") || lower.contains("spk") { DriverType::Audio }
    else if lower.contains("pci") || lower.contains("usb") { DriverType::Bus }
    else if lower.contains("net") || lower.contains("eth") { DriverType::Net }
    else if lower.contains("serial") || lower.contains("tty") { DriverType::Char }
    else if lower.contains("block") || lower.contains("disk") { DriverType::Block }
    else { DriverType::Unknown }
}

pub fn get_driver_info(pid: u32) -> Option<&'static DriverInfo> {
    unsafe { DRIVER_MANAGER.get_driver(pid) }
}

pub fn send_event_to_driver(pid: u32, et: u32, d1: u32, d2: u32, d3: u32) -> bool {
    unsafe { DRIVER_MANAGER.send_event(pid, DriverEvent::new(et, d1, d2, d3)) }
}

pub fn event_key(scancode: u8, ascii: u8, pressed: bool) -> DriverEvent {
    DriverEvent::new(EVENT_KEY, scancode as u32, ascii as u32, if pressed { 1 } else { 0 })
}

pub fn event_mouse(x: i32, y: i32, buttons: u8) -> DriverEvent {
    DriverEvent::new(EVENT_MOUSE, x as u32, y as u32, buttons as u32)
}

pub fn event_timer(ticks: u64) -> DriverEvent {
    DriverEvent::new(
        EVENT_TIMER,
        (ticks & 0xFFFF_FFFF) as u32,
        ((ticks >> 32) & 0xFFFF_FFFF) as u32,
        0,
    )
}

/// Print table with telemetry summary.
pub fn list_drivers() {
    crate::println!("SLOT | PID  | TYPE    | STATE   | PRI | IRQ | STEPS    | NAME");
    crate::println!("-----|------|---------|---------|-----|-----|----------|------------------");

    unsafe {
        let mut any = false;
        for i in 0..MAX_DRIVERS {
            if let Some(info) = DRIVER_MANAGER.drivers[i].as_ref() {
                any = true;
                let mut irq_buf = [0u8; 24];
                let mut ip = 0usize;
                for j in 0..info.irq_count {
                    if j > 0 && ip < 23 { irq_buf[ip] = b','; ip += 1; }
                    let d = info.claimed_irqs[j];
                    if d >= 10 && ip + 1 < 23 {
                        irq_buf[ip] = b'0' + d / 10; ip += 1;
                        irq_buf[ip] = b'0' + d % 10; ip += 1;
                    } else if ip < 23 {
                        irq_buf[ip] = b'0' + d; ip += 1;
                    }
                }
                let irq_str = if ip == 0 { "none" }
                    else { core::str::from_utf8(&irq_buf[..ip]).unwrap_or("?") };

                crate::println!(
                    "{:3}  | {:4} | {:7} | {:7} | {:3} | {:3} | {:8} | {}",
                    i, info.pid,
                    info.driver_type_enum().as_str(),
                    info.state_enum().as_str(),
                    info.priority,
                    irq_str,
                    info.steps_total,
                    info.name_str()
                );
            }
        }
        if !any { crate::println!("(no drivers registered)"); }
    }
}