#![no_std]
#![no_main]

mod boot;
mod gop;
mod font;
mod bmp;
mod initrd;
mod console;
mod cpu;
mod serial;
mod idt;
mod mouse;      // <-- THÊM
mod timer;
mod keyboard;
mod shell;
mod user;
mod graphics;
mod ybc;          // THÊM
mod ybc_vm;       // THÊM
mod process;      // THÊM
mod abp;          // THÊM
mod syscall;      // THÊM

use core::panic::PanicInfo;
use core::fmt::Write;
use gop::{Color, GraphicsOutput};
use console::CONSOLE;
use graphics::gfx;

static mut FONT_ATLAS_BUFFER: [u32; 256 * 256] = [0; 256 * 256];
static mut CMD_BUF: [u8; 256] = [0; 256];
static mut CMD_LEN: usize = 0;
static mut LAST_DISPLAYED_SEC: u64 = u64::MAX;

fn print_prompt() {
    let current_user = user::get_current_user();
    let symbol = match current_user.role {
        user::UserRole::Root => "#",
        user::UserRole::RegularUser => "$",
    };
    print!("{}@openYanase: {} ", current_user.username, symbol);
}

#[no_mangle]
pub extern "C" fn kmain(magic: u64, mb_info_ptr: u64) -> ! {
    serial::serial_init();
    serial::serial_write_str("INFO: Serial initialized\r\n");

    if magic != 0x36d76289 {
        serial::serial_write_str("ERROR: Invalid multiboot magic number\r\n");
        loop { unsafe { core::arch::asm!("hlt"); } }
    }

    let mut display = match unsafe { GraphicsOutput::from_multiboot(mb_info_ptr) } {
        Some(gop) => gop,
        None => {
            serial::serial_write_str("ERROR: Failed to get GOP framebuffer\r\n");
            loop { unsafe { core::arch::asm!("hlt"); } }
        }
    };

    display.clear(Color::BLACK);
    serial::serial_write_str("INFO: GOP initialized successfully\r\n");

    let mut initrd_start_addr: *const u8 = core::ptr::null();
    unsafe {
        let addr = mb_info_ptr as *const u32;
        let total_size = addr.read_volatile();
        let mut current = mb_info_ptr + 8;
        let end = mb_info_ptr + total_size as u64;

        while current < end {
            let tag_ptr = current as *const u32;
            let tag_type = tag_ptr.read_volatile();
            let tag_size = tag_ptr.add(1).read_volatile();

            if tag_type == 0 { break; }

            if tag_type == 3 {
                let mod_start = (current + 8) as *const u32;
                initrd_start_addr = mod_start.read_volatile() as *const u8;
                initrd::INITRD_ADDR = initrd_start_addr;
                serial::serial_write_str("INFO: Initrd found\r\n");
                break;
            }

            current = (current + tag_size as u64 + 7) & !7;
        }
    }

    idt::init_idt();
    syscall::init_syscall();  // THÊM — set gate 0x80 DPL=3
    timer::init_timer(1000);
    mouse::init_mouse();          // <-- THÊM
    keyboard::init_keyboard();
    idt::enable_interrupts();
    serial::serial_write_str("INFO: Interrupts enabled\r\n");

    // --- LOAD SPLASH ---
    let mut splash_loaded = false;
    if !initrd_start_addr.is_null() {
        let splash_names = ["splash.bmp", "bg.bmp", "wallpaper.bmp"];
        for &name in splash_names.iter() {
            if let Some(bmp_bytes) = unsafe { initrd::find_file_in_tar(initrd_start_addr, name) } {
                bmp::draw_bmp_fullscreen(&mut display, bmp_bytes);
                splash_loaded = true;
                serial::serial_write_str("INFO: Loaded splash\r\n");
                break;
            }
        }
    }

    if splash_loaded {
        timer::sleep(945);
    }

    // --- INIT CONSOLE ---
    unsafe {
        CONSOLE.init(
            display.raw_addr(),
            display.width(),
            display.height(),
            display.pitch(),
            core::ptr::null(),
            0,
            0,
        );
    }

    // --- LOAD FONT ---
    let mut font_loaded = false;
    if !initrd_start_addr.is_null() {
        serial::serial_write_str("INFO: Searching for font.psf...\r\n");
        if let Some(font_bytes) = unsafe { initrd::find_file_in_tar(initrd_start_addr, "font.psf") } {
            serial::serial_write_str("INFO: font.psf found, baking...\r\n");
            unsafe {
                let (tex_w, tex_h) = crate::graphics::font::bake_psf_to_texture(
                    font_bytes,
                    &mut FONT_ATLAS_BUFFER,
                    Color::WHITE,
                );

                if tex_w > 0 && tex_h > 0 {
                    CONSOLE.set_font(FONT_ATLAS_BUFFER.as_ptr(), tex_w, tex_h);
                    font_loaded = true;
                    serial::serial_write_str("INFO: Font loaded successfully\r\n");
                } else {
                    serial::serial_write_str("WARN: Failed to bake font\r\n");
                }
            }
        } else {
            serial::serial_write_str("WARN: font.psf not found\r\n");
        }
    }

    // Nếu font chưa load được, tạo font giả
    if !font_loaded {
        serial::serial_write_str("INFO: Creating fallback font\r\n");
        unsafe {
            create_fallback_font(&mut FONT_ATLAS_BUFFER);
            CONSOLE.set_font(FONT_ATLAS_BUFFER.as_ptr(), 128, 256);
        }
    }

    // --- INIT GRAPHICS SUBSYSTEM ---
    unsafe {
        gfx::init_graphics(
            display.raw_addr(),
            display.width(),
            display.height(),
            display.pitch(),
            CONSOLE.tex,
            CONSOLE.tex_w,
            CONSOLE.tex_h,
        );
    }

    println!("========================================");
    println!("  openYanase Kernel v2.0.0");
    println!("  UEFI 64-bit / Long Mode Active!");
    println!("========================================");
    println!("kernel: Graphics subsystem initialized");
    println!("kernel: System boot completed successfully.");
    println!("----------------------------------------");
    println!("");
    println!("  TTY Mode:  Press F1 to enter Graphics Mode");
    println!("  GFX Mode:  Press F2 to return to TTY");
    println!("");
    println!("  Commands: gfx   - Enter graphics mode");
    println!("            tty   - Return to console mode");
    println!("            help  - Show available commands");
    println!("");
    println!("  Key bindings:");
    println!("    Up/Down    - Command history");
    println!("    Tab        - Command completion");
    println!("    Page Up/Dn - Scroll console");
    println!("");
    println!("----------------------------------------");
    println!("VTTY Console & Keyboard Input Ready.");

    print_prompt();
    unsafe { CONSOLE.show_cursor(); }

    // --- MAIN LOOP ---
    let mut last_blink_tick = timer::get_ticks();
    let mut last_gfx_update = timer::get_ticks();

    loop {
        let current_tick = timer::get_ticks();

        if console::get_display_mode() == console::DisplayMode::Console {
            if current_tick.wrapping_sub(last_blink_tick) >= 500 {
                unsafe {
                    CONSOLE.toggle_cursor();
                }
                last_blink_tick = current_tick;
            }
        }

        if let Some(ch) = keyboard::pop_char() {
            unsafe {
                match ch {
                    '1' => {
                        if console::get_display_mode() == console::DisplayMode::Console {
                            serial::serial_write_str("INPUT: F1 - Entering Graphics Mode\r\n");
                            CONSOLE.hide_cursor();
                            gfx::enter_graphics_mode();
                            continue;
                        }
                    }
                    '2' => {
                        if console::get_display_mode() == console::DisplayMode::Graphics {
                            serial::serial_write_str("INPUT: F2 - Returning to Console Mode\r\n");
                            gfx::exit_graphics_mode();
                            print_prompt();
                            CONSOLE.show_cursor();
                            continue;
                        }
                    }
                    '\x1B' => {
                        if console::get_display_mode() == console::DisplayMode::Graphics {
                            serial::serial_write_str("INPUT: ESC - Returning to console\r\n");
                            gfx::exit_graphics_mode();
                            print_prompt();
                            CONSOLE.show_cursor();
                            continue;
                        }
                    }
                    _ => {}
                }

                if console::get_display_mode() == console::DisplayMode::Graphics {
                    continue;
                }

                CONSOLE.hide_cursor();

                match ch {
                    '\n' => {
                        println!();
                        if CMD_LEN > 0 {
                            if let Ok(cmd_str) = core::str::from_utf8(&CMD_BUF[..CMD_LEN]) {
                                let cmd = cmd_str.trim();
                                if cmd == "gfx" {
                                    serial::serial_write_str("INPUT: Entering graphics mode via command\r\n");
                                    CONSOLE.hide_cursor();
                                    gfx::enter_graphics_mode();
                                    CMD_LEN = 0;
                                    continue;
                                } else if cmd == "tty" {
                                    if console::get_display_mode() == console::DisplayMode::Graphics {
                                        serial::serial_write_str("INPUT: Returning to console via command\r\n");
                                        gfx::exit_graphics_mode();
                                        print_prompt();
                                        CONSOLE.show_cursor();
                                        CMD_LEN = 0;
                                        continue;
                                    }
                                } else {
                                    shell::execute(cmd);
                                }
                            }
                            CMD_LEN = 0;
                        }
                        print_prompt();
                    }
                    '\x08' => {
                        if CMD_LEN > 0 {
                            CMD_LEN -= 1;
                            CONSOLE.backspace();
                        }
                    }
                    '\t' => {
                        if CMD_LEN > 0 {
                            if let Ok(cmd_str) = core::str::from_utf8(&CMD_BUF[..CMD_LEN]) {
                                if let Some(completed) = shell::complete_command(cmd_str.trim()) {
                                    for _ in 0..CMD_LEN {
                                        CONSOLE.backspace();
                                    }
                                    CMD_LEN = 0;
                                    for c in completed.chars() {
                                        if CMD_LEN < CMD_BUF.len() {
                                            CMD_BUF[CMD_LEN] = c as u8;
                                            CMD_LEN += 1;
                                            CONSOLE.write_char(c);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {
                        if CMD_LEN < CMD_BUF.len() {
                            CMD_BUF[CMD_LEN] = ch as u8;
                            CMD_LEN += 1;
                            CONSOLE.write_char(ch);
                        }
                    }
                }
                CONSOLE.show_cursor();
                last_blink_tick = current_tick;
            }
        }

        if console::get_display_mode() == console::DisplayMode::Graphics {
            if current_tick.wrapping_sub(last_gfx_update) >= 16 {
                let cur_sec = current_tick / 1000;
                unsafe {
                    if LAST_DISPLAYED_SEC != cur_sec {
                        LAST_DISPLAYED_SEC = cur_sec;
                        gfx::mark_dirty();
                    }
                }
                gfx::draw_graphics_demo(); // hàm này không phải unsafe fn -> bỏ unsafe bọc ngoài
                last_gfx_update = current_tick;
            }
        }
        unsafe { core::arch::asm!("hlt", options(nomem, nostack)); }
    }
}

unsafe fn create_fallback_font(buffer: &mut [u32]) {
    let tex_w = 128;
    let tex_h = 256;
    
    for i in 0..(tex_w * tex_h) as usize {
        buffer[i] = 0x00000000;
    }
    
    // Tạo font đơn giản 8x16
    for ch in 32..127 {
        let char_x = ((ch % 16) * 8) as u32;
        let char_y = ((ch / 16) * 16) as u32;
        
        for y in 0..16 {
            for x in 0..8 {
                let idx = ((char_y + y) * tex_w + (char_x + x)) as usize;
                if idx < buffer.len() {
                    if y == 0 || y == 15 || x == 0 || x == 7 {
                        buffer[idx] = 0xFFFFFFFF;
                    } else if y > 2 && y < 14 && x > 1 && x < 6 {
                        buffer[idx] = 0xFFFFFFFF;
                    }
                }
            }
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let mut panic_writer = PanicWriter;
    let _ = write!(&mut panic_writer, "\r\n[KERNEL PANIC]\r\n");
    let _ = write!(&mut panic_writer, "{}\r\n", info.message());
    if let Some(location) = info.location() {
        let _ = write!(&mut panic_writer, "File: {}:{}\r\n", location.file(), location.line());
    }
    loop {
        unsafe { core::arch::asm!("hlt"); }
    }
}

struct PanicWriter;

impl core::fmt::Write for PanicWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        serial::serial_write_str(s);
        Ok(())
    }
}