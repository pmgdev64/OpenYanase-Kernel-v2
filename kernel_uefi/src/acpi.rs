// src/acpi.rs

use core::ptr::read_unaligned;

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct RsdpHeader {
    pub signature: [u8; 8],
    pub checksum: u8,
    pub oem_id: [u8; 6],
    pub revision: u8,
    pub rsdt_address: u32,
    pub length: u32,
    pub xsdt_address: u64,
    pub extended_checksum: u8,
    pub reserved: [u8; 3],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct AcpiTableHeader {
    pub signature: [u8; 4],
    pub length: u32,
    pub revision: u8,
    pub checksum: u8,
    pub oem_id: [u8; 6],
    pub oem_table_id: [u8; 8],
    pub oem_revision: u32,
    pub creator_id: u32,
    pub creator_revision: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct GenericAddressStructure {
    pub address_space: u8,
    pub bit_width: u8,
    pub bit_offset: u8,
    pub access_size: u8,
    pub address: u64,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct Fadt {
    pub header: AcpiTableHeader,
    pub firmware_ctrl: u32,
    pub dsdt: u32,
    pub reserved1: u8,
    pub preferred_pm_profile: u8,
    pub sci_interrupt: u16,
    pub smi_command_port: u32,
    pub acpi_enable: u8,
    pub acpi_disable: u8,
    pub S4bios_req: u8,
    pub pstate_control: u8,
    pub pm1a_event_block: u32,
    pub pm1b_event_block: u32,
    pub pm1a_control_block: u32,
    pub pm1b_control_block: u32,
    pub pm2_control_block: u32,
    pub pm_timer_block: u32,
    pub gpe0_block: u32,
    pub gpe1_block: u32,
    pub pm1_event_length: u8,
    pub pm1_control_length: u8,
    pub pm2_control_length: u8,
    pub pm_timer_length: u8,
    pub gpe0_length: u8,
    pub gpe1_length: u8,
    pub gpe1_base: u8,
    pub cstate_control: u8,
    pub worst_c2_latency: u16,
    pub worst_c3_latency: u16,
    pub flush_size: u16,
    pub flush_stride: u16,
    pub duty_offset: u8,
    pub duty_width: u8,
    pub day_alarm: u8,
    pub month_alarm: u8,
    pub century: u8,
    pub boot_architecture_flags: u16,
    pub reserved2: u8,
    pub flags: u32,
    pub reset_reg: GenericAddressStructure,
    pub reset_value: u8,
    pub arm_boot_arch: u16,
    pub fadt_minor_version: u8,
    pub x_firmware_ctrl: u64,
    pub x_dsdt: u64,
    pub x_pm1a_event_block: GenericAddressStructure,
    pub x_pm1b_event_block: GenericAddressStructure,
    pub x_pm1a_control_block: GenericAddressStructure,
    pub x_pm1b_control_block: GenericAddressStructure,
}

static mut PM1A_CNT_BLK: u32 = 0;
static mut PM1B_CNT_BLK: u32 = 0;
static mut SLP_TYPA: u16 = 0x2000; // Giá trị S5 mặc định cho QEMU/Bochs
static mut SLP_TYPB: u16 = 0x2000;
static mut RESET_REG_ADDR: u64 = 0;
static mut RESET_VALUE: u8 = 0;
static mut RESET_SPACE: u8 = 0; // 0 = Memory, 1 = I/O
static mut ACPI_INITIALIZED: bool = false;

unsafe fn outb(port: u16, val: u8) {
    core::arch::asm!("out dx, al", in("dx") port, in("al") val, options(nomem, nostack, preserves_flags));
}

unsafe fn outw(port: u16, val: u16) {
    core::arch::asm!("out dx, ax", in("dx") port, in("ax") val, options(nomem, nostack, preserves_flags));
}

unsafe fn inb(port: u16) -> u8 {
    let val: u8;
    core::arch::asm!("in al, dx", out("al") val, in("dx") port, options(nomem, nostack, preserves_flags));
    val
}

pub unsafe fn init_acpi(mb_info_ptr: u64) {
    if mb_info_ptr == 0 {
        crate::println!("ACPI: Invalid Multiboot info pointer");
        return;
    }

    crate::println!("ACPI: Initializing...");

    // Tìm RSDP pointer từ Multiboot2 tags
    let rsdp_ptr = match find_rsdp_from_mb2(mb_info_ptr) {
        Some(ptr) => ptr,
        None => {
            crate::println!("ACPI: RSDP tag not found in Multiboot2");
            return;
        }
    };

    let rsdp = read_unaligned(rsdp_ptr as *const RsdpHeader);

    if &rsdp.signature != b"RSD PTR " {
        crate::println!("ACPI: Invalid RSDP signature");
        return;
    }

    crate::println!("ACPI: RSDP found via Multiboot2 tag");

    // Tìm bảng FADT (Signature "FACP")
    let fadt_ptr = match find_table(rsdp_ptr, b"FACP") {
        Some(ptr) => ptr as *const Fadt,
        None => {
            crate::println!("ACPI: RSDP found but FADT lookup failed");
            return;
        }
    };

    let fadt = read_unaligned(fadt_ptr);

    // Cấu hình PM1a / PM1b Control Block
    if fadt.x_pm1a_control_block.address != 0 && fadt.x_pm1a_control_block.address_space == 1 {
        PM1A_CNT_BLK = fadt.x_pm1a_control_block.address as u32;
    } else if fadt.pm1a_control_block != 0 {
        PM1A_CNT_BLK = fadt.pm1a_control_block;
    }

    if fadt.x_pm1b_control_block.address != 0 && fadt.x_pm1b_control_block.address_space == 1 {
        PM1B_CNT_BLK = fadt.x_pm1b_control_block.address as u32;
    } else if fadt.pm1b_control_block != 0 {
        PM1B_CNT_BLK = fadt.pm1b_control_block;
    }

    // Cấu hình Reset Register
    if (fadt.flags & (1 << 10)) != 0 || fadt.reset_reg.address != 0 {
        RESET_REG_ADDR = fadt.reset_reg.address;
        RESET_VALUE = fadt.reset_value;
        RESET_SPACE = fadt.reset_reg.address_space;
    }

    // Đọc DSDT để trích xuất gói _S5 (Shutdown state)
    let dsdt_ptr = if fadt.header.revision >= 2 && fadt.x_dsdt != 0 {
        fadt.x_dsdt as *const u8
    } else if fadt.dsdt != 0 {
        fadt.dsdt as *const u8
    } else {
        core::ptr::null()
    };

    if !dsdt_ptr.is_null() {
        parse_s5_from_dsdt(dsdt_ptr);
    }

    ACPI_INITIALIZED = true;
    crate::println!("ACPI: Successfully initialized (FADT found, PM1a_CNT = {:#X})", PM1A_CNT_BLK);
}

unsafe fn find_rsdp_from_mb2(mb_info_ptr: u64) -> Option<*const u8> {
    let total_size = (mb_info_ptr as *const u32).read_volatile();
    let mut current = mb_info_ptr + 8;
    let end = mb_info_ptr + total_size as u64;

    while current < end {
        let tag_type = (current as *const u32).read_volatile();
        let tag_size = ((current + 4) as *const u32).read_volatile();

        if tag_type == 0 { break; }

        // Tag 14 = ACPI v1 RSDP, Tag 15 = ACPI v2+ RSDP
        if tag_type == 14 || tag_type == 15 {
            return Some((current + 8) as *const u8);
        }

        current = (current + tag_size as u64 + 7) & !7;
    }

    None
}

pub unsafe fn find_table(rsdp_ptr: *const u8, target_sig: &[u8; 4]) -> Option<*const AcpiTableHeader> {
    if rsdp_ptr.is_null() { return None; }

    let rsdp = read_unaligned(rsdp_ptr as *const RsdpHeader);

    // 1. Quét XSDT (64-bit pointers) nếu ACPI 2.0+
    if rsdp.revision >= 2 && rsdp.xsdt_address != 0 {
        let xsdt_ptr = rsdp.xsdt_address as *const AcpiTableHeader;
        let xsdt = read_unaligned(xsdt_ptr);
        let header_size = core::mem::size_of::<AcpiTableHeader>();

        if (xsdt.length as usize) > header_size {
            let entries_count = (xsdt.length as usize - header_size) / 8;
            let entries_ptr = (xsdt_ptr as usize + header_size) as *const u64;

            for i in 0..entries_count {
                let table_addr = read_unaligned(entries_ptr.add(i)) as *const AcpiTableHeader;
                if !table_addr.is_null() {
                    let header = read_unaligned(table_addr);
                    if &header.signature == target_sig {
                        return Some(table_addr);
                    }
                }
            }
        }
    }

    // 2. Quét RSDT (32-bit pointers) nếu ACPI 1.0
    if rsdp.rsdt_address != 0 {
        let rsdt_ptr = rsdp.rsdt_address as *const AcpiTableHeader;
        let rsdt = read_unaligned(rsdt_ptr);
        let header_size = core::mem::size_of::<AcpiTableHeader>();

        if (rsdt.length as usize) > header_size {
            let entries_count = (rsdt.length as usize - header_size) / 4;
            let entries_ptr = (rsdt_ptr as usize + header_size) as *const u32;

            for i in 0..entries_count {
                let table_addr = read_unaligned(entries_ptr.add(i)) as *const AcpiTableHeader;
                if !table_addr.is_null() {
                    let header = read_unaligned(table_addr);
                    if &header.signature == target_sig {
                        return Some(table_addr);
                    }
                }
            }
        }
    }

    None
}

unsafe fn parse_s5_from_dsdt(dsdt_ptr: *const u8) -> bool {
    let header = read_unaligned(dsdt_ptr as *const AcpiTableHeader);
    if &header.signature != b"DSDT" {
        return false;
    }

    let len = header.length as usize;
    let slice = core::slice::from_raw_parts(dsdt_ptr, len);

    let mut i = 0;
    while i < len - 4 {
        if &slice[i..i + 4] == b"_S5_" {
            let mut ptr = i + 4;
            if ptr >= len { break; }

            if slice[ptr] == 0x12 { // PackageOp
                ptr += 1;
                let pkg_len_byte = slice[ptr];
                let num_bytes = (pkg_len_byte >> 6) as usize;
                ptr += 1 + num_bytes;

                if ptr < len { ptr += 1; } // Skip num_elements

                if ptr < len && slice[ptr] == 0x0A { ptr += 1; }
                if ptr < len {
                    SLP_TYPA = (slice[ptr] as u16) << 10;
                    ptr += 1;
                }

                if ptr < len && slice[ptr] == 0x0A { ptr += 1; }
                if ptr < len {
                    SLP_TYPB = (slice[ptr] as u16) << 10;
                }

                return true;
            }
        }
        i += 1;
    }

    false
}

// src/acpi.rs

pub fn shutdown() -> ! {
    crate::println!("kernel: Shutting down system via ACPI...");

    let slp_en = 1 << 13;

    unsafe {
        if ACPI_INITIALIZED && PM1A_CNT_BLK != 0 {
            outw(PM1A_CNT_BLK as u16, SLP_TYPA | slp_en);
            if PM1B_CNT_BLK != 0 {
                outw(PM1B_CNT_BLK as u16, SLP_TYPB | slp_en);
            }
        }

        // Fallback cho QEMU, Bochs, VirtualBox
        outw(0x604, 0x2000);   // QEMU testdev / debugexit
        outw(0xB004, 0x2000);  // Bochs ACPI shutdown
        outw(0x4004, 0x3400);  // VirtualBox ACPI
        outb(0x501, 0x00);     // ISA debug exit

        loop {
            core::arch::asm!("cli; hlt");
        }
    }
}

pub fn reboot() -> ! {
    crate::println!("kernel: Initiating hardware reboot sequence...");

    unsafe {
        // 1. Tắt toàn bộ ngắt phần cứng ngay lập tức
        core::arch::asm!("cli", options(nomem, nostack));

        // 2. Thử ACPI Reset Register
        if RESET_REG_ADDR != 0 {
            if RESET_SPACE == 1 {
                outb(RESET_REG_ADDR as u16, RESET_VALUE);
            } else if RESET_SPACE == 0 {
                (RESET_REG_ADDR as *mut u8).write_volatile(RESET_VALUE);
            }
        }

        // 3. Thử PCI Reset Port (0xCF9)
        outb(0xCF9, 0x02);
        outb(0xCF9, 0x06);

        // 4. Thử PS/2 Keyboard Controller Reset (0x64 -> 0xFE)
        let mut timeout = 0;
        while (inb(0x64) & 0x02) != 0 && timeout < 10000 {
            timeout += 1;
        }
        outb(0x64, 0xFE);

        // 5. Thử Fast Reset via System Control Port A (0x92)
        let val = inb(0x92);
        if (val & 0x01) == 0 {
            outb(0x92, val | 0x01);
        }

        // 6. Fallback an toàn cho Bare-Metal
        crate::println!("kernel: Reset failed on this hardware. Please power off manually.");

        loop {
            core::arch::asm!("hlt", options(nomem, nostack));
        }
    }
}