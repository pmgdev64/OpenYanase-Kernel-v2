// src/abp.rs
use crate::initrd;
use crate::process;

pub fn run_abp_file(tar_start: *const u8, filename: &str) -> Result<(), &'static str> {
    if !filename.ends_with(".abp") {
        return Err("not a .abp file");
    }

    let abp_bytes = unsafe { initrd::find_file_in_tar(tar_start, filename) }
        .ok_or("file not found in package store")?;

    let manifest = unsafe { initrd::find_file_in_tar(abp_bytes.as_ptr(), "manifest.txt") }
        .ok_or("manifest.txt missing in .abp")?;

    let entry_name = parse_entry_from_manifest(manifest)
        .ok_or("invalid manifest")?;

    let ybc_bytes = unsafe { initrd::find_file_in_tar(abp_bytes.as_ptr(), entry_name) }
        .ok_or("entry .ybc not found in .abp")?;

    let pid = process::spawn_ybc(filename, ybc_bytes)?;
    process::run_to_completion(pid)?;
    
    Ok(())
}

fn parse_entry_from_manifest(data: &[u8]) -> Option<&'static str> {
    let text = core::str::from_utf8(data).ok()?;
    
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(v) = trimmed.strip_prefix("entry=") 
            .or_else(|| trimmed.strip_prefix("entry:"))
            .or_else(|| trimmed.strip_prefix("ENTRY="))
        {
            let entry = v.trim();
            if entry.ends_with(".ybc") && !entry.is_empty() {
                // Return as static str using static buffer
                static mut ENTRY_BUF: [u8; 64] = [0; 64];
                unsafe {
                    let bytes = entry.as_bytes();
                    let len = bytes.len().min(63);
                    ENTRY_BUF[..len].copy_from_slice(&bytes[..len]);
                    ENTRY_BUF[len] = 0;
                    if let Ok(s) = core::str::from_utf8(&ENTRY_BUF[..len]) {
                        return Some(s);
                    }
                }
            }
        }
    }
    None
}