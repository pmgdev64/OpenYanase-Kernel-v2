// src/acpi.rs
use core::ptr;

#[repr(C, packed)]
pub struct AcpiRsdp {
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
pub struct AcpiSdtHeader {
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
pub struct BgrtHeader {
    pub header: AcpiSdtHeader,
    pub version: u16,
    pub status: u8,
    pub image_type: u8,
    pub image_address: u64,
    pub image_offset_x: u32,
    pub image_offset_y: u32,
}

// Status bits
pub const BGRT_STATUS_VALID: u8 = 0x01;
pub const BGRT_STATUS_DISPLAYED: u8 = 0x02;

pub fn find_bgrt(rsdp_addr: *const AcpiRsdp) -> Option<*const BgrtHeader> {
    unsafe {
        if rsdp_addr.is_null() {
            return None;
        }

        let rsdp = &*rsdp_addr;
        
        // Sử dụng XSDT nếu có (ACPI 2.0+)
        let xsdt_addr = rsdp.xsdt_address;
        if xsdt_addr == 0 {
            return None;
        }

        let xsdt_ptr = xsdt_addr as *const u32;
        let xsdt_len = *(xsdt_ptr.add(1)) as u64; // length field
        let num_entries = (xsdt_len - 36) / 8;

        for i in 0..num_entries {
            let entry_addr = *(xsdt_ptr.add(2 + i as usize)) as u64;
            let header = entry_addr as *const AcpiSdtHeader;
            
            // Kiểm tra signature "BGRT"
            let sig = &(*header).signature;
            if sig[0] == b'B' && sig[1] == b'G' && sig[2] == b'R' && sig[3] == b'T' {
                return Some(entry_addr as *const BgrtHeader);
            }
        }
    }
    
    None
}

/// Lấy logo từ BGRT
pub fn get_boot_logo(bgrt: *const BgrtHeader) -> Option<&'static [u8]> {
    unsafe {
        if bgrt.is_null() {
            return None;
        }

        let bgrt = &*bgrt;
        
        // Kiểm tra status
        if (bgrt.status & BGRT_STATUS_VALID) == 0 {
            return None;
        }

        if bgrt.image_address == 0 {
            return None;
        }

        // Đọc header BMP
        let img_ptr = bgrt.image_address as *const u8;
        if img_ptr.is_null() {
            return None;
        }

        // Kiểm tra magic BMP
        if *img_ptr != b'B' || *img_ptr.add(1) != b'M' {
            return None;
        }

        // Lấy size từ BMP header
        let size = *(img_ptr.add(2) as *const u32);
        if size == 0 || size > 1024 * 1024 * 4 { // Max 4MB
            return None;
        }

        let slice = core::slice::from_raw_parts(img_ptr, size as usize);
        Some(slice)
    }
}

/// Tìm RSDP trong memory
pub unsafe fn find_rsdp() -> Option<*const AcpiRsdp> {
    // Tìm trong EBDA (Extended BIOS Data Area)
    let ebda_seg = *(0x40E as *const u16);
    let ebda_addr = (ebda_seg as u32) << 4;
    
    // Kiểm tra EBDA
    for offset in 0..1024 {
        let addr = (ebda_addr + offset) as *const u8;
        if check_rsdp(addr) {
            return Some(addr as *const AcpiRsdp);
        }
    }

    // Tìm trong BIOS memory từ 0xE0000 đến 0xFFFFF
    for addr in (0xE0000..0x100000).step_by(16) {
        let ptr = addr as *const u8;
        if check_rsdp(ptr) {
            return Some(ptr as *const AcpiRsdp);
        }
    }

    None
}

unsafe fn check_rsdp(ptr: *const u8) -> bool {
    if ptr.is_null() {
        return false;
    }

    // Kiểm tra signature "RSD PTR "
    let sig = ptr as *const [u8; 8];
    let expected = b"RSD PTR ";
    for i in 0..8 {
        if (*sig)[i] != expected[i] {
            return false;
        }
    }

    // Kiểm tra checksum (chỉ tính 20 bytes đầu cho ACPI 1.0)
    let mut sum: u8 = 0;
    for i in 0..20 {
        sum = sum.wrapping_add(*ptr.add(i));
    }
    sum == 0
}