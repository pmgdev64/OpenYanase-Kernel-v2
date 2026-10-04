// src/init.rs
// Init script: chạy các lệnh shell trong globalsys/init.rc (initrd) lúc boot.
//
// Cú pháp init.rc (mỗi dòng một lệnh):
//   # comment          -> bỏ qua (chỉ comment nguyên dòng)
//   (dòng trống)       -> bỏ qua
//   sleep <ms>         -> chờ (tối đa 10000 ms)
//   <lệnh khác>        -> đưa nguyên dòng cho shell::execute
//                         (echo, ls, system, tên app .abp, ...)
use crate::{initrd, serial, shell, timer};

const INIT_PATH: &str = "globalsys/init.rc";
const MAX_LINES: usize = 64;
const MAX_SLEEP_MS: u64 = 10_000;

pub fn run() {
    let tar = unsafe { initrd::INITRD_ADDR };
    if tar.is_null() {
        serial::serial_write_str("INIT: initrd not loaded, skipping\r\n");
        return;
    }

    let data = match unsafe { initrd::find_file_in_tar(tar, INIT_PATH) } {
        Some(d) => d,
        None => {
            serial::serial_write_str("INIT: no globalsys/init.rc, skipping\r\n");
            return;
        }
    };

    let text = match core::str::from_utf8(data) {
        Ok(t) => t,
        Err(_) => {
            serial::serial_write_str("INIT: init.rc is not valid UTF-8, skipping\r\n");
            return;
        }
    };

    serial::serial_write_str("INIT: running init.rc\r\n");

    let mut count = 0usize;
    for line in text.lines() {
        let cmd = line.trim();
        if cmd.is_empty() || cmd.starts_with('#') {
            continue;
        }

        count += 1;
        if count > MAX_LINES {
            serial::serial_write_str("INIT: too many lines, stopping\r\n");
            break;
        }

        serial::serial_write_str("INIT: > ");
        serial::serial_write_str(cmd);
        serial::serial_write_str("\r\n");

        if let Some(arg) = cmd.strip_prefix("sleep ") {
            match arg.trim().parse::<u64>() {
                Ok(ms) => timer::sleep(ms.min(MAX_SLEEP_MS)),
                Err(_) => serial::serial_write_str("INIT: bad sleep value, ignored\r\n"),
            }
            continue;
        }

        shell::execute(cmd);
    }

    // Phím gõ trong lúc init chạy không được lọt vào prompt đầu tiên.
    crate::keyboard::flush_keys();

    serial::serial_write_str("INIT: done\r\n");
}