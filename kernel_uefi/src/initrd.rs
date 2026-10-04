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

pub fn normalize_tar_name(s: &str) -> &str {
    let mut name = s;
    if name.starts_with("./") {
        name = &name[2..];
    }
    if name.starts_with('/') {
        name = &name[1..];
    }
    name
}

pub unsafe fn find_file_in_tar(tar_start: *const u8, target_filename: &str) -> Option<&'static [u8]> {
    if tar_start.is_null() {
        return None;
    }

    while INITRD_LOCK.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
        core::hint::spin_loop();
    }

    let mut current_ptr = tar_start;
    let clean_target = normalize_tar_name(target_filename).trim_matches('/');

    let result = loop {
        let header = &*(current_ptr as *const TarHeader);

        if header.name[0] == 0 {
            break None;
        }

        let file_size = octal_to_u32(&header.size);
        let mut name_len = 0;
        while name_len < header.name.len() && header.name[name_len] != 0 {
            name_len += 1;
        }

        if let Ok(raw_name) = core::str::from_utf8(&header.name[..name_len]) {
            let clean_name = normalize_tar_name(raw_name).trim_matches('/');
            if clean_name == clean_target {
                let data_ptr = current_ptr.add(512);
                let slice = core::slice::from_raw_parts(data_ptr, file_size as usize);
                break Some(slice);
            }
        }

        let blocks = (file_size + 511) / 512;
        let skip_size = 512 + (blocks * 512) as usize;
        current_ptr = current_ptr.add(skip_size);
    };

    INITRD_LOCK.store(false, Ordering::Release);
    result
}

// Sửa F: FnMut lấy tham số &'static str để truyền đúng lifetime ra ngoài closure
pub unsafe fn for_each_file_with_prefix<F: FnMut(&'static str, u8, &'static [u8])>(
    tar_start: *const u8,
    prefix: &str,
    mut callback: F,
) {
    if tar_start.is_null() {
        return;
    }

    while INITRD_LOCK.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
        core::hint::spin_loop();
    }

    let mut current_ptr = tar_start;
    loop {
        let header = &*(current_ptr as *const TarHeader);
        if header.name[0] == 0 {
            break;
        }

        let mut name_len = 0;
        while name_len < header.name.len() && header.name[name_len] != 0 {
            name_len += 1;
        }

        let file_size = octal_to_u32(&header.size);

        // Tạo slice &'static trực tiếp từ vùng nhớ Initrd RAM
        let name_bytes = core::slice::from_raw_parts(header.name.as_ptr(), name_len);
        if let Ok(raw_name) = core::str::from_utf8(name_bytes) {
            let clean_name = normalize_tar_name(raw_name);
            let tf = header.typeflag;
            if (tf == b'0' || tf == 0 || tf == b'5') && clean_name.starts_with(prefix) {
                let data_ptr = current_ptr.add(512);
                let slice = core::slice::from_raw_parts(data_ptr, file_size as usize);
                callback(clean_name, tf, slice);
            }
        }

        let blocks = (file_size + 511) / 512;
        let skip_size = 512 + (blocks * 512) as usize;
        current_ptr = current_ptr.add(skip_size);
    }

    INITRD_LOCK.store(false, Ordering::Release);
}