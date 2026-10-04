// src/shell.rs
use crate::console::CONSOLE;
use crate::memory;
use crate::initrd;

const HISTORY_SIZE: usize = 32;
const MAX_CMD_LEN: usize = 64;

static mut HISTORY: [[u8; MAX_CMD_LEN]; HISTORY_SIZE] = [[0; MAX_CMD_LEN]; HISTORY_SIZE];
static mut HISTORY_HEAD: usize = 0;
static mut HISTORY_COUNT: usize = 0;
static mut HISTORY_POS: usize = 0;

pub static COMMANDS: &[&str] = &[
    "help", "clear", "cls", "info", "gfx", "tty",
    "echo", "version", "history", "meminfo", "reboot", "shutdown",
    "cd", "ls", "cat", "pwd",
    "drivers", "lsdrv", "drvstate", "unloaddrv",
    "shark", "gura", "mumei", "hooman",
    "calli", "deadbeat", "ina", "takodachi",
    "kiara", "kfp", "hololive", "myth", "council",
];

pub fn execute(input: &str) {
    let cmd = input.trim();
    if cmd.is_empty() {
        return;
    }

    unsafe {
        let bytes = cmd.as_bytes();
        let len = bytes.len().min(MAX_CMD_LEN - 1);
        let idx = HISTORY_HEAD % HISTORY_SIZE;
        HISTORY[idx][..len].copy_from_slice(&bytes[..len]);
        HISTORY[idx][len] = 0;

        HISTORY_HEAD += 1;
        if HISTORY_COUNT < HISTORY_SIZE {
            HISTORY_COUNT += 1;
        }
        HISTORY_POS = HISTORY_HEAD;
    }

    let mut parts: [&str; 16] = [""; 16];
    let mut part_count = 0;

    for part in cmd.split_whitespace() {
        if part_count < 16 {
            parts[part_count] = part;
            part_count += 1;
        } else {
            break;
        }
    }

    if part_count == 0 {
        return;
    }

    let command = parts[0];
    let args = &parts[1..part_count];

    match command {
        "help" => {
            crate::println!("=== openYanase Shell v2.0 ===");
            crate::println!("");
            crate::println!("Commands:");
            crate::println!("  help           - Show this help");
            crate::println!("  clear / cls    - Clear the screen");
            crate::println!("  info           - Show system information");
            crate::println!("  gfx            - Switch to Graphics Mode");
            crate::println!("  tty            - Switch to Console Mode");
            crate::println!("  echo <text>    - Echo text");
            crate::println!("  version        - Show kernel version");
            crate::println!("  reboot         - Reboot the system");
            crate::println!("  shutdown       - Shutdown the system");
            crate::println!("  history        - Show command history");
            crate::println!("  meminfo        - Show memory information");
            crate::println!("  cd <path>      - Change directory");
            crate::println!("  ls [path]      - List directory contents");
            crate::println!("  cat <file>     - Display file contents");
            crate::println!("  pwd            - Print working directory");
            crate::println!("");
            crate::println!("Driver commands:");
            crate::println!("  drivers        - List loaded kernel drivers");
            crate::println!("  lsdrv          - Alias for drivers");
            crate::println!("  drvstate <pid> - Show driver state by PID");
            crate::println!("  unloaddrv <pid>- Unload a driver by PID");
            crate::println!("");
            crate::println!("  <appname>      - Run application (with or without .abp)");
            crate::println!("");
            crate::println!("Key bindings:");
            crate::println!("  Up/Down        - Command history");
            crate::println!("  Tab            - Command completion");
            crate::println!("  Page Up/Dn     - Scroll console");
            crate::println!("  F1             - Switch to Graphics Mode");
            crate::println!("  F2             - Switch to Console Mode");
        }
        "clear" | "cls" => {
            unsafe { CONSOLE.clear(); }
        }
        "gfx" => {
            unsafe {
                CONSOLE.hide_cursor();
                crate::graphics::gfx::set_demo_mode(true);
                crate::graphics::gfx::enter_graphics_mode();
            }
        }
        "tty" => {
            unsafe {
                crate::graphics::gfx::exit_graphics_mode();
            }
        }
        "info" => {
            let w = unsafe { CONSOLE.fb_width };
            let h = unsafe { CONSOLE.fb_height };
            crate::println!("=== System Information ===");
            crate::println!("Resolution: {}x{}", w, h);
            crate::println!("Architecture: x86_64 UEFI");
            crate::println!("Kernel: OpenYanase v2.0.0");
            crate::println!("Timer: PIT 1000Hz");
            crate::println!("Interrupts: Enabled");
        }
        "echo" => {
            if args.is_empty() || args[0].is_empty() {
                crate::println!();
            } else {
                let mut first = true;
                for arg in args {
                    if !arg.is_empty() {
                        if !first {
                            crate::print!(" ");
                        }
                        crate::print!("{}", arg);
                        first = false;
                    }
                }
                crate::println!();
            }
        }
        "version" => {
            crate::println!("OpenYanase Kernel v2.0.0");
            crate::println!("UEFI 64-bit / Long Mode Active");
        }
        "meminfo" => {
            unsafe {
                let tag = memory::find_memory_tag(crate::KMBDATA);
                if !tag.is_null() {
                    let stats = memory::calculate_memory_stats(tag);

                    fn fmt_mb(buf: &mut [u8; 24], bytes: u64) -> &str {
                        let hundredths = (bytes.saturating_mul(100)) / (1024 * 1024);
                        let whole = hundredths / 100;
                        let frac  = hundredths % 100;

                        let mut wbuf = [0u8; 20];
                        let mut wlen = 0;
                        let mut w = whole;
                        if w == 0 {
                            wbuf[0] = b'0';
                            wlen = 1;
                        } else {
                            while w > 0 {
                                wbuf[wlen] = b'0' + (w % 10) as u8;
                                wlen += 1;
                                w /= 10;
                            }
                        }

                        let mut pos = 0;
                        for i in 0..wlen {
                            buf[pos] = wbuf[wlen - 1 - i];
                            pos += 1;
                        }
                        buf[pos] = b'.'; pos += 1;
                        buf[pos] = b'0' + (frac / 10) as u8; pos += 1;
                        buf[pos] = b'0' + (frac % 10) as u8; pos += 1;
                        buf[pos] = b' '; pos += 1;
                        buf[pos] = b'M'; pos += 1;
                        buf[pos] = b'B'; pos += 1;

                        unsafe { core::str::from_utf8_unchecked(&buf[..pos]) }
                    }

                    let mut b1 = [0u8; 24];
                    let mut b2 = [0u8; 24];
                    let mut b3 = [0u8; 24];
                    let mut b4 = [0u8; 24];
                    let mut b5 = [0u8; 24];

                    crate::println!("=== Memory Information ===");
                    crate::println!("Total RAM:      {}", fmt_mb(&mut b1, stats.total_ram));
                    crate::println!("Usable RAM:     {}", fmt_mb(&mut b2, stats.usable_ram));
                    crate::println!("Reserved:       {}", fmt_mb(&mut b3, stats.reserved_ram));
                    crate::println!("Kernel start:   {:#x}", stats.kernel_start);
                    crate::println!("Kernel end:     {:#x}", stats.kernel_end);
                    crate::println!("Kernel Used:    {}", fmt_mb(&mut b4, stats.kernel_used));
                    crate::println!("Free RAM:       {}", fmt_mb(&mut b5, stats.free_ram));
                    crate::println!("Available:      {}%", stats.mem_available_percent);
                } else {
                    crate::println!("No memory map available");
                }
            }
        }
        "drivers" | "lsdrv" => {
            crate::driver::list_drivers();
        }
        "drvstate" => {
            if args.is_empty() {
                crate::println!("Usage: drvstate <pid>");
                return;
            }
            match args[0].parse::<u32>() {
                Ok(pid) => {
                    if let Some(info) = crate::driver::get_driver_info(pid) {
                        crate::println!("Driver PID: {}", pid);
                        crate::println!("  Name:   {}", info.name_str());
                        crate::println!("  Type:   {}", info.driver_type_enum().as_str());
                        crate::println!("  State:  {}", info.state_enum().as_str());
                        crate::println!("  Pri:    {}", info.priority);
                        crate::println!("  Events: {} (queued)", info.event_count);
                    } else {
                        crate::println!("Driver PID {} not found", pid);
                    }
                }
                Err(_) => crate::println!("Invalid PID: {}", args[0]),
            }
        }
        "unloaddrv" => {
            if args.is_empty() {
                crate::println!("Usage: unloaddrv <pid>");
                return;
            }
            match args[0].parse::<u32>() {
                Ok(pid) => {
                    unsafe {
                        if crate::driver::DRIVER_MANAGER.unload_driver(pid) {
                            crate::println!("Driver PID {} unloaded", pid);
                        } else {
                            crate::println!("Failed to unload driver PID {}", pid);
                        }
                    }
                }
                Err(_) => crate::println!("Invalid PID: {}", args[0]),
            }
        }
        "cd" => {
            if args.is_empty() {
                crate::change_directory("/");
            } else {
                crate::change_directory(args[0]);
            }
        }
        "ls" => {
            let path = if args.is_empty() {
                crate::get_current_path()
            } else {
                args[0]
            };

            if let Some(vfs) = crate::vfs::get_vfs() {
                let mut count = 0;

                let vfs_path = if path == "/" || path.is_empty() {
                    "/"
                } else if path.starts_with('/') {
                    path
                } else {
                    path
                };

                let display_path = if vfs_path == "/" { "/" } else { vfs_path };
                crate::println!("Contents of: {}", display_path);

                vfs.list_directory(vfs_path, &mut |entry| {
                    count += 1;
                    let is_dir = entry.ends_with('/');
                    if is_dir {
                        crate::println!("  [DIR]  {}", entry);
                    } else {
                        crate::println!("  [FILE] {}", entry);
                    }
                });

                if count == 0 {
                    crate::println!("  (empty)");
                }
            } else {
                crate::println!("VFS not initialized");
            }
        }
        "cat" => {
            if args.is_empty() {
                crate::println!("Usage: cat <file>");
                return;
            }

            let path = args[0];

            let full_path = if path.starts_with('/') {
                path
            } else {
                path
            };

            if let Some(vfs) = crate::vfs::get_vfs() {
                if let Some(data) = vfs.read_file(full_path) {
                    if let Ok(s) = core::str::from_utf8(data) {
                        crate::println!("{}", s);
                    } else {
                        crate::println!("<binary data>");
                    }
                } else {
                    crate::println!("File not found: {}", path);
                }
            } else {
                crate::println!("VFS not initialized");
            }
        }
        "pwd" => {
            let path = crate::get_current_path();
            crate::println!("{}", path);
        }
        "history" => {
            unsafe {
                crate::println!("=== Command History ===");
                let start = if HISTORY_COUNT > HISTORY_SIZE {
                    HISTORY_HEAD - HISTORY_SIZE
                } else {
                    0
                };
                for i in 0..HISTORY_COUNT.min(HISTORY_SIZE) {
                    let idx = (start + i) % HISTORY_SIZE;
                    let mut len = 0;
                    while len < MAX_CMD_LEN && HISTORY[idx][len] != 0 {
                        len += 1;
                    }
                    if let Ok(s) = core::str::from_utf8(&HISTORY[idx][..len]) {
                        crate::println!("  {}: {}", i, s);
                    }
                }
            }
        }
        "shark" | "gura" => {
            crate::println!("");
            crate::println!("         /\\_/\\");
            crate::println!("    ____/ o o \\");
            crate::println!("  /~____  =o= /");
            crate::println!(" (______)__m_m)");
            crate::println!("");
            crate::println!("    WAH! Gawr Gura!");
            crate::println!("    A! A! A!");
            crate::println!("");
        }
        "mumei" | "hooman" => {
            crate::println!("");
            crate::println!("       .---.");
            crate::println!("      /     \\");
            crate::println!("     |  o o  |");
            crate::println!("     |   ^   |");
            crate::println!("     |  '-'  |");
            crate::println!("      \\     /");
            crate::println!("       '---'");
            crate::println!("");
            crate::println!("    Hoo~ Nanashi Mumei!");
            crate::println!("    I'm not a criminal... probably!");
            crate::println!("");
        }
        "calli" | "deadbeat" => {
            crate::println!("");
            crate::println!("         .--.");
            crate::println!("        /  _  \\");
            crate::println!("       /  |_|  \\");
            crate::println!("      /   /_\\   \\");
            crate::println!("     /  __|_|__  \\");
            crate::println!("    /  /       \\  \\");
            crate::println!("   /  /         \\  \\");
            crate::println!("  /  /           \\  \\");
            crate::println!(" /  /             \\  \\");
            crate::println!("/__/               \\__\\");
            crate::println!("");
            crate::println!("    Calliope Mori!");
            crate::println!("    RAP! RAP! RAP!");
            crate::println!("");
        }
        "ina" | "takodachi" => {
            crate::println!("");
            crate::println!("      .---.");
            crate::println!("     /     \\");
            crate::println!("    |  o o  |");
            crate::println!("    |   ^   |");
            crate::println!("    |  '-'  |");
            crate::println!("     \\  ~  /");
            crate::println!("      '---'");
            crate::println!("");
            crate::println!("    Ninomae Ina'nis!");
            crate::println!("    Wah! WAH!");
            crate::println!("");
        }
        "kiara" | "kfp" => {
            crate::println!("");
            crate::println!("      .--.");
            crate::println!("     /    \\");
            crate::println!("    |  ##  |");
            crate::println!("    |  ##  |");
            crate::println!("    |  ##  |");
            crate::println!("     \\  v  /");
            crate::println!("      '--'");
            crate::println!("");
            crate::println!("    Takanashi Kiara!");
            crate::println!("    KFP! KFP! KFP!");
            crate::println!("");
        }
        "hololive" => {
            crate::println!("");
            crate::println!("    +---------------------------------------+");
            crate::println!("    |     Hololive Production EN          |");
            crate::println!("    +---------------------------------------+");
            crate::println!("");
            crate::println!("    [S] Gawr Gura        - Chumbud");
            crate::println!("    [O] Nanashi Mumei    - Hooman");
            crate::println!("    [D] Calliope Mori    - Deadbeat");
            crate::println!("    [I] Ninomae Ina'nis  - Takodachi");
            crate::println!("    [F] Takanashi Kiara  - KFP");
            crate::println!("    [P] Tsukumo Sana     - Sana");
            crate::println!("    [R] Hakos Baelz      - Bae");
            crate::println!("    [C] Ouro Kronii      - Kronie");
            crate::println!("    [N] Ceres Fauna      - Sapling");
            crate::println!("");
            crate::println!("    [H] IRyS            - Nephilim");
            crate::println!("    [L] La+ Darknesss   - Laplus");
            crate::println!("    [E] Regis Altare    - Stargazer");
            crate::println!("");
        }
        "myth" => {
            crate::println!("");
            crate::println!("    +---------------------------------------+");
            crate::println!("    |     Hololive EN - Myth              |");
            crate::println!("    +---------------------------------------+");
            crate::println!("");
            crate::println!("    [S] Gawr Gura        - Chumbud");
            crate::println!("    [D] Calliope Mori    - Deadbeat");
            crate::println!("    [I] Ninomae Ina'nis  - Takodachi");
            crate::println!("    [F] Takanashi Kiara  - KFP");
            crate::println!("    [P] Tsukumo Sana     - Sana");
            crate::println!("");
        }
        "council" => {
            crate::println!("");
            crate::println!("    +---------------------------------------+");
            crate::println!("    |     Hololive EN - Council           |");
            crate::println!("    +---------------------------------------+");
            crate::println!("");
            crate::println!("    [O] Nanashi Mumei    - Hooman");
            crate::println!("    [R] Hakos Baelz      - Bae");
            crate::println!("    [C] Ouro Kronii      - Kronie");
            crate::println!("    [N] Ceres Fauna      - Sapling");
            crate::println!("");
        }
        "reboot" => {
            crate::println!("kernel: Initiating system reboot...");
            crate::acpi::reboot();
        }
        "shutdown" => {
            crate::println!("kernel: Shutting down system...");
            crate::acpi::shutdown();
        }
        _ => {
            let initrd_addr = unsafe { crate::initrd::INITRD_ADDR };
            if initrd_addr.is_null() {
                crate::println!("Error: initrd not loaded, cannot run packages");
                return;
            }

            let clean_name = command.trim_start_matches("./").trim_start_matches('/');
            if clean_name.is_empty() {
                crate::println!("Unknown command: '{}'. Type 'help' for available commands.", command);
                return;
            }

            let abp_name = if clean_name.ends_with(".abp") {
                clean_name
            } else {
                static mut NAME_BUF: [u8; 128] = [0; 128];
                unsafe {
                    let bytes = clean_name.as_bytes();
                    let len = bytes.len().min(120);
                    NAME_BUF[..len].copy_from_slice(&bytes[..len]);
                    NAME_BUF[len..len+4].copy_from_slice(b".abp");
                    match core::str::from_utf8(&NAME_BUF[..len+4]) {
                        Ok(s) => s,
                        Err(_) => {
                            crate::println!("Unknown command: '{}'. Type 'help' for available commands.", command);
                            return;
                        }
                    }
                }
            };

            if unsafe { initrd::find_file_in_tar(initrd_addr, abp_name).is_some() } {
                run_abp_command(abp_name);
                return;
            }

            static mut PATH_BUF: [u8; 256] = [0; 256];
            unsafe {
                let prefix = b"globalsys/";
                let name_bytes = abp_name.as_bytes();
                let total_len = prefix.len() + name_bytes.len();
                if total_len < PATH_BUF.len() {
                    PATH_BUF[..prefix.len()].copy_from_slice(prefix);
                    PATH_BUF[prefix.len()..total_len].copy_from_slice(name_bytes);
                    if let Ok(full_path) = core::str::from_utf8(&PATH_BUF[..total_len]) {
                        if initrd::find_file_in_tar(initrd_addr, full_path).is_some() {
                            run_abp_command(full_path);
                            return;
                        }
                    }
                }
            }

            unsafe {
                let prefix = b"apps/";
                let name_bytes = abp_name.as_bytes();
                let total_len = prefix.len() + name_bytes.len();
                if total_len < PATH_BUF.len() {
                    PATH_BUF[..prefix.len()].copy_from_slice(prefix);
                    PATH_BUF[prefix.len()..total_len].copy_from_slice(name_bytes);
                    if let Ok(full_path) = core::str::from_utf8(&PATH_BUF[..total_len]) {
                        if initrd::find_file_in_tar(initrd_addr, full_path).is_some() {
                            run_abp_command(full_path);
                            return;
                        }
                    }
                }
            }

            unsafe {
                let prefix = b"globalsys/apps/";
                let name_bytes = abp_name.as_bytes();
                let total_len = prefix.len() + name_bytes.len();
                if total_len < PATH_BUF.len() {
                    PATH_BUF[..prefix.len()].copy_from_slice(prefix);
                    PATH_BUF[prefix.len()..total_len].copy_from_slice(name_bytes);
                    if let Ok(full_path) = core::str::from_utf8(&PATH_BUF[..total_len]) {
                        if initrd::find_file_in_tar(initrd_addr, full_path).is_some() {
                            run_abp_command(full_path);
                            return;
                        }
                    }
                }
            }

            if let Some(matched) = complete_command(command) {
                crate::println!("{}", matched);
            } else {
                crate::println!("Unknown command: '{}'. Type 'help' for available commands.", command);
            }
        }
    }
}

pub fn complete_command(prefix: &str) -> Option<&'static str> {
    let mut matches: [&str; 16] = [""; 16];
    let mut count = 0;

    for &cmd in COMMANDS.iter() {
        if cmd.starts_with(prefix) && count < 16 {
            matches[count] = cmd;
            count += 1;
        }
    }

    if count == 1 {
        Some(matches[0])
    } else if count > 1 {
        crate::println!("");
        for i in 0..count {
            crate::println!("  {}", matches[i]);
        }
        None
    } else {
        None
    }
}

pub unsafe fn get_history_up() -> Option<&'static str> {
    if HISTORY_COUNT == 0 {
        return None;
    }
    if HISTORY_POS > 0 {
        HISTORY_POS -= 1;
    }
    let idx = HISTORY_POS % HISTORY_SIZE;
    let mut len = 0;
    while len < MAX_CMD_LEN && HISTORY[idx][len] != 0 {
        len += 1;
    }
    if len > 0 {
        core::str::from_utf8(&HISTORY[idx][..len]).ok()
    } else {
        None
    }
}

pub unsafe fn get_history_down() -> Option<&'static str> {
    if HISTORY_POS < HISTORY_HEAD - 1 {
        HISTORY_POS += 1;
        let idx = HISTORY_POS % HISTORY_SIZE;
        let mut len = 0;
        while len < MAX_CMD_LEN && HISTORY[idx][len] != 0 {
            len += 1;
        }
        if len > 0 {
            core::str::from_utf8(&HISTORY[idx][..len]).ok()
        } else {
            None
        }
    } else {
        HISTORY_POS = HISTORY_HEAD;
        None
    }
}

fn run_abp_command(filename: &str) {
    let initrd_addr = unsafe { crate::initrd::INITRD_ADDR };
    if initrd_addr.is_null() {
        crate::println!("Error: initrd not loaded, cannot run packages");
        return;
    }

    let clean_name = filename.trim_start_matches("./").trim_start_matches('/');

    if let Some(_) = unsafe { crate::initrd::find_file_in_tar(initrd_addr, clean_name) } {
        if let Err(e) = crate::abp::run_abp_file(initrd_addr, clean_name) {
            crate::println!("Error running {}: {}", clean_name, e);
        }
        return;
    }

    crate::println!("Error: package '{}' not found", filename);
}