// src/process.rs

use crate::vm::{Vm, Trap};

pub const MAX_PROCESSES: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProcessState {
    Ready,
    Running,
    Blocked,
    Terminated,
}

pub struct Process {
    pub pid: u32,
    pub name: [u8; 32],
    pub state: ProcessState,
    pub vm: Option<Vm<'static>>,
    pub package_name: [u8; 64],
    pub exit_code: i32,
    pub steps_run: u32,
}

impl Process {
    pub fn new(pid: u32, name: &str, vm: Vm<'static>) -> Self {
        let mut name_buf = [0u8; 32];
        let name_bytes = name.as_bytes();
        let len = name_bytes.len().min(31);
        name_buf[..len].copy_from_slice(&name_bytes[..len]);
        
        Process {
            pid,
            name: name_buf,
            state: ProcessState::Ready,
            vm: Some(vm),
            package_name: [0u8; 64],
            exit_code: 0,
            steps_run: 0,
        }
    }
}

pub struct ProcessManager {
    processes: [Option<Process>; MAX_PROCESSES],
    next_pid: u32,
    current_pid: u32,
    terminated_pids: [u32; MAX_PROCESSES],
    terminated_count: usize,
}

impl ProcessManager {
    pub const fn new() -> Self {
        ProcessManager {
            processes: [const { None }; MAX_PROCESSES],
            next_pid: 1,
            current_pid: 0,
            terminated_pids: [0; MAX_PROCESSES],
            terminated_count: 0,
        }
    }
    
    pub fn create_process(&mut self, name: &str, mut vm: Vm<'static>) -> Option<u32> {
        for i in 0..MAX_PROCESSES {
            if self.processes[i].is_none() {
                let pid = self.next_pid;
                self.next_pid += 1;
                
                vm.set_pid(pid);
                
                let mut process = Process::new(pid, name, vm);
                process.state = ProcessState::Ready;
                
                self.processes[i] = Some(process);
                return Some(pid);
            }
        }
        None
    }
    
    pub fn get_process(&self, pid: u32) -> Option<&Process> {
        for i in 0..MAX_PROCESSES {
            if let Some(ref p) = self.processes[i] {
                if p.pid == pid {
                    return Some(p);
                }
            }
        }
        None
    }
    
    pub fn get_process_mut(&mut self, pid: u32) -> Option<&mut Process> {
        for i in 0..MAX_PROCESSES {
            if let Some(ref mut p) = self.processes[i] {
                if p.pid == pid {
                    return Some(p);
                }
            }
        }
        None
    }
    
    pub fn terminate_process(&mut self, pid: u32) -> bool {
        for i in 0..MAX_PROCESSES {
            if let Some(ref mut p) = self.processes[i] {
                if p.pid == pid {
                    p.state = ProcessState::Terminated;
                    if self.terminated_count < MAX_PROCESSES {
                        self.terminated_pids[self.terminated_count] = pid;
                        self.terminated_count += 1;
                    }
                    return true;
                }
            }
        }
        false
    }
    
    pub fn cleanup_terminated(&mut self) {
        let mut new_count = 0;
        for i in 0..MAX_PROCESSES {
            if let Some(p) = self.processes[i].take() {
                if p.state != ProcessState::Terminated {
                    self.processes[new_count] = Some(p);
                    new_count += 1;
                }
            }
        }
        for i in new_count..MAX_PROCESSES {
            self.processes[i] = None;
        }
        
        self.terminated_count = 0;
    }
    
    pub fn list_processes(&self) -> [(u32, &str, ProcessState); MAX_PROCESSES] {
        let mut result = [(0, "", ProcessState::Terminated); MAX_PROCESSES];
        let mut count = 0;
        
        for i in 0..MAX_PROCESSES {
            if let Some(ref p) = self.processes[i] {
                if count < MAX_PROCESSES {
                    let name = core::str::from_utf8(&p.name).unwrap_or("unknown");
                    result[count] = (p.pid, name, p.state);
                    count += 1;
                }
            }
        }
        result
    }
}

pub static mut PROCESS_MANAGER: ProcessManager = ProcessManager::new();

