// src/main.rs
#![no_std]
#![no_main]
#![allow(static_mut_refs)]

mod boot;
mod gop;
mod font;
mod bmp;
mod initrd;
mod console;
mod cpu;
mod serial;
mod idt;
mod mouse;
mod acpi;
mod timer;
mod keyboard;
mod shell;
mod init;
mod user;
mod graphics;
mod ybc;
mod ybc_vm;
mod process;
mod abp;
mod syscall;
mod driver;
mod driver_loader;
mod kvm;
mod kvm_guard;
mod memory;
mod vfs;

extern crate alloc;

use core::panic::PanicInfo;
use core::fmt::Write;
use core::alloc::{GlobalAlloc, Layout};
use core::sync::atomic::{AtomicUsize, Ordering};
use gop::{Color, GraphicsOutput};
use console::CONSOLE;
use graphics::gfx;
use vfs::RamDiskFs;

// ============================================
// SIMPLE BUMP ALLOCATOR
// ============================================
static HEAP_START: AtomicUsize = AtomicUsize::new(0);
static HEAP_END: AtomicUsize = AtomicUsize::new(0);
static HEAP_NEXT: AtomicUsize = AtomicUsize::new(0);

pub struct BumpAllocator;

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let align = layout.align();
        let size = layout.size();

        let next = HEAP_NEXT.load(Ordering::Relaxed);
        let aligned = (next + align - 1) & !(align - 1);
        let new_next = aligned + size;

        let end = HEAP_END.load(Ordering::Relaxed);
        if new_next <= end {
            HEAP_NEXT.store(new_next, Ordering::Relaxed);
            aligned as *mut u8
        } else {
            core::ptr::null_mut()
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[global_allocator]
static ALLOCATOR: BumpAllocator = BumpAllocator;

pub static mut KMBDATA: u64 = 0;
pub static mut MEM_STATS: memory::MemoryStats = memory::MemoryStats::new();

pub static mut PATH_BUF: [u8; 256] = {
    let mut b = [0u8; 256];
    b[0] = b'r'; b[1] = b'a'; b[2] = b'm'; b[3] = b'0';
    b[4] = b':'; b[5] = b'/'; b[6] = b'/';
    b
};
pub static mut PATH_LEN: usize = 7;

static mut RAMDISK_FS: RamDiskFs = RamDiskFs::new();
static mut INITRD_FS: InitrdFs = InitrdFs::new();

static mut FONT_ATLAS_BUFFER: [u32; 512 * 512] = [0; 512 * 512];

static mut FONT_TEX_W: u32 = 128;
static mut FONT_TEX_H: u32 = 256;

static mut CMD_BUF: [u8; 256] = [0; 256];
static mut CMD_LEN: usize = 0;
static mut LAST_DISPLAYED_SEC: u64 = u64::MAX;
static mut CURSOR_FORCE_TIMER: u64 = 0;

static mut HEAP: [u8; 1024 * 128] = [0; 1024 * 128];

pub fn get_current_path() -> &'static str {
    unsafe { core::str::from_utf8_unchecked(&PATH_BUF[..PATH_LEN]) }
}

pub fn set_current_path(new_path: &str) {
    unsafe {
        let bytes = new_path.as_bytes();
        let len = bytes.len().min(PATH_BUF.len());
        PATH_BUF[..len].copy_from_slice(&bytes[..len]);
        PATH_LEN = len;
    }
}

pub fn resolve_path(current: &str, input: &str, out: &mut [u8]) -> usize {
    let (cur_scheme, cur_rel) = match current.find("://") {
        Some(idx) => (&current[..idx], &current[idx + 3..]),
        None => ("ram0", current.trim_start_matches('/')),
    };

    let (target_scheme, target_input) = match input.find("://") {
        Some(idx) => (&input[..idx], &input[idx + 3..]),
        None => (cur_scheme, input),
    };

    let mut parts: [&str; 16] = [""; 16];
    let mut count = 0;

    let is_absolute = input.contains("://") || target_input.starts_with('/');

    if !is_absolute {
        for comp in cur_rel.split('/') {
            if !comp.is_empty() && comp != "." {
                if count < parts.len() {
                    parts[count] = comp;
                    count += 1;
                }
            }
        }
    }

    for comp in target_input.split('/') {
        if comp.is_empty() || comp == "." {
            continue;
        } else if comp == ".." {
            if count > 0 { count -= 1; }
        } else {
            if count < parts.len() {
                parts[count] = comp;
                count += 1;
            }
        }
    }

    let mut len = 0;
    let scheme_bytes = target_scheme.as_bytes();
    let copy_s = scheme_bytes.len().min(out.len());
    out[..copy_s].copy_from_slice(&scheme_bytes[..copy_s]);
    len += copy_s;

    if len + 3 <= out.len() {
        out[len..len + 3].copy_from_slice(b"://");
        len += 3;
    }

    for i in 0..count {
        if i > 0 && len < out.len() {
            out[len] = b'/';
            len += 1;
        }
        let bytes = parts[i].as_bytes();
        let copy_len = bytes.len().min(out.len() - len);
        out[len..len + copy_len].copy_from_slice(&bytes[..copy_len]);
        len += copy_len;
    }

    len
}

