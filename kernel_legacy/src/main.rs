// src/main.rs

#![no_std]
#![no_main]

mod initrd;
mod vbe;
mod serial;
mod console;
mod bmp;
mod cpu;
mod gdt;
mod pic;
mod idt;
mod keyboard;
mod shell;
mod desktop;
mod mouse;
mod timer;
mod vm;
mod appload;
mod process;
mod bugcheck;
mod driver;

use core::arch::global_asm;
use core::panic::PanicInfo;
use core::fmt::Write;

global_asm!(
r#"
    .section .multiboot, "a"
    .align 8
    multiboot_header:
        .long 0xE85250D6
        .long 0
        .long header_end - multiboot_header
        .long -(0xE85250D6 + 0 + (header_end - multiboot_header))

        .align 8
    framebuffer_tag_start:
        .short 5
        .short 0
        .long framebuffer_tag_end - framebuffer_tag_start
        .long 800
        .long 600
        .long 32
    framebuffer_tag_end:

        .align 8
        .short 0
        .short 0
        .long 8
    header_end:

    .section .text
    .extern stack_top
    .global _start
    .type _start, @function
    _start:
        cli
        mov esp, offset stack_top
        mov ebp, esp

        push ebx
        push eax
        call kmain

    .hang:
        hlt
        jmp .hang
"#
);

#[repr(C)]
struct TagHeader {
    typ: u32,
    size: u32,
}

#[repr(C)]
struct Mb2TagModule {
    typ: u32,
    size: u32,
    mod_start: u32,
    mod_end: u32,
    string: [u8; 0],
}

pub static mut FONT_TEXTURE: [u32; 512 * 512] = [0; 512 * 512];

fn u32_to_str(mut num: u32, buf: &mut [u8]) -> usize {
    if num == 0 {
        buf[0] = b'0';
        return 1;
    }
    
    let mut temp = [0u8; 10];
    let mut i = 0;
    
    while num > 0 {
        temp[i] = b'0' + (num % 10) as u8;
        num /= 10;
        i += 1;
    }
    
    let mut pos = 0;
    for j in (0..i).rev() {
        buf[pos] = temp[j];
        pos += 1;
    }
    
    pos
}

fn get_splash_filename(width: u32, height: u32, buf: &mut [u8]) -> usize {
    let prefix = "splash_";
    let suffix = ".bmp";
    
    let mut pos = 0;
    
    for &b in prefix.as_bytes() {
        if pos < buf.len() - 1 {
            buf[pos] = b;
            pos += 1;
        }
    }
    
    let mut num_buf = [0u8; 10];
    let num_len = u32_to_str(width, &mut num_buf);
    for i in 0..num_len {
        if pos < buf.len() - 1 {
            buf[pos] = num_buf[i];
            pos += 1;
        }
    }
    
    if pos < buf.len() - 1 {
        buf[pos] = b'x';
        pos += 1;
    }
    
    let num_len = u32_to_str(height, &mut num_buf);
    for i in 0..num_len {
        if pos < buf.len() - 1 {
            buf[pos] = num_buf[i];
            pos += 1;
        }
    }
    
    for &b in suffix.as_bytes() {
        if pos < buf.len() - 1 {
            buf[pos] = b;
            pos += 1;
        }
    }
    
    if pos < buf.len() {
        buf[pos] = 0;
    }
    
    pos
}

fn check_bmp_size(bmp_data: &[u8], fb_w: u32, fb_h: u32) -> bool {
    if bmp_data.len() < 54 || bmp_data[0] != b'B' || bmp_data[1] != b'M' {
        return false;
    }
    
    let read_i32 = |offset: usize| -> i32 {
        i32::from_le_bytes([
            bmp_data[offset], 
            bmp_data[offset+1], 
            bmp_data[offset+2], 
            bmp_data[offset+3]
        ])
    };
    
    let bmp_width = read_i32(18) as u32;
    let bmp_height = read_i32(22).abs() as u32;
    
    bmp_width == fb_w && bmp_height == fb_h
}

