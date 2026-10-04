// src/driver_loader.rs
use crate::initrd;
use crate::kvm::KernelVm;
use crate::kvm_guard::{mint_driver_token, Capabilities};
use crate::ybc;
use crate::serial;
use crate::driver::{self, DRIVER_MANAGER};

const DRIVER_DIR_PREFIX: &str = "globalsys/drivers/";
const MAX_DRIVERS: usize = 16;

const YBC_MAGIC: u32 = 0x59424331;
const TAR_BLOCK: usize = 512;
const MAX_TAR_ENTRIES: usize = 64;

fn preset_for_driver(name: &str) -> Capabilities {
    match name {
        "ps2.drv" | "ps2_mouse.drv" | "ps2_keyboard.drv" => Capabilities::DRIVER_PS2,
        "timer.drv" | "pit.drv" => Capabilities::DRIVER_TIMER,
        "ata.drv" | "ide.drv" | "block.drv" => Capabilities::DRIVER_BLOCK,
        _ => Capabilities::NONE,
    }
}

/// Scan tar ustar tìm file `.ybc` đầu tiên.
unsafe fn find_ybc_in_tar(tar_start: *const u8) -> Option<&'static [u8]> {
    if tar_start.is_null() { return None; }

    let mut current = tar_start;
    let mut scanned = 0usize;

    while scanned < MAX_TAR_ENTRIES {
        scanned += 1;

        let first_byte = core::ptr::read_volatile(current);
        if first_byte == 0 { return None; }

        let header = &*(current as *const initrd::TarHeader);

        let mut name_len = 0usize;
        while name_len < 100 && header.name[name_len] != 0 { name_len += 1; }
        if name_len == 0 { return None; }

        let raw_name = core::str::from_utf8(&header.name[..name_len]).ok()?;
        let clean_name = initrd::normalize_tar_name(raw_name);

        let size = initrd::octal_to_u32(&header.size) as usize;

        if clean_name.ends_with(".ybc") && size > 0 {
            let data_ptr = current.add(TAR_BLOCK);
            return Some(core::slice::from_raw_parts(data_ptr, size));
        }

        let data_blocks = (size + TAR_BLOCK - 1) / TAR_BLOCK;
        current = current.add(TAR_BLOCK + data_blocks * TAR_BLOCK);
    }
    None
}

/// Trích YBC bytecode: RAW (magic match) hoặc TAR (chứa *.ybc).
fn extract_ybc(drv_bytes: &'static [u8]) -> Option<&'static [u8]> {
    if drv_bytes.len() < 4 { return None; }

    let magic = u32::from_le_bytes([drv_bytes[0], drv_bytes[1], drv_bytes[2], drv_bytes[3]]);
    if magic == YBC_MAGIC { return Some(drv_bytes); }

    unsafe { find_ybc_in_tar(drv_bytes.as_ptr()) }
}

/// Load .drv: extract → verify → mint token → build VM → register.
/// Token KHÔNG revoke — sống suốt driver lifecycle (revoke khi unload/fault).
fn load_driver(
    name: &str,
    drv_bytes: &'static [u8],
    preset: Capabilities,
) -> Result<u32, &'static str> {
    let ybc_bytes = extract_ybc(drv_bytes).ok_or("no .ybc entry found in .drv")?;

    ybc::validate_ybc(ybc_bytes)?;
    let header = ybc::parse_header(ybc_bytes).ok_or("header parse failed")?;

    let token = mint_driver_token(preset).ok_or("no free capability token slot")?;

    let vm = match KernelVm::new(ybc_bytes, header, token) {
        Ok(v) => v,
        Err(_) => {
            crate::kvm_guard::revoke(token);
            return Err("vm init rejected");
        }
    };

    let dtype = driver::detect_driver_type(name);
    let pid = unsafe {
        DRIVER_MANAGER.register(name, dtype, 5, token, vm)
            .ok_or("driver table full")?
    };

    Ok(pid)
}

pub fn autoload_all_drivers() {
    let tar_start = unsafe { initrd::INITRD_ADDR };
    if tar_start.is_null() {
        serial::serial_write_str("DRIVER: initrd not loaded, skipping autoload\r\n");
        return;
    }

    serial::serial_write_str("DRIVER: Scanning /globalsys/drivers ...\r\n");

    let mut loaded = 0u32;
    let mut denied = 0u32;

    unsafe {
        initrd::for_each_file_with_prefix(tar_start, DRIVER_DIR_PREFIX, |name, _tf, data| {
            if loaded + denied >= MAX_DRIVERS as u32 { return; }
            if !name.ends_with(".drv") { return; }

            let short_name = &name[DRIVER_DIR_PREFIX.len()..];
            let caps = preset_for_driver(short_name);

            let no_perms = !caps.can_io_port && !caps.can_irq_register
                && !caps.can_draw && !caps.can_print;
            if no_perms {
                serial::serial_write_str("DRIVER: DENIED (no capability preset) -> ");
                serial::serial_write_str(short_name);
                serial::serial_write_str("\r\n");
                denied += 1;
                return;
            }

            match load_driver(short_name, data, caps) {
                Ok(pid) => {
                    serial::serial_write_str("DRIVER: loaded -> ");
                    serial::serial_write_str(short_name);
                    serial::serial_write_str(" (pid ");
                    write_u32_serial(pid);
                    serial::serial_write_str(")\r\n");
                    loaded += 1;
                }
                Err(e) => {
                    serial::serial_write_str("DRIVER: FAILED -> ");
                    serial::serial_write_str(short_name);
                    serial::serial_write_str(" (");
                    serial::serial_write_str(e);
                    serial::serial_write_str(")\r\n");
                    denied += 1;
                }
            }
        });
    }

    serial::serial_write_str("DRIVER: autoload complete\r\n");
}

fn write_u32_serial(mut n: u32) {
    if n == 0 { serial::serial_write_str("0"); return; }
    let mut buf = [0u8; 10];
    let mut i = 0;
    while n > 0 {
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
        i += 1;
    }
    let mut out = [0u8; 10];
    for j in 0..i { out[j] = buf[i - 1 - j]; }
    if let Ok(s) = core::str::from_utf8(&out[..i]) {
        serial::serial_write_str(s);
    }
}