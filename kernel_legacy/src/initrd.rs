// src/initrd.rs

pub static mut INITRD_ADDR: *const u8 = core::ptr::null();

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
    let mut current_ptr = tar_start;
    
    loop {
        let header = &*(current_ptr as *const TarHeader);
        
        if header.name[0] == 0 {
            break;
        }
        
        let file_size = octal_to_u32(&header.size);
        
        if name_matches(&header.name, target_filename) {
            let data_ptr = current_ptr.add(512);
            let slice = core::slice::from_raw_parts(data_ptr, file_size as usize);
            return Some(slice);
        }
        
        let blocks = (file_size + 511) / 512;
        let skip_size = 512 + (blocks * 512) as usize;
        current_ptr = current_ptr.add(skip_size);
    }
    
    None
}

pub fn list_files() {
    unsafe {
        if INITRD_ADDR.is_null() {
            crate::println!("ERROR: No initrd loaded in memory.");
            return;
        }

        let mut current_ptr = INITRD_ADDR;
        
        loop {
            let header = &*(current_ptr as *const TarHeader);
            
            if header.name[0] == 0 {
                break;
            }
            
            let mut name_len = 0;
            while name_len < header.name.len() && header.name[name_len] != 0 {
                name_len += 1;
            }
            
            if let Ok(name_str) = core::str::from_utf8(&header.name[..name_len]) {
                let size = octal_to_u32(&header.size);
                crate::println!("- {} ({} bytes)", name_str, size);
            }
            
            let file_size = octal_to_u32(&header.size);
            let blocks = (file_size + 511) / 512;
            let skip_size = 512 + (blocks * 512) as usize;
            current_ptr = current_ptr.add(skip_size);
        }
    }
}