pub fn create_process_from_package(package_name: &str) -> Option<u32> {
    if unsafe { crate::initrd::INITRD_ADDR.is_null() } {
        crate::println!("ERROR: Initrd is not loaded.");
        return None;
    }
    
    unsafe {
        let package_bytes = match crate::initrd::find_file_in_tar(crate::initrd::INITRD_ADDR, package_name) {
            Some(b) => b,
            None => {
                crate::println!("ERROR: Package '{}' not found.", package_name);
                return None;
            }
        };
        
        if package_bytes.len() < 512 {
            crate::println!("ERROR: Package '{}' too small.", package_name);
            return None;
        }
        
        let bytecode = match crate::initrd::find_file_in_tar(package_bytes.as_ptr(), "main.bc") {
            Some(b) => b,
            None => {
                crate::println!("ERROR: 'main.bc' not found in '{}'.", package_name);
                return None;
            }
        };
        
        let mut vm = Vm::new(bytecode);
        
        if let Some(data) = crate::initrd::find_file_in_tar(package_bytes.as_ptr(), "data.bin") {
            vm.load_initial_memory(data);
        }
        
        if let Some(entry_data) = crate::initrd::find_file_in_tar(package_bytes.as_ptr(), "entry.bin") {
            if entry_data.len() >= 4 {
                let mut buf = [0u8; 4];
                buf.copy_from_slice(&entry_data[0..4]);
                let entry_pc = u32::from_le_bytes(buf) as usize;
                let _ = vm.set_pc(entry_pc);
            }
        }
        
        let pid = PROCESS_MANAGER.create_process(package_name, vm);
        
        if let Some(pid) = pid {
            if let Some(proc) = PROCESS_MANAGER.get_process_mut(pid) {
                let name_bytes = package_name.as_bytes();
                let len = name_bytes.len().min(63);
                proc.package_name[..len].copy_from_slice(&name_bytes[..len]);
            }
        }
        
        pid
    }
}

pub fn run_process(pid: u32, steps: u32) -> bool {
    unsafe {
        if let Some(proc) = PROCESS_MANAGER.get_process_mut(pid) {
            if proc.state == ProcessState::Terminated {
                return false;
            }
            
            proc.state = ProcessState::Running;
            
            if let Some(ref mut vm) = proc.vm {
                match vm.run(steps) {
                    Ok(actual_steps) => {
                        proc.steps_run += actual_steps;
                        if vm.halted {
                            proc.state = ProcessState::Terminated;
                        } else {
                            proc.state = ProcessState::Ready;
                        }
                        return true;
                    }
                    Err(Trap::Halted) => {
                        proc.state = ProcessState::Terminated;
                        return true;
                    }
                    Err(trap) => {
                        proc.state = ProcessState::Terminated;
                        
                        let is_driver = crate::driver::DRIVER_MANAGER.is_driver(pid);
                        if is_driver {
                            // Format và đẩy thẳng ra Serial (không làm rác màn hình console chính)
                            struct SerialWriter<'a> {
                                buf: &'a mut [u8],
                                pos: usize,
                            }
                            impl<'a> core::fmt::Write for SerialWriter<'a> {
                                fn write_str(&mut self, s: &str) -> core::fmt::Result {
                                    let bytes = s.as_bytes();
                                    if self.pos + bytes.len() > self.buf.len() {
                                        return Err(core::fmt::Error);
                                    }
                                    self.buf[self.pos..self.pos + bytes.len()].copy_from_slice(bytes);
                                    self.pos += bytes.len();
                                    Ok(())
                                }
                            }

                            let mut buf = [0u8; 128];
                            let mut writer = SerialWriter { buf: &mut buf, pos: 0 };
                            if core::fmt::write(&mut writer, format_args!("\n[Driver {}] Trapped: {} (at step {})\n", 
                                pid, crate::vm::trap_name(trap), proc.steps_run)).is_ok() 
                            {
                                if let Ok(s) = core::str::from_utf8(&writer.buf[..writer.pos]) {
                                    crate::serial::serial_write(s);
                                }
                            }
                        } else {
                            // Ứng dụng thông thường vẫn in ra console màn hình
                            crate::println!("\n[Process {}] Trapped: {} (at step {})", 
                                pid, crate::vm::trap_name(trap), proc.steps_run);
                        }
                        
                        return false;
                    }
                }
            }
        }
        false
    }
}

pub fn kill_process(pid: u32) -> bool {
    unsafe {
        if PROCESS_MANAGER.terminate_process(pid) {
            return true;
        }
        false
    }
}

pub fn list_processes() {
    unsafe {
        let processes = PROCESS_MANAGER.list_processes();
        crate::println!("PID | State      | Steps | Name");
        crate::println!("----|------------|-------|------------------");
        
        for (pid, name, state) in processes.iter() {
            if *pid != 0 {
                let state_str = match state {
                    ProcessState::Ready => "Ready",
                    ProcessState::Running => "Running",
                    ProcessState::Blocked => "Blocked",
                    ProcessState::Terminated => "Terminated",
                };
                
                let steps = if let Some(proc) = PROCESS_MANAGER.get_process(*pid) {
                    proc.steps_run
                } else {
                    0
                };
                
                crate::println!("{:3} | {:10} | {:5} | {}", 
                    pid, state_str, steps, name);
            }
        }
    }
}

pub fn get_process_state(pid: u32) -> Option<ProcessState> {
    unsafe {
        if let Some(proc) = PROCESS_MANAGER.get_process(pid) {
            Some(proc.state)
        } else {
            None
        }
    }
}