fn get_bmp_size(bmp_data: &[u8]) -> (u32, u32) {
    if bmp_data.len() < 54 {
        return (0, 0);
    }
    
    let read_i32 = |offset: usize| -> i32 {
        i32::from_le_bytes([
            bmp_data[offset], 
            bmp_data[offset+1], 
            bmp_data[offset+2], 
            bmp_data[offset+3]
        ])
    };
    
    let width = read_i32(18) as u32;
    let height = read_i32(22).abs() as u32;
    
    (width, height)
}

/// Bật I/O Space và Bus Master cho AC97 PCI Device (Vendor 0x8086, Device 0x2415)
fn enable_ac97_pci_busmaster() {
    // Đọc/Ghi PCI Config Space qua Port 0xCF8 & 0xCFC
    // Quét Bus 0, Slot 0..32, Func 0
    for slot in 0..32 {
        let address = (1 << 31) | (slot << 11);
        unsafe {
            cpu::outl(0xCF8, address);
            let id = cpu::inl(0xCFC);
            // QEMU AC97 Device ID = 0x24158086
            if id == 0x24158086 {
                // Đọc Command Register tại Offset 0x04
                cpu::outl(0xCF8, address | 0x04);
                let cmd = cpu::inl(0xCFC);
                // Bật Bit 0 (IO Space) + Bit 2 (Bus Master)
                cpu::outl(0xCFC, cmd | 0x05);
                return;
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn kmain(magic: u32, mb_info_ptr: u32) -> ! {
    serial::serial_init();
    let mut serial_out = serial::Serial;
    
    writeln!(&mut serial_out, "\n=== OpenYanase Kernel v0.1 ===").unwrap();
    writeln!(&mut serial_out, "Magic: 0x{:X}", magic).unwrap();
    
    if magic != 0x36D76289 {
        writeln!(&mut serial_out, "ERROR: Invalid Multiboot2 magic number!").unwrap();
        loop { unsafe { core::arch::asm!("hlt", options(nomem, nostack)); } }
    }

    unsafe {
        core::arch::asm!("cli", options(nomem, nostack));
    }

    gdt::init();
    writeln!(&mut serial_out, "1. GDT Initialized.").unwrap();

    pic::init();
    writeln!(&mut serial_out, "2. PIC Initialized & Remapped.").unwrap();

    idt::init();
    writeln!(&mut serial_out, "3. IDT Initialized.").unwrap();

    timer::init_timer(1000);
    writeln!(&mut serial_out, "4. PIT Timer Initialized (1000Hz).").unwrap();

    mouse::init_mouse();
    enable_ac97_pci_busmaster(); // Bật Bus Master DMA cho AC97

    unsafe {
        core::arch::asm!("sti", options(nomem, nostack));
    }
    writeln!(&mut serial_out, "5. Hardware Interrupts Enabled (STI).").unwrap();

    let mut fb_tag: Option<&vbe::Mb2TagFramebuffer> = None;
    let mut initrd_tar_addr: Option<*const u8> = None;

    unsafe {
        let total_size = *(mb_info_ptr as *const u32);
        writeln!(&mut serial_out, "Total MB2 Info size: {}", total_size).unwrap();
        
        let mut offset = 8;
        
        while offset < total_size {
            let tag_ptr = (mb_info_ptr + offset) as *const TagHeader;
            let tag = &*tag_ptr;
            
            if tag.typ == 0 {
                break;
            }
            
            if tag.typ == 3 {
                let mod_tag = &*(tag_ptr as *const Mb2TagModule);
                initrd_tar_addr = Some(mod_tag.mod_start as *const u8);
                crate::initrd::INITRD_ADDR = mod_tag.mod_start as *const u8;
                writeln!(&mut serial_out, "Initrd found at: 0x{:X}", mod_tag.mod_start).unwrap();
            } else if tag.typ == 8 {
                fb_tag = Some(&*(tag_ptr as *const vbe::Mb2TagFramebuffer));
                let fb = fb_tag.unwrap();
                writeln!(&mut serial_out, "Framebuffer found: {}x{}x{} at 0x{:X}", 
                    fb.framebuffer_width, fb.framebuffer_height, 
                    fb.framebuffer_bpp, fb.framebuffer_addr).unwrap();
            }
            
            offset = (offset + tag.size + 7) & !7;
        }

        if let Some(fb) = fb_tag {
            if let Some(tar_ptr) = initrd_tar_addr {
                
                // --- SPLASH SCREEN ---
                let fb_w = fb.framebuffer_width;
                let fb_h = fb.framebuffer_height;
                
                let size = (fb_h * fb.framebuffer_pitch) as usize;
                core::ptr::write_bytes(fb.framebuffer_addr as *mut u8, 0, size);

                writeln!(&mut serial_out, "Looking for splash screen ({}x{})...", fb_w, fb_h).unwrap();
                
                let mut splash_name_buf = [0u8; 64];
                let name_len = get_splash_filename(fb_w, fb_h, &mut splash_name_buf);
                let splash_name = core::str::from_utf8(&splash_name_buf[..name_len]).unwrap_or("splash.bmp");
                
                let mut found_splash = false;
                
                if let Some(bmp_bytes) = initrd::find_file_in_tar(tar_ptr, splash_name) {
                    if check_bmp_size(bmp_bytes, fb_w, fb_h) {
                        bmp::draw_bmp_fullscreen(fb, bmp_bytes);
                        timer::sleep_ms(945);
                        found_splash = true;
                    }
                }
                
                if !found_splash {
                    if let Some(bmp_bytes) = initrd::find_file_in_tar(tar_ptr, "splash.bmp") {
                        if check_bmp_size(bmp_bytes, fb_w, fb_h) {
                            bmp::draw_bmp_fullscreen(fb, bmp_bytes);
                            timer::sleep_ms(945);
                        }
                    }
                }

                // --- VTTY CONSOLE INIT ---
                writeln!(&mut serial_out, "Looking for font.psf...").unwrap();
                if let Some(font_bytes) = initrd::find_file_in_tar(tar_ptr, "font.psf") {
                    let (tex_w, tex_h) = vbe::bake_psf_to_texture(font_bytes, &mut FONT_TEXTURE, 0x00FFFFFF);
                    if tex_w > 0 && tex_h > 0 {
                        console::CONSOLE.init(fb, FONT_TEXTURE.as_ptr(), tex_w, tex_h);
                    }
                }

                // --- BUILT-IN DRIVERS INIT (ĐỘC LẬP HOÀN TOÀN) ---
                println!("Loading built-in drivers...");
                crate::driver::init_builtin_drivers();

                for _ in 0..5 {
                    crate::driver::run_drivers(50);
                }

                println!("Driver scheduler initialized.");
                println!("Welcome to OpenYanase Kernel v0.1!");
                println!("Resolution: {}x{} ({} bpp)", fb_w, fb_h, fb.framebuffer_bpp);
                println!("--------------------------------------");
                print!("> ");

            } else {
                writeln!(&mut serial_out, "No initrd found").unwrap();
            }
        } else {
            writeln!(&mut serial_out, "ERROR: No framebuffer tag found!").unwrap();
        }
    }

    // --- MAIN LOOP ---
    let mut cursor_visible = true;
    let mut last_blink_time = timer::get_ticks();
    let mut last_driver_time = timer::get_ticks();
    const DRIVER_INTERVAL_MS: u64 = 10;

    loop {
        let current_time = timer::get_ticks();

        if current_time - last_blink_time >= 500 {
            unsafe {
                console::CONSOLE.draw_cursor(cursor_visible);
            }
            cursor_visible = !cursor_visible;
            last_blink_time = current_time;
        }

        if current_time - last_driver_time >= DRIVER_INTERVAL_MS {
            crate::driver::run_drivers(20);
            last_driver_time = current_time;
        }

        unsafe {
            core::arch::asm!("hlt", options(nomem, nostack));
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let mut serial = serial::Serial;
    let _ = writeln!(&mut serial, "\n!!! KERNEL PANIC !!!\n{}", info);
    
    crate::println!("\n!!! KERNEL PANIC !!!");
    crate::println!("{}", info);
    
    crate::bugcheck::bugcheck(crate::bugcheck::BugCheckCode::UnknownFatalFailure);
}