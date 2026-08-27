// src/shell.rs

pub fn execute(input: &str) {
    let cmd = input.trim();
    if cmd.is_empty() {
        return;
    }

    let mut parts = cmd.split_whitespace();
    let command = parts.next().unwrap_or("");

    match command {
        "help" => {
            crate::println!("=== OpenYanase Kernel Shell v0.1 ===");
            crate::println!("  help        - Display available commands");
            crate::println!("  clear / cls - Clear console screen");
            crate::println!("  echo <text> - Print text to console");
            crate::println!("  info        - Show system & hardware info");
            crate::println!("  ls          - List files in initial ramdisk (initrd)");
            crate::println!("  setfont     - Change console font (e.g., setfont font2.psf)");
            crate::println!("  runapp <pkg> - Load & run a sandboxed app package");
            crate::println!("  startx      - Launch VBE Desktop GUI Mode");
            crate::println!("  panic       - Trigger test Kernel Panic");
            crate::println!("  reboot      - Restart system via PS/2 Controller");
            crate::println!("  drivers     - List loaded kernel drivers");
            crate::println!("  loaddrv <pkg> <type> [pri] - Load a driver");
            crate::println!("  unloaddrv <pid> - Unload a driver by PID");
            crate::println!("  drvstate <pid> - Show driver state");
            crate::println!("  <app_name>  - Directly execute package");
        }

        "ls" => {
            crate::initrd::list_files();
        }

        "setfont" => {
            let font_name = parts.next().unwrap_or("");
            if font_name.is_empty() {
                crate::println!("Usage: setfont <font_filename.psf>");
                return;
            }

            unsafe {
                if crate::initrd::INITRD_ADDR.is_null() {
                    crate::println!("ERROR: Initrd is not loaded.");
                    return;
                }

                if let Some(font_bytes) = crate::initrd::find_file_in_tar(crate::initrd::INITRD_ADDR, font_name) {
                    let (tex_w, tex_h) = crate::vbe::bake_psf_to_texture(font_bytes, &mut crate::FONT_TEXTURE, 0x00FFFFFF);

                    if tex_w > 0 && tex_h > 0 {
                        crate::console::CONSOLE.set_font(crate::FONT_TEXTURE.as_ptr(), tex_w, tex_h);
                        crate::println!("Successfully updated font '{}' and redrew screen.", font_name);
                    } else {
                        crate::println!("ERROR: Failed to parse font '{}' or unsupported format.", font_name);
                    }
                } else {
                    crate::println!("ERROR: Font file '{}' not found in initrd.", font_name);
                }
            }
        }

        "runapp" => {
            let pkg_name = parts.next().unwrap_or("");
            if pkg_name.is_empty() {
                crate::println!("Usage: runapp <package.abp>");
                return;
            }
            crate::appload::run_app_package(pkg_name);
        }

        "startx" => {
            crate::println!("Starting OpenYanase Desktop Environment...");
            crate::desktop::run_desktop();
        }

        "clear" | "cls" => {
            unsafe {
                crate::console::CONSOLE.clear();
            }
        }

        "echo" => {
            let rest = cmd.strip_prefix("echo ").unwrap_or("").trim();
            crate::println!("{}", rest);
        }

        "info" => {
            crate::println!("OS Name      : OpenYanase Kernel");
            crate::println!("Architecture : x86 (IA-32 Protected Mode)");
            crate::println!("Privilege    : Ring 0 (Kernel Land)");
            crate::println!("Display      : VBE Linear Framebuffer 800x600 32bpp");
            crate::println!("Interrupts   : Remapped 8259 PIC + IDT Enabled");
        }

        "panic" => {
            panic!("Test Panic triggered manually from Shell!");
        }

        "reboot" => {
            crate::println!("Rebooting system...");
            unsafe {
                crate::cpu::outb(0x64, 0xFE);
            }
        }

        "drivers" | "lsdrv" => {
            crate::driver::list_drivers();
        }

        "loaddrv" => {
            let name = parts.next().unwrap_or("");
            let dtype_str = parts.next().unwrap_or("input");
            let pri_str = parts.next().unwrap_or("5");

            if name.is_empty() {
                crate::println!("Usage: loaddrv <package> <type> [priority]");
                crate::println!("Types: block, net, input, display, audio, hid, bus, char");
                return;
            }

            let dtype = match dtype_str {
                "block" => crate::driver::DriverType::Block,
                "net" => crate::driver::DriverType::Net,
                "input" => crate::driver::DriverType::Input,
                "display" => crate::driver::DriverType::Display,
                "audio" => crate::driver::DriverType::Audio,
                "hid" => crate::driver::DriverType::Hid,
                "bus" => crate::driver::DriverType::Bus,
                "char" => crate::driver::DriverType::Char,
                _ => crate::driver::DriverType::Input,
            };

            let priority = pri_str.parse::<u8>().unwrap_or(5);
            let full_name = build_driver_name(name);

            if let Some(pid) = crate::driver::load_driver(full_name, dtype, priority) {
                crate::println!("Driver '{}' loaded with PID {}", full_name, pid);
                // Chạy driver đủ lần để in hết thông báo trước khi prompt mới xuất hiện
                for _ in 0..5 {
                    crate::driver::run_drivers(50);
                }
            } else {
                crate::println!("Failed to load driver '{}'", full_name);
            }
        }

        "unloaddrv" => {
            let pid_str = parts.next().unwrap_or("");
            if pid_str.is_empty() {
                crate::println!("Usage: unloaddrv <pid>");
                return;
            }

            if let Ok(pid) = pid_str.parse::<u32>() {
                unsafe {
                    if crate::driver::DRIVER_MANAGER.unload_driver(pid) {
                        crate::println!("Driver PID {} unloaded", pid);
                    } else {
                        crate::println!("Failed to unload driver PID {}", pid);
                    }
                }
            } else {
                crate::println!("Invalid PID: {}", pid_str);
            }
        }

        "drvstate" => {
            let pid_str = parts.next().unwrap_or("");
            if pid_str.is_empty() {
                crate::println!("Usage: drvstate <pid>");
                return;
            }

            if let Ok(pid) = pid_str.parse::<u32>() {
                if let Some(info) = crate::driver::get_driver_info(pid) {
                    crate::println!("Driver PID: {}", pid);
                    crate::println!("  Name:   {}", info.name_str());
                    crate::println!("  Type:   {}", info.driver_type_enum().as_str());
                    crate::println!("  State:  {}", info.state_enum().as_str());
                    crate::println!("  Pri:    {}", info.priority);

                    let mut irq_buf = [0u8; 32];
                    let mut irq_pos = 0;
                    for j in 0..info.irq_count {
                        if j > 0 && irq_pos < 31 {
                            irq_buf[irq_pos] = b',';
                            irq_pos += 1;
                        }
                        if irq_pos < 31 {
                            let digit = info.claimed_irqs[j];
                            if digit >= 10 {
                                irq_buf[irq_pos] = b'0' + (digit / 10);
                                irq_pos += 1;
                                if irq_pos < 31 {
                                    irq_buf[irq_pos] = b'0' + (digit % 10);
                                    irq_pos += 1;
                                }
                            } else {
                                irq_buf[irq_pos] = b'0' + digit;
                                irq_pos += 1;
                            }
                        }
                    }
                    let irq_str = core::str::from_utf8(&irq_buf[..irq_pos]).unwrap_or("none");
                    crate::println!("  IRQs:   {}", irq_str);
                    crate::println!("  Events: {}", info.event_count);
                } else {
                    crate::println!("Driver PID {} not found", pid);
                }
            } else {
                crate::println!("Invalid PID: {}", pid_str);
            }
        }

        other => {
            if try_run_direct_app(other) {
                return;
            }
            crate::println!("Unknown command: '{}'. Type 'help' for commands.", command);
        }
    }
}

