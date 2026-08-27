// src/appload.rs

use crate::process;
// src/appload.rs - Thêm vào cuối file

use crate::driver::{DriverType, load_driver};

/// Chạy app package (blocking - chạy xong mới trả về)
pub fn run_app_package(package_name: &str) {
    if unsafe { crate::initrd::INITRD_ADDR.is_null() } {
        crate::println!("ERROR: Initrd is not loaded.");
        return;
    }

    // Tạo process từ package
    let pid = process::create_process_from_package(package_name);
    
    if let Some(pid) = pid {
        // Chạy từng slice 100 steps cho tới khi process Terminated hẳn
        loop {
            let ran = process::run_process(pid, 100);
            if !ran {
                break;
            }
            
            if let Some(state) = process::get_process_state(pid) {
                if state == process::ProcessState::Terminated {
                    break;
                }
            } else {
                break;
            }
        }
        
        // Clean up
        process::kill_process(pid);
        unsafe {
            process::PROCESS_MANAGER.cleanup_terminated();
        }
    }
}

/// Chạy app trong background (non-blocking)
pub fn run_app_background(package_name: &str) -> Option<u32> {
    if unsafe { crate::initrd::INITRD_ADDR.is_null() } {
        crate::println!("ERROR: Initrd is not loaded.");
        return None;
    }
    
    process::create_process_from_package(package_name)
}

/// Chạy app với số bước giới hạn (dùng cho scheduler)
pub fn run_app_steps(package_name: &str, steps: u32) -> Option<u32> {
    if unsafe { crate::initrd::INITRD_ADDR.is_null() } {
        crate::println!("ERROR: Initrd is not loaded.");
        return None;
    }

    let pid = process::create_process_from_package(package_name);
    
    if let Some(pid) = pid {
        process::run_process(pid, steps);
        Some(pid)
    } else {
        None
    }
}

/// Tiếp tục chạy process đã có
pub fn continue_process(pid: u32, steps: u32) -> bool {
    process::run_process(pid, steps)
}

/// Load a driver package (registers with driver manager)
pub fn load_driver_package(package_name: &str, driver_type: DriverType, priority: u8) -> Option<u32> {
    load_driver(package_name, driver_type, priority)
}

/// Load driver by type name
pub fn load_driver_by_name(package_name: &str, type_name: &str, priority: u8) -> Option<u32> {
    let dtype = match type_name {
        "block" => DriverType::Block,
        "net" => DriverType::Net,
        "input" => DriverType::Input,
        "display" => DriverType::Display,
        "audio" => DriverType::Audio,
        "hid" => DriverType::Hid,
        "bus" => DriverType::Bus,
        "char" => DriverType::Char,
        _ => DriverType::Unknown,
    };
    load_driver(package_name, dtype, priority)
}