pub struct InitrdFs {
    mounted: bool,
}

impl InitrdFs {
    pub const fn new() -> Self { Self { mounted: false } }
}

impl crate::vfs::FileSystem for InitrdFs {
    fn mount(&mut self) -> bool { self.mounted = true; true }
    fn unmount(&mut self) { self.mounted = false; }

    fn read_file(&self, path: &str) -> Option<&'static [u8]> {
        let tar_start = unsafe { initrd::INITRD_ADDR };
        if tar_start.is_null() { return None; }
        let file_path = path.trim_matches('/');
        unsafe { initrd::find_file_in_tar(tar_start, file_path) }
    }

    fn list_directory(&self, path: &str, callback: &mut dyn FnMut(&str)) {
        let tar_start = unsafe { initrd::INITRD_ADDR };
        if tar_start.is_null() { return; }

        let target_dir = path.trim_matches('/');
        let mut seen: [&str; 64] = [""; 64];
        let mut seen_count = 0;

        unsafe {
            initrd::for_each_file_with_prefix(tar_start, "", |name, _tf, _data| {
                let clean_name = initrd::normalize_tar_name(name).trim_matches('/');
                if clean_name.is_empty() { return; }

                let rel = if target_dir.is_empty() {
                    clean_name
                } else if clean_name.starts_with(target_dir) {
                    let rem = &clean_name[target_dir.len()..];
                    if rem.starts_with('/') {
                        &rem[1..]
                    } else { return; }
                } else { return; };

                if rel.is_empty() { return; }

                let entry = match rel.find('/') {
                    Some(idx) => &rel[..idx],
                    None => rel,
                };

                if entry.is_empty() || entry == "." || entry == ".." { return; }

                let mut exists = false;
                for i in 0..seen_count {
                    if seen[i] == entry {
                        exists = true;
                        break;
                    }
                }

                if !exists && seen_count < seen.len() {
                    seen[seen_count] = entry;
                    seen_count += 1;
                    callback(entry);
                }
            });
        }
    }

    fn file_exists(&self, path: &str) -> bool {
        let tar_start = unsafe { initrd::INITRD_ADDR };
        if tar_start.is_null() { return false; }
        let file_path = path.trim_matches('/');
        unsafe { initrd::find_file_in_tar(tar_start, file_path).is_some() }
    }

    fn directory_exists(&self, path: &str) -> bool {
        let tar_start = unsafe { initrd::INITRD_ADDR };
        if tar_start.is_null() { return false; }

        let target = path.trim_matches('/');
        if target.is_empty() { return true; }

        let mut found = false;
        unsafe {
            initrd::for_each_file_with_prefix(tar_start, "", |name, _tf, _data| {
                let clean_name = initrd::normalize_tar_name(name).trim_matches('/');
                if clean_name == target {
                    found = true;
                } else if clean_name.starts_with(target) {
                    let rem = &clean_name[target.len()..];
                    if rem.starts_with('/') { found = true; }
                }
            });
        }
        found
    }
}

pub fn change_directory(path: &str) {
    let current = get_current_path();
    let mut buf = [0u8; 256];
    let new_len = resolve_path(current, path, &mut buf);

    if let Ok(new_path) = core::str::from_utf8(&buf[..new_len]) {
        let exists = if let Some(vfs) = crate::vfs::get_vfs() {
            vfs.directory_exists(new_path)
        } else {
            false
        };

        if exists || new_path.ends_with("://") {
            set_current_path(new_path);
        } else {
            println!("cd: {}: No such file or directory", path);
        }
    }
}

fn print_prompt() {
    let current_user = user::get_current_user();
    let symbol = match current_user.role {
        user::UserRole::Root => "#",
        user::UserRole::RegularUser => "$",
    };

    let path = get_current_path();
    print!("{}@OpenYanase:{} {} ", current_user.username, path, symbol);
}