// src/shell.rs - Sửa hàm build_driver_name

fn build_driver_name(name: &str) -> &'static str {
    static mut BUF: [u8; 64] = [0; 64];
    unsafe {
        let name_bytes = name.as_bytes();
        let len = name_bytes.len().min(60);
        BUF[..len].copy_from_slice(&name_bytes[..len]);
        
        let end_len = if name.ends_with(".drv") {
            len
        } else {
            let drv_len = len + 4;
            BUF[len] = b'.';
            BUF[len + 1] = b'd';
            BUF[len + 2] = b'r';
            BUF[len + 3] = b'v';
            drv_len
        };
        
        // Quan trọng: phải trả về &'static str từ static BUF
        // Không được dùng name vì name có lifetime ngắn hơn
        core::str::from_utf8(&BUF[..end_len]).unwrap_or("unknown")
    }
}

fn try_run_direct_app(name: &str) -> bool {
    unsafe {
        if crate::initrd::INITRD_ADDR.is_null() {
            return false;
        }

        let clean_name = name.strip_prefix("./").unwrap_or(name);

        let mut name_buf = [0u8; 64];
        let mut abp_buf = [0u8; 64];
        let mut dot_slash_abp_buf = [0u8; 64];

        let name_str = make_str(&mut name_buf, clean_name);
        let abp_str = make_str_fmt(&mut abp_buf, "", ".abp", clean_name);
        let dot_slash_abp_str = make_str_fmt(&mut dot_slash_abp_buf, "./", ".abp", clean_name);

        let candidates = [name_str, abp_str, dot_slash_abp_str];

        for cand in candidates.iter() {
            if cand.is_empty() {
                continue;
            }
            if crate::initrd::find_file_in_tar(crate::initrd::INITRD_ADDR, cand).is_some() {
                crate::appload::run_app_package(cand);
                return true;
            }
        }
    }
    false
}

fn make_str<'a>(buf: &'a mut [u8], src: &str) -> &'a str {
    let bytes = src.as_bytes();
    if bytes.len() > buf.len() {
        return "";
    }
    buf[..bytes.len()].copy_from_slice(bytes);
    core::str::from_utf8(&buf[..bytes.len()]).unwrap_or("")
}

fn make_str_fmt<'a>(buf: &'a mut [u8], prefix: &str, suffix: &str, name: &str) -> &'a str {
    let p_bytes = prefix.as_bytes();
    let n_bytes = name.as_bytes();
    let s_bytes = suffix.as_bytes();
    let total = p_bytes.len() + n_bytes.len() + s_bytes.len();
    if total > buf.len() {
        return "";
    }
    let mut pos = 0;
    buf[pos..pos + p_bytes.len()].copy_from_slice(p_bytes);
    pos += p_bytes.len();
    buf[pos..pos + n_bytes.len()].copy_from_slice(n_bytes);
    pos += n_bytes.len();
    buf[pos..pos + s_bytes.len()].copy_from_slice(s_bytes);
    core::str::from_utf8(&buf[..total]).unwrap_or("")
}