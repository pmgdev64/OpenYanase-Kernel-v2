// src/abp.rs
use crate::initrd;
use crate::process;
use crate::serial;

pub fn run_abp_file(tar_start: *const u8, filename: &str) -> Result<(), &'static str> {
    if !filename.ends_with(".abp") {
        return Err("not a .abp file");
    }

    let abp_bytes = unsafe { initrd::find_file_in_tar(tar_start, filename) }
        .ok_or("file not found in package store")?;

    // .abp chính là 1 tar lồng bên trong — chứa manifest.txt + main.ybc
    let manifest = unsafe { initrd::find_file_in_tar(abp_bytes.as_ptr(), "manifest.txt") }
        .ok_or("manifest.txt missing in .abp")?;

    let entry_name = parse_entry_from_manifest(manifest).ok_or("invalid manifest")?;

    let ybc_bytes = unsafe { initrd::find_file_in_tar(abp_bytes.as_ptr(), entry_name) }
        .ok_or("entry .ybc not found in .abp")?;

    let pid = process::spawn_ybc(filename, ybc_bytes)?;
    serial::serial_write_str("ABP: process spawned, running...\r\n");

    process::run_to_completion(pid)?;
    Ok(())
}

fn parse_entry_from_manifest(data: &[u8]) -> Option<&'static str> {
    let text = core::str::from_utf8(data).ok()?;
    for line in text.lines() {
        if let Some(v) = line.strip_prefix("entry=") {
            let trimmed = v.trim();
            // Vì initrd::find_file_in_tar cần &'static str, và manifest nằm trong buffer tạm,
            // ta chấp nhận subset tên cố định phổ biến để tránh lifetime phức tạp ở bản đầu.
            // Cách đúng lâu dài: đổi find_file_in_tar nhận &[u8] thay vì &'static str.
            return match trimmed {
                "main.ybc" => Some("main.ybc"),
                _ => None,
            };
        }
    }
    None
}