// src/initrd.rs
use core::sync::atomic::{AtomicBool, Ordering};

pub static mut INITRD_ADDR: *const u8 = core::ptr::null();
static INITRD_LOCK: AtomicBool = AtomicBool::new(false);

#[repr(C, packed)]
pub struct TarHeader {
    pub name: [u8; 100],
    pub mode: [u8; 8],
    pub uid: [u8; 8],
    pub gid: [u8; 8],
    pub size: [u8; 12],
    pub mtime: [u8; 12],
    pub chksum: [u8; 8],
    pub typeflag: u8,
    pub linkname: [u8; 100],
    pub magic: [u8; 6],
}

pub fn octal_to_u32(octal_bytes: &[u8]) -> u32 {
    let mut result: u32 = 0;
    for &b in octal_bytes {
        if b >= b'0' && b <= b'7' {
            result = result * 8 + (b - b'0') as u32;
        } else {
            break;
        }
    }
    result
}

fn name_matches(tar_name: &[u8], target: &str) -> bool {
    let mut name_len = 0;
    while name_len < tar_name.len() && tar_name[name_len] != 0 {
        name_len += 1;
    }

    let actual_name = &tar_name[..name_len];
    let target_bytes = target.as_bytes();

    if actual_name.len() < target_bytes.len() {
        return false;
    }

    let suffix = &actual_name[actual_name.len() - target_bytes.len()..];

    if suffix == target_bytes {
        if actual_name.len() == target_bytes.len() || actual_name[actual_name.len() - target_bytes.len() - 1] == b'/' {
            return true;
        }
    }

    false
}

pub unsafe fn find_file_in_tar(tar_start: *const u8, target_filename: &str) -> Option<&'static [u8]> {
    // Kiểm tra null
    if tar_start.is_null() {
        return None;
    }

    // Acquire lock
    while INITRD_LOCK.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
        core::hint::spin_loop();
    }

    let mut current_ptr = tar_start;
    let result = loop {
        let header = &*(current_ptr as *const TarHeader);

        if header.name[0] == 0 {
            break None;
        }

        let file_size = octal_to_u32(&header.size);

        if name_matches(&header.name, target_filename) {
            let data_ptr = current_ptr.add(512);
            let slice = core::slice::from_raw_parts(data_ptr, file_size as usize);
            break Some(slice);
        }

        let blocks = (file_size + 511) / 512;
        let skip_size = 512 + (blocks * 512) as usize;
        current_ptr = current_ptr.add(skip_size);
    };

    // Release lock
    INITRD_LOCK.store(false, Ordering::Release);
    
    result
}