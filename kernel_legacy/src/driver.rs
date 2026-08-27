// src/driver.rs
// OpenYanase Kernel Driver System v2.0

pub const MAX_DRIVERS: usize = 32;
pub const MAX_DRIVER_NAME: usize = 32;
pub const MAX_DRIVER_EVENTS: usize = 64;
pub const MAX_DRIVER_IRQS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DriverType {
    Block = 0,
    Net = 1,
    Input = 2,
    Display = 3,
    Audio = 4,
    Hid = 5,
    Bus = 6,
    Char = 7,
    Unknown = 255,
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
            0 => DriverType::Block,
            1 => DriverType::Net,
            2 => DriverType::Input,
            3 => DriverType::Display,
            4 => DriverType::Audio,
            5 => DriverType::Hid,
            6 => DriverType::Bus,
            7 => DriverType::Char,
            _ => DriverType::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DriverState {
    Unloaded = 0,
    Loaded = 1,
    Initializing = 2,
    Running = 3,
    Faulted = 4,
    Unloading = 5,
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
            0 => DriverState::Unloaded,
            1 => DriverState::Loaded,
            2 => DriverState::Initializing,
            3 => DriverState::Running,
            4 => DriverState::Faulted,
            5 => DriverState::Unloading,
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
    pub const fn new(event_type: u32, data1: u32, data2: u32, data3: u32) -> Self {
        DriverEvent { event_type, data1, data2, data3 }
    }

    pub const fn empty() -> Self {
        DriverEvent { event_type: 0, data1: 0, data2: 0, data3: 0 }
    }
}

pub const EVENT_KEY: u32 = 1;
pub const EVENT_MOUSE: u32 = 2;
pub const EVENT_IRQ: u32 = 3;
pub const EVENT_TIMER: u32 = 4;
pub const EVENT_IO: u32 = 5;
pub const EVENT_DEVICE: u32 = 6;
pub const EVENT_POWER: u32 = 7;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct DriverInfo {
    pub pid: u32,
    pub driver_type: u8,
    pub state: u8,
    pub priority: u8,
    pub reserved: u8,
    pub name: [u8; MAX_DRIVER_NAME],
    pub events: [DriverEvent; MAX_DRIVER_EVENTS],
    pub event_count: usize,
    pub event_read: usize,
    pub claimed_irqs: [u8; MAX_DRIVER_IRQS],
    pub irq_count: usize,
    pub slot: usize,
}

impl DriverInfo {
    pub const fn new() -> Self {
        DriverInfo {
            pid: 0,
            driver_type: 0,
            state: 0,
            priority: 0,
            reserved: 0,
            name: [0; MAX_DRIVER_NAME],
            events: [DriverEvent::empty(); MAX_DRIVER_EVENTS],
            event_count: 0,
            event_read: 0,
            claimed_irqs: [0; MAX_DRIVER_IRQS],
            irq_count: 0,
            slot: 0,
        }
    }

    pub fn name_str(&self) -> &str {
        let len = self.name.iter().position(|&b| b == 0).unwrap_or(MAX_DRIVER_NAME);
        core::str::from_utf8(&self.name[..len]).unwrap_or("invalid")
    }

    pub fn driver_type_enum(&self) -> DriverType {
        DriverType::from_u8(self.driver_type)
    }

    pub fn state_enum(&self) -> DriverState {
        DriverState::from_u8(self.state)
    }

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
        if self.irq_count >= MAX_DRIVER_IRQS {
            return false;
        }
        for i in 0..self.irq_count {
            if self.claimed_irqs[i] == irq {
                return true;
            }
        }
        self.claimed_irqs[self.irq_count] = irq;
        self.irq_count += 1;
        true
    }

    pub fn has_irq(&self, irq: u8) -> bool {
        for i in 0..self.irq_count {
            if self.claimed_irqs[i] == irq {
                return true;
            }
        }
        false
    }
}

pub struct DriverManager {
    drivers: [Option<DriverInfo>; MAX_DRIVERS],
    driver_pid_to_slot: [i32; MAX_DRIVERS],
    count: usize,
    irq_routing: [i32; 16],
}

impl DriverManager {
    pub const fn new() -> Self {
        DriverManager {
            drivers: [const { None }; MAX_DRIVERS],
            driver_pid_to_slot: [-1; MAX_DRIVERS],
            count: 0,
            irq_routing: [-1; 16],
        }
    }

    pub fn register_driver(&mut self, pid: u32, name: &str, driver_type: DriverType, priority: u8) -> Option<usize> {
        for i in 0..MAX_DRIVERS {
            if let Some(info) = &mut self.drivers[i] {
                if info.pid == pid {
                    let mut name_buf = [0u8; MAX_DRIVER_NAME];
                    let name_bytes = name.as_bytes();
                    let len = name_bytes.len().min(MAX_DRIVER_NAME - 1);
                    name_buf[..len].copy_from_slice(&name_bytes[..len]);

                    info.driver_type = driver_type as u8;
                    info.state = DriverState::Loaded as u8;
                    info.priority = priority.min(10);
                    info.name = name_buf;
                    return Some(i);
                }
            }
        }

        let mut free_slot = None;
        for i in 0..MAX_DRIVERS {
            if self.drivers[i].is_none() {
                free_slot = Some(i);
                break;
            }
        }

        let slot = free_slot?;

        let mut name_buf = [0u8; MAX_DRIVER_NAME];
        let name_bytes = name.as_bytes();
        let len = name_bytes.len().min(MAX_DRIVER_NAME - 1);
        name_buf[..len].copy_from_slice(&name_bytes[..len]);

        let mut info = DriverInfo::new();
        info.pid = pid;
        info.driver_type = driver_type as u8;
        info.state = DriverState::Loaded as u8;
        info.priority = priority.min(10);
        info.name = name_buf;
        info.slot = slot;

        self.drivers[slot] = Some(info);
        if (pid as usize) < MAX_DRIVERS {
            self.driver_pid_to_slot[pid as usize] = slot as i32;
        }
        self.count += 1;

        Some(slot)
    }

    pub fn mark_initializing(&mut self, pid: u32) -> bool {
        for i in 0..MAX_DRIVERS {
            if let Some(info) = &mut self.drivers[i] {
                if info.pid == pid {
                    info.state = DriverState::Initializing as u8;
                    return true;
                }
            }
        }
        false
    }

    pub fn mark_running(&mut self, pid: u32) -> bool {
        for i in 0..MAX_DRIVERS {
            if let Some(info) = &mut self.drivers[i] {
                if info.pid == pid {
                    info.state = DriverState::Running as u8;
                    return true;
                }
            }
        }
        false
    }

    pub fn mark_faulted(&mut self, pid: u32) -> bool {
        for i in 0..MAX_DRIVERS {
            if let Some(info) = &mut self.drivers[i] {
                if info.pid == pid {
                    info.state = DriverState::Faulted as u8;
                    return true;
                }
            }
        }
        false
    }

    pub fn unload_driver(&mut self, pid: u32) -> bool {
        for i in 0..MAX_DRIVERS {
            if let Some(info) = &self.drivers[i] {
                if info.pid == pid {
                    for irq in 0..16 {
                        if self.irq_routing[irq] == i as i32 {
                            self.irq_routing[irq] = -1;
                        }
                    }
                    self.drivers[i] = None;
                    if (pid as usize) < MAX_DRIVERS {
                        self.driver_pid_to_slot[pid as usize] = -1;
                    }
                    if self.count > 0 {
                        self.count -= 1;
                    }
                    return true;
                }
            }
        }
        false
    }

    pub fn send_event(&mut self, pid: u32, event: DriverEvent) -> bool {
        for i in 0..MAX_DRIVERS {
            if let Some(info) = &mut self.drivers[i] {
                if info.pid == pid {
                    return info.push_event(event);
                }
            }
        }
        false
    }

    pub fn claim_irq(&mut self, pid: u32, irq: u8) -> bool {
        if irq as usize >= 16 {
            return false;
        }

        if self.irq_routing[irq as usize] >= 0 {
            for i in 0..MAX_DRIVERS {
                if let Some(info) = &self.drivers[i] {
                    if info.pid == pid && info.has_irq(irq) {
                        return true;
                    }
                }
            }
            return false;
        }

        for i in 0..MAX_DRIVERS {
            if let Some(info) = &mut self.drivers[i] {
                if info.pid == pid {
                    if info.claim_irq(irq) {
                        self.irq_routing[irq as usize] = i as i32;
                        return true;
                    }
                }
            }
        }
        false
    }

    pub fn release_irq(&mut self, pid: u32, irq: u8) -> bool {
        if irq as usize >= 16 {
            return false;
        }

        for i in 0..MAX_DRIVERS {
            if let Some(info) = &mut self.drivers[i] {
                if info.pid == pid && info.has_irq(irq) {
                    self.irq_routing[irq as usize] = -1;
                    let mut new_count = 0;
                    for j in 0..info.irq_count {
                        if info.claimed_irqs[j] != irq {
                            info.claimed_irqs[new_count] = info.claimed_irqs[j];
                            new_count += 1;
                        }
                    }
                    info.irq_count = new_count;
                    return true;
                }
            }
        }
        false
    }

    pub fn get_irq_driver(&self, irq: u8) -> Option<u32> {
        if irq as usize >= 16 {
            return None;
        }
        let slot = self.irq_routing[irq as usize];
        if slot < 0 {
            return None;
        }
        self.drivers[slot as usize].as_ref().map(|info| info.pid)
    }

    pub fn route_irq(&mut self, irq: u8) -> bool {
        if let Some(pid) = self.get_irq_driver(irq) {
            let event = DriverEvent::new(EVENT_IRQ, irq as u32, 0, 0);
            self.send_event(pid, event)
        } else {
            false
        }
    }

    pub fn get_driver(&self, pid: u32) -> Option<&DriverInfo> {
        for i in 0..MAX_DRIVERS {
            if let Some(info) = &self.drivers[i] {
                if info.pid == pid {
                    return Some(info);
                }
            }
        }
        None
    }

    pub fn get_driver_mut(&mut self, pid: u32) -> Option<&mut DriverInfo> {
        for i in 0..MAX_DRIVERS {
            if let Some(info) = &mut self.drivers[i] {
                if info.pid == pid {
                    return Some(info);
                }
            }
        }
        None
    }

    pub fn get_driver_by_slot(&self, slot: usize) -> Option<&DriverInfo> {
        if slot < MAX_DRIVERS {
            self.drivers[slot].as_ref()
        } else {
            None
        }
    }

    pub fn is_driver(&self, pid: u32) -> bool {
        self.get_driver(pid).is_some()
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn cleanup_faulted(&mut self) {
        for i in 0..MAX_DRIVERS {
            if let Some(info) = &self.drivers[i] {
                if info.state == DriverState::Faulted as u8 || info.state == DriverState::Unloading as u8 {
                    let pid = info.pid;
                    self.unload_driver(pid);
                }
            }
        }
    }
}

pub static mut DRIVER_MANAGER: DriverManager = DriverManager::new();

pub mod driver_syscalls {
    use super::*;
    use crate::vm::{Vm, Trap};

    pub fn sys_driver_register(vm: &mut Vm) -> Result<(), Trap> {
        let name_addr = vm.pop()?;
        let name_len = vm.pop()?;
        let driver_type = vm.pop()? as u8;
        let priority = vm.pop()? as u8;

        unsafe {
            if DRIVER_MANAGER.is_driver(vm.pid) {
                vm.push(0)?; 
                return Ok(());
            }
        }

        if name_len < 0 || name_len > 32 || name_addr < 0 {
            vm.push(-1)?;
            return Ok(());
        }

        let (addr, len) = (name_addr as usize, name_len as usize);
        if addr + len > crate::vm::MAX_MEM {
            vm.push(-1)?;
            return Ok(());
        }

        let mem = vm.memory();
        let name_bytes = &mem[addr..addr + len];
        let name = core::str::from_utf8(name_bytes).unwrap_or("driver");

        crate::serial_println!("[DRIVER] priority={}", priority);
        crate::serial_println!("[DRIVER] driver_type={}", driver_type);
        crate::serial_println!("[DRIVER] name_len={}", name_len);
        crate::serial_println!("[DRIVER] name_addr={}", name_addr);
        crate::serial_println!("[DRIVER] name={}", name);

        let dtype = DriverType::from_u8(driver_type);

        unsafe {
            if let Some(slot) = DRIVER_MANAGER.register_driver(vm.pid, name, dtype, priority) {
                crate::serial_println!("[DRIVER] Registered slot={}", slot);
                vm.push(slot as i32)?;
            } else {
                crate::serial_println!("[DRIVER] Register failed");
                vm.push(-1)?;
            }
        }

        Ok(())
    }

    pub fn sys_driver_ready(vm: &mut Vm) -> Result<(), Trap> {
        unsafe {
            if DRIVER_MANAGER.mark_running(vm.pid) {
                vm.push(1)?;
            } else {
                vm.push(0)?;
            }
        }
        Ok(())
    }

    pub fn sys_driver_io_port(vm: &mut Vm) -> Result<(), Trap> {
        let port = vm.pop()? as u16;
        let value = vm.pop()?;

        unsafe {
            if !DRIVER_MANAGER.is_driver(vm.pid) {
                return Err(Trap::UnknownSyscall(0xFF));
            }
        }

        let result = if value < 0 {
            unsafe { crate::cpu::inb(port) as i32 }
        } else {
            unsafe { crate::cpu::outb(port, value as u8); }
            0
        };

        vm.push(result)?;
        Ok(())
    }

    pub fn sys_driver_claim_irq(vm: &mut Vm) -> Result<(), Trap> {
        let irq = vm.pop()? as u8;

        unsafe {
            if !DRIVER_MANAGER.is_driver(vm.pid) {
                return Err(Trap::UnknownSyscall(0xFF));
            }
        }

        unsafe {
            if DRIVER_MANAGER.claim_irq(vm.pid, irq) {
                vm.push(1)?;
            } else {
                vm.push(0)?;
            }
        }

        Ok(())
    }

    pub fn sys_driver_release_irq(vm: &mut Vm) -> Result<(), Trap> {
        let irq = vm.pop()? as u8;

        unsafe {
            if !DRIVER_MANAGER.is_driver(vm.pid) {
                return Err(Trap::UnknownSyscall(0xFF));
            }
        }

        unsafe {
            if DRIVER_MANAGER.release_irq(vm.pid, irq) {
                vm.push(1)?;
            } else {
                vm.push(0)?;
            }
        }

        Ok(())
    }

    pub fn sys_driver_info(vm: &mut Vm) -> Result<(), Trap> {
        let slot = vm.pop()? as usize;
        let info_ptr = vm.pop()?;

        if info_ptr < 0 || info_ptr as usize + 64 > crate::vm::MAX_MEM {
            return Err(Trap::MemOutOfBounds);
        }

        unsafe {
            if let Some(info) = DRIVER_MANAGER.get_driver_by_slot(slot) {
                let ptr = info_ptr as usize;
                let mem = vm.memory_mut();

                mem[ptr..ptr + 4].copy_from_slice(&info.pid.to_le_bytes());
                mem[ptr + 4] = info.driver_type;
                mem[ptr + 5] = info.state;
                mem[ptr + 6] = info.priority;
                mem[ptr + 7] = 0;

                let name = info.name_str();
                let name_bytes = name.as_bytes();
                let len = name_bytes.len().min(32);
                mem[ptr + 8..ptr + 8 + len].copy_from_slice(&name_bytes[..len]);

                mem[ptr + 40..ptr + 44].copy_from_slice(&(info.irq_count as u32).to_le_bytes());
                mem[ptr + 44..ptr + 48].copy_from_slice(&(info.event_count as u32).to_le_bytes());

                vm.push(0)?;
            } else {
                vm.push(-1)?;
            }
        }

        Ok(())
    }

    pub fn sys_driver_unregister(vm: &mut Vm) -> Result<(), Trap> {
        unsafe {
            if DRIVER_MANAGER.unload_driver(vm.pid) {
                vm.push(1)?;
            } else {
                vm.push(0)?;
            }
        }
        Ok(())
    }

    pub fn sys_driver_send_event(vm: &mut Vm) -> Result<(), Trap> {
        let data3 = vm.pop()? as u32;
        let data2 = vm.pop()? as u32;
        let data1 = vm.pop()? as u32;
        let event_type = vm.pop()? as u32;
        let to_pid = vm.pop()? as u32;

        let event = DriverEvent::new(event_type, data1, data2, data3);

        unsafe {
            if DRIVER_MANAGER.send_event(to_pid, event) {
                vm.push(1)?;
            } else {
                vm.push(0)?;
            }
        }

        Ok(())
    }

    pub fn sys_driver_wait_event(vm: &mut Vm) -> Result<(), Trap> {
        unsafe {
            if !DRIVER_MANAGER.is_driver(vm.pid) {
                return Err(Trap::UnknownSyscall(0xFF));
            }

            if let Some(info) = DRIVER_MANAGER.get_driver_mut(vm.pid) {
                if let Some(event) = info.pop_event() {
                    vm.push(event.event_type as i32)?;
                    return Ok(());
                }
            }
        }

        // FIXED: Thêm logic rewind_pc và sleep đúng chuẩn blocking
        vm.rewind_pc(2)?;
        let current_ticks = crate::timer::get_ticks();
        vm.sleep_until = current_ticks + 1;
        Ok(())
    }

    pub fn sys_driver_poll_event(vm: &mut Vm) -> Result<(), Trap> {
        unsafe {
            if !DRIVER_MANAGER.is_driver(vm.pid) {
                return Err(Trap::UnknownSyscall(0xFF));
            }

            if let Some(info) = DRIVER_MANAGER.get_driver_mut(vm.pid) {
                if let Some(event) = info.pop_event() {
                    vm.push(event.event_type as i32)?;
                    return Ok(());
                }
            }
        }

        vm.push(0)?;
        Ok(())
    }

    pub fn sys_driver_get_state(vm: &mut Vm) -> Result<(), Trap> {
        let pid = vm.pop()? as u32;

        unsafe {
            if let Some(info) = DRIVER_MANAGER.get_driver(pid) {
                vm.push(info.state as i32)?;
            } else {
                vm.push(DriverState::Unloaded as i32)?;
            }
        }

        Ok(())
    }

    pub fn sys_driver_get_count(vm: &mut Vm) -> Result<(), Trap> {
        unsafe {
            vm.push(DRIVER_MANAGER.count() as i32)?;
        }
        Ok(())
    }
}

pub fn load_driver(package_name: &str, driver_type: DriverType, _priority: u8) -> Option<u32> {
    if unsafe { crate::initrd::INITRD_ADDR.is_null() } {
        crate::serial_println!("ERROR: Initrd is not loaded.");
        return None;
    }

    let pid = crate::process::create_process_from_package(package_name);

    if let Some(pid) = pid {
        unsafe {
            DRIVER_MANAGER.mark_initializing(pid);
            let _ = crate::process::run_process(pid, 50);

            if DRIVER_MANAGER.is_driver(pid) {
                let registered_type = DRIVER_MANAGER.get_driver(pid)
                    .map(|info| info.driver_type_enum())
                    .unwrap_or(driver_type);

                crate::serial_println!("[Driver] Loaded '{}' as {} driver (PID {})",
                    package_name, registered_type.as_str(), pid);
                return Some(pid);
            } else {
                crate::process::kill_process(pid);
                crate::serial_println!("[Driver] Failed to register '{}'", package_name);
                return None;
            }
        }
    }

    None
}

pub fn run_drivers(steps_per_driver: u32) {
    unsafe {
        let mut faulted_pids = [0u32; MAX_DRIVERS];
        let mut fault_count = 0;

        for i in 0..MAX_DRIVERS {
            if let Some(info) = DRIVER_MANAGER.get_driver_by_slot(i) {
                if info.state == DriverState::Running as u8 ||
                   info.state == DriverState::Loaded as u8 ||
                   info.state == DriverState::Initializing as u8 {
                    
                    let success = crate::process::run_process(info.pid, steps_per_driver);
                    if !success {
                        crate::serial_println!("[DRIVER] PID {} faulted/stopped. Stopping driver.", info.pid);
                        faulted_pids[fault_count] = info.pid;
                        fault_count += 1;
                    }
                }
            }
        }

        for i in 0..fault_count {
            DRIVER_MANAGER.mark_faulted(faulted_pids[i]);
        }

        DRIVER_MANAGER.cleanup_faulted();
    }
}

pub fn list_drivers() {
    unsafe {
        crate::println!("SLOT | PID  | TYPE    | STATE     | PRI | IRQ | NAME");
        crate::println!("-----|------|---------|-----------|-----|-----|------------------");

        for i in 0..MAX_DRIVERS {
            if let Some(info) = DRIVER_MANAGER.get_driver_by_slot(i) {
                let mut irq_buf = [0u8; 32];
                let mut irq_pos = 0;
                for j in 0..info.irq_count {
                    if j > 0 && irq_pos < 31 {
                        irq_buf[irq_pos] = b',';
                        irq_pos += 1;
                    }
                    if irq_pos < 31 {
                        let digit = info.claimed_irqs[j];
                        if digit >= 10 {
                            irq_buf[irq_pos] = b'0' + (digit / 10);
                            irq_pos += 1;
                            if irq_pos < 31 {
                                irq_buf[irq_pos] = b'0' + (digit % 10);
                                irq_pos += 1;
                            }
                        } else {
                            irq_buf[irq_pos] = b'0' + digit;
                            irq_pos += 1;
                        }
                    }
                }
                let irq_str = core::str::from_utf8(&irq_buf[..irq_pos]).unwrap_or("none");

                crate::println!("{:3}  | {:4} | {:7} | {:9} | {:3} | {:3} | {}",
                    i, info.pid, info.driver_type_enum().as_str(),
                    info.state_enum().as_str(), info.priority, irq_str, info.name_str());
            }
        }
    }
}

pub fn get_driver_info(pid: u32) -> Option<DriverInfo> {
    unsafe {
        if let Some(info) = DRIVER_MANAGER.get_driver(pid) {
            Some(info.clone())
        } else {
            None
        }
    }
}

pub fn send_event_to_driver(pid: u32, event_type: u32, data1: u32, data2: u32, data3: u32) -> bool {
    let event = DriverEvent::new(event_type, data1, data2, data3);
    unsafe { DRIVER_MANAGER.send_event(pid, event) }
}

pub fn route_irq(irq: u8) -> bool {
    unsafe { DRIVER_MANAGER.route_irq(irq) }
}

pub fn init_builtin_drivers() {
    unsafe {
        if crate::initrd::INITRD_ADDR.is_null() {
            return;
        }

        let mut tar_ptr = crate::initrd::INITRD_ADDR;
        let mut count = 0;

        loop {
            let header = &*(tar_ptr as *const crate::initrd::TarHeader);
            if header.name[0] == 0 {
                break;
            }

            let mut name_len = 0;
            while name_len < header.name.len() && header.name[name_len] != 0 {
                name_len += 1;
            }

            if let Ok(name) = core::str::from_utf8(&header.name[..name_len]) {
                if name.ends_with(".drv") {
                    let driver_type = detect_driver_type(name);
                    let _ = load_driver(name, driver_type, 5);
                    count += 1;
                }
            }

            let file_size = crate::initrd::octal_to_u32(&header.size);
            let blocks = (file_size + 511) / 512;
            let skip_size = 512 + (blocks * 512) as usize;
            tar_ptr = tar_ptr.add(skip_size);
        }

        if count > 0 {
            crate::serial_println!("[Driver] Loaded {} built-in drivers", count);
        }
    }
}

fn detect_driver_type(name: &str) -> DriverType {
    let mut name_lower_buf = [0u8; 64];
    let name_bytes = name.as_bytes();
    let len = name_bytes.len().min(63);
    name_lower_buf[..len].copy_from_slice(&name_bytes[..len]);
    
    for i in 0..len {
        if name_lower_buf[i] >= b'A' && name_lower_buf[i] <= b'Z' {
            name_lower_buf[i] = name_lower_buf[i] + 32;
        }
    }
    
    let name_lower = core::str::from_utf8(&name_lower_buf[..len]).unwrap_or(name);
    
    if name_lower.contains("kbd") || name_lower.contains("keyboard") {
        DriverType::Input
    } else if name_lower.contains("mouse") || name_lower.contains("hid") {
        DriverType::Hid
    } else if name_lower.contains("fb") || name_lower.contains("vesa") || name_lower.contains("gpu") {
        DriverType::Display
    } else if name_lower.contains("audio") || name_lower.contains("sound") || name_lower.contains("beep") || name_lower.contains("spk") {
        DriverType::Audio
    } else if name_lower.contains("pci") || name_lower.contains("usb") {
        DriverType::Bus
    } else if name_lower.contains("net") || name_lower.contains("eth") {
        DriverType::Net
    } else if name_lower.contains("serial") || name_lower.contains("tty") {
        DriverType::Char
    } else if name_lower.contains("block") || name_lower.contains("disk") {
        DriverType::Block
    } else {
        DriverType::Unknown
    }
}

pub fn event_key(scancode: u8, ascii: u8, pressed: bool) -> DriverEvent {
    let flags = if pressed { 1 } else { 0 };
    DriverEvent::new(EVENT_KEY, scancode as u32, ascii as u32, flags)
}

pub fn event_mouse(x: i32, y: i32, buttons: u8) -> DriverEvent {
    DriverEvent::new(EVENT_MOUSE, x as u32, y as u32, buttons as u32)
}

pub fn event_irq(irq: u8) -> DriverEvent {
    DriverEvent::new(EVENT_IRQ, irq as u32, 0, 0)
}

pub fn event_timer(ticks: u64) -> DriverEvent {
    DriverEvent::new(EVENT_TIMER, (ticks & 0xFFFFFFFF) as u32, ((ticks >> 32) & 0xFFFFFFFF) as u32, 0)
}