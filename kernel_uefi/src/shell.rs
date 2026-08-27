// src/shell.rs
use crate::console::CONSOLE;

const HISTORY_SIZE: usize = 32;
const MAX_CMD_LEN: usize = 64;

static mut HISTORY: [[u8; MAX_CMD_LEN]; HISTORY_SIZE] = [[0; MAX_CMD_LEN]; HISTORY_SIZE];
static mut HISTORY_HEAD: usize = 0;
static mut HISTORY_COUNT: usize = 0;
static mut HISTORY_POS: usize = 0;

pub static COMMANDS: &[&str] = &[
    "help", "clear", "cls", "info", "gfx", "tty", 
    "echo", "version", "history", "reboot",
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
            crate::println!("  history        - Show command history");
            crate::println!("");
            crate::println!("Key bindings:");
            crate::println!("  Up/Down        - Command history");
            crate::println!("  Tab            - Command completion");
            crate::println!("  Page Up/Down   - Scroll console");
            crate::println!("  F1             - Switch to Graphics Mode");
            crate::println!("  F2             - Switch to Console Mode");
        }
        "clear" | "cls" => {
            unsafe { CONSOLE.clear(); }
        }
        "gfx" => {
            unsafe {
                CONSOLE.hide_cursor();
                crate::graphics::gfx::enter_graphics_mode();
            }
        }
        "tty" => {
            unsafe {
                crate::graphics::gfx::exit_graphics_mode();
            }
        }
        "info" => {
            unsafe {
                let w = CONSOLE.fb_width;
                let h = CONSOLE.fb_height;
                crate::println!("=== System Information ===");
                crate::println!("Resolution: {}x{}", w, h);
                crate::println!("Architecture: x86_64 UEFI");
                crate::println!("Kernel: OpenYanase v2.0.0");
                crate::println!("Timer: PIT 1000Hz");
                crate::println!("Interrupts: Enabled");
            }
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
        "reboot" => {
            crate::println!("Rebooting...");
            unsafe { crate::cpu::outb(0x64, 0xFE); }
        }
        _ => {
            // Nếu gõ đúng tên file kết thúc bằng .abp, thử chạy như 1 process
            if command.ends_with(".abp") {
                run_abp_command(command);
                return;
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

    crate::println!("Launching '{}'...", filename);
    match crate::abp::run_abp_file(initrd_addr, filename) {
        Ok(()) => crate::println!("Process '{}' exited normally", filename),
        Err(e) => crate::println!("Failed to run '{}': {}", filename, e),
    }
}