#[no_mangle]
pub extern "C" fn kmain(magic: u64, mb_info_ptr: u64) -> ! {
    unsafe { KMBDATA = mb_info_ptr; }

    serial::serial_init();

    unsafe {
        let start = HEAP.as_mut_ptr() as usize;
        let end = start + HEAP.len();
        HEAP_START.store(start, Ordering::Relaxed);
        HEAP_END.store(end, Ordering::Relaxed);
        HEAP_NEXT.store(start, Ordering::Relaxed);
        serial::serial_write_str("INFO: Heap initialized (128KB)\r\n");
    }

    if magic != 0x36d76289 {
        serial::serial_write_str("ERROR: Invalid multiboot magic\r\n");
        loop { unsafe { core::arch::asm!("hlt"); } }
    }

    let mut display = match unsafe { GraphicsOutput::from_multiboot(mb_info_ptr) } {
        Some(gop) => gop,
        None => {
            serial::serial_write_str("ERROR: No framebuffer found\r\n");
            loop { unsafe { core::arch::asm!("hlt"); } }
        }
    };

    display.clear(Color::BLACK);

    idt::init_idt();
    syscall::init_syscall();
    timer::init_timer(1000);
    mouse::init_mouse();
    keyboard::init_keyboard();
    idt::enable_interrupts();

    let mut initrd_start_addr: *const u8 = core::ptr::null();
    let mut initrd_found = false;
    let mut mem_parsed = false;

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
                initrd_found = true;
            }

            if tag_type == 6 {
                let mem_tag = current as *const memory::Mb2MemoryMapTag;
                MEM_STATS = memory::calculate_memory_stats(mem_tag);
                mem_parsed = true;
            }

            current = (current + tag_size as u64 + 7) & !7;
        }
    }

    // ============================================
    // LOAD FONT
    // ============================================
    let mut font_loaded = false;
    if !initrd_start_addr.is_null() {
        let font_names = [
            "globalsys/systemres/fonts/default.yfn",
            "globalsys/systemres/fonts/default.psf",
            "globalsys/systemres/fonts/default.psf2",
        ];
        for &fname in font_names.iter() {
            if let Some(font_bytes) = unsafe { initrd::find_file_in_tar(initrd_start_addr, fname) } {
                unsafe {
                    let (tex_w, tex_h) = crate::graphics::font::bake_psf(
                        font_bytes,
                        &mut FONT_ATLAS_BUFFER,
                        Color::WHITE,
                    );
                    if tex_w > 0 && tex_h > 0 {
                        FONT_TEX_W = tex_w;
                        FONT_TEX_H = tex_h;
                        font_loaded = true;
                        serial::serial_write_str("MAIN: font loaded OK\r\n");
                        break;
                    } else {
                        serial::serial_write_str("MAIN: bake_psf failed, trying next\r\n");
                    }
                }
            }
        }
    }

    if !font_loaded {
        unsafe {
            let (tex_w, tex_h) = crate::graphics::font::create_default_font(&mut FONT_ATLAS_BUFFER);
            FONT_TEX_W = tex_w;
            FONT_TEX_H = tex_h;
            serial::serial_write_str("MAIN: fallback font used\r\n");
        }
    }

    // ============================================
    // DISPLAY SPLASH
    // ============================================
    let mut splash_loaded = false;
    if !initrd_start_addr.is_null() {
        let splash_names = ["globalsys/systemres/splash.bmp", "globalsys/systemres/bg.bmp"];
        for &name in splash_names.iter() {
            if let Some(bmp_bytes) = unsafe { initrd::find_file_in_tar(initrd_start_addr, name) } {
                bmp::draw_bmp_fullscreen(&mut display, bmp_bytes);
                splash_loaded = true;
                unsafe {
                    draw_splash_credits(
                        &mut display,
                        &FONT_ATLAS_BUFFER,
                        FONT_TEX_W,
                        FONT_TEX_H,
                    );
                }
                break;
            }
        }
    }

    if splash_loaded {
        timer::sleep(945);
        display.clear(Color::BLACK);
    }

    // ============================================
    // INIT CONSOLE + GFX
    // ============================================
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
        CONSOLE.set_font(
            FONT_ATLAS_BUFFER.as_ptr(),
            FONT_TEX_W,
            FONT_TEX_H,
        );
        gfx::init_graphics(
            display.raw_addr(),
            display.width(),
            display.height(),
            display.pitch(),
            CONSOLE.tex,
            CONSOLE.tex_w,
            CONSOLE.tex_h,
        );

        serial::serial_write_str("MAIN: console initialized\r\n");
    }

    println!("INFO: GOP initialized successfully");
    if initrd_found { println!("INFO: Initrd found"); }
    if mem_parsed { println!("INFO: Memory map parsed"); }
    println!("INFO: Interrupts enabled");

    unsafe { crate::acpi::init_acpi(mb_info_ptr); }

    crate::vfs::init_vfs();

    unsafe {
        let initrd_addr = initrd::INITRD_ADDR;
        if !initrd_addr.is_null() {
            println!("VFS: Mounting initrd...");
            if let Some(vfs) = crate::vfs::get_vfs() {
                vfs.mount(crate::vfs::DeviceType::Ramdisk, 0, "ram0://", &mut INITRD_FS, true);
            }
        }

        let ramdisk = &mut RAMDISK_FS;
        ramdisk.add_file("README.txt", b"OpenYanase Kernel v2.0\n");
        ramdisk.add_file("version", b"2.0.0\n");
        ramdisk.add_dir("home/");
        ramdisk.add_dir("tmp/");

        if let Some(vfs) = crate::vfs::get_vfs() {
            vfs.mount(crate::vfs::DeviceType::Ramdisk, 1, "ram1://", ramdisk, false);
        }
    }

    driver_loader::autoload_all_drivers();

    println!("========================================");
    println!("  openYanase Kernel v2.0.0");
    println!("  UEFI 64-bit / Long Mode Active!");
    println!("========================================");
    println!("");
    println!("kernel: Graphics subsystem initialized");
    println!("kernel: System boot completed successfully.");
    println!("----------------------------------------");
    println!("");

    // Init script (globalsys/init.rc) chạy trước prompt đầu tiên
    init::run();

    print_prompt();
    unsafe { CONSOLE.force_show_cursor(); }

    let mut last_blink_tick = timer::get_ticks();
    let mut last_gfx_update = timer::get_ticks();

    // ============================================
    // MAIN LOOP
    // ============================================
    loop {
        unsafe { core::arch::asm!("hlt", options(nomem, nostack)); }

        let current_tick = timer::get_ticks();

        // Console mode - cursor blink
        if console::get_display_mode() == console::DisplayMode::Console {
            unsafe {
                if CONSOLE.cursor_force_visible {
                    if current_tick.wrapping_sub(CURSOR_FORCE_TIMER) > 2000 {
                        CONSOLE.release_cursor_force();
                    }
                }
            }
            if current_tick.wrapping_sub(last_blink_tick) >= 500 {
                unsafe { if CONSOLE.cursor_active && !CONSOLE.cursor_force_visible { CONSOLE.toggle_cursor(); } }
                last_blink_tick = current_tick;
            }
        }

        // Keyboard input
        while let Some(ch) = keyboard::pop_char() {
            unsafe {
                if ch == crate::keyboard::KEY_F1 {
                    if console::get_display_mode() == console::DisplayMode::Console {
                        CONSOLE.hide_cursor();
                        gfx::enter_graphics_mode();
                    }
                    continue;
                }
                if ch == crate::keyboard::KEY_F2 {
                    if console::get_display_mode() == console::DisplayMode::Graphics {
                        gfx::clear_buffers();
                        gfx::exit_graphics_mode();
                        CONSOLE.force_show_cursor();
                        print_prompt();
                        CURSOR_FORCE_TIMER = current_tick;
                    }
                    continue;
                }

                if console::get_display_mode() == console::DisplayMode::Graphics { continue; }

                CURSOR_FORCE_TIMER = current_tick;
                CONSOLE.force_show_cursor();

                match ch {
                    '\n' | '\r' => {
                        CONSOLE.write_char('\n');
                        if CMD_LEN > 0 {
                            if let Ok(cmd_str) = core::str::from_utf8(&CMD_BUF[..CMD_LEN]) {
                                let cmd = cmd_str.trim();
                                if cmd == "gfx" {
                                    CONSOLE.hide_cursor();
                                    gfx::enter_graphics_mode();
                                    CMD_LEN = 0;
                                    continue;
                                } else if cmd == "tty" {
                                    if console::get_display_mode() == console::DisplayMode::Graphics {
                                        gfx::clear_buffers();
                                        gfx::exit_graphics_mode();
                                        CONSOLE.force_show_cursor();
                                        print_prompt();
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
                        CONSOLE.force_show_cursor();
                    }
                    '\x08' => {
                        if CMD_LEN > 0 {
                            CMD_LEN -= 1;
                            CONSOLE.backspace();
                            CONSOLE.force_show_cursor();
                        }
                    }
                    '\t' => {
                        if CMD_LEN > 0 {
                            if let Ok(cmd_str) = core::str::from_utf8(&CMD_BUF[..CMD_LEN]) {
                                if let Some(completed) = shell::complete_command(cmd_str.trim()) {
                                    for _ in 0..CMD_LEN { CONSOLE.backspace(); }
                                    CMD_LEN = 0;
                                    for c in completed.chars() {
                                        if CMD_LEN < CMD_BUF.len() {
                                            CMD_BUF[CMD_LEN] = c as u8;
                                            CMD_LEN += 1;
                                            CONSOLE.write_char(c);
                                        }
                                    }
                                    CONSOLE.force_show_cursor();
                                }
                            }
                        }
                    }
                    _ => {
                        if CMD_LEN < CMD_BUF.len() {
                            CMD_BUF[CMD_LEN] = ch as u8;
                            CMD_LEN += 1;
                            CONSOLE.write_char(ch);
                            CONSOLE.force_show_cursor();
                        }
                    }
                }
            }
        }

        // Graphics mode render 60 FPS
        if console::get_display_mode() == console::DisplayMode::Graphics {
            if current_tick.wrapping_sub(last_gfx_update) >= 16 {
                unsafe {
                    if gfx::is_dirty() {
                        if gfx::is_demo_mode() {
                            gfx::draw_graphics_demo();
                        }
                        gfx::update_frame();
                    }
                }
                last_gfx_update = current_tick;
            }
        }
    }
}

// ============================================
// SPLASH DRAW
// ============================================
unsafe fn draw_splash_credits(
    display: &mut GraphicsOutput,
    font_atlas: &[u32],
    tex_w: u32,
    tex_h: u32,
) {
    let glyph_w = tex_w / 16;
    let glyph_h = tex_h / 16;
    let warning_text = "WARNING: Development version.";
    let left_text = "Powered By OpenYanase v2";
    let right_text = "Creator: PmgDev64";
    let screen_w = display.width();
    let screen_h = display.height();
    let bottom_y = screen_h.saturating_sub(glyph_h + 16);

    draw_text_on_display(display, font_atlas, tex_w, glyph_w, glyph_h, warning_text, 12, 12);
    draw_text_on_display(display, font_atlas, tex_w, glyph_w, glyph_h, left_text, 12, bottom_y);

    let right_text_w = right_text.len() as u32 * glyph_w;
    let right_x = screen_w.saturating_sub(right_text_w + 12);
    draw_text_on_display(display, font_atlas, tex_w, glyph_w, glyph_h, right_text, right_x, bottom_y);
}

unsafe fn draw_text_on_display(
    display: &mut GraphicsOutput,
    font_atlas: &[u32],
    tex_w: u32,
    glyph_w: u32,
    glyph_h: u32,
    text: &str,
    start_x: u32,
    y: u32,
) {
    let mut cur_x = start_x;
    for ch in text.chars() {
        let byte = if (ch as u32) <= 0x7F { ch as u8 } else { b'?' };
        let gx = (byte as u32 % 16) * glyph_w;
        let gy = (byte as u32 / 16) * glyph_h;
        for row in 0..glyph_h {
            for col in 0..glyph_w {
                let idx = ((gy + row) * tex_w + (gx + col)) as usize;
                if idx < font_atlas.len() {
                    let a = font_atlas[idx] >> 24;
                    let x = cur_x + col;
                    let yy = y + row;
                    if a != 0 && x < display.width() && yy < display.height() {
                        let p = display.raw_addr()
                            .add(yy as usize * (display.pitch() / 4) as usize + x as usize);
                        let dst = p.read_volatile();
                        p.write_volatile(crate::graphics::font::blend_rgb(0xFFFF_FFFF, dst, a));
                    }
                }
            }
        }
        cur_x += glyph_w;
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let mut panic_writer = PanicWriter;
    let _ = write!(&mut panic_writer, "\r\n[KERNEL PANIC]\r\n");
    if let Some(loc) = info.location() {
        let _ = write!(&mut panic_writer, "{}:{}\r\n", loc.file(), loc.line());
    }
    loop { unsafe { core::arch::asm!("hlt"); } }
}

struct PanicWriter;
impl core::fmt::Write for PanicWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        serial::serial_write_str(s);
        Ok(())
    }
}