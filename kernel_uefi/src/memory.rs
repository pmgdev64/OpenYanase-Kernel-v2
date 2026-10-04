// src/memory.rs
#![allow(dead_code)]
#![allow(unused_imports)]

/// Multiboot2 Memory Map Entry (Tag type 6)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Mb2MemoryMapEntry {
    pub base_addr: u64,
    pub length: u64,
    pub typ: u32,
    pub reserved: u32,
}

/// Multiboot2 Memory Map Tag
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Mb2MemoryMapTag {
    pub typ: u32,
    pub size: u32,
    pub entry_size: u32,
    pub entry_version: u32,
    // entries follow
}

/// Memory region types (from Multiboot2 spec)
pub const MEMORY_AVAILABLE: u32 = 1;
pub const MEMORY_RESERVED: u32 = 2;
pub const MEMORY_ACPI_RECLAIMABLE: u32 = 3;
pub const MEMORY_ACPI_NVS: u32 = 4;
pub const MEMORY_BAD: u32 = 5;

/// Linker symbols export từ boot.rs
extern "C" {
    static _kernel_start: u8;
    static _kernel_end: u8;
}

/// Iterator over memory map entries
pub struct MemoryMapIter {
    ptr: *const u8,
    count: usize,
    entry_size: u32,
}

impl MemoryMapIter {
    pub unsafe fn from_tag(tag_ptr: *const Mb2MemoryMapTag) -> Self {
        let tag = &*tag_ptr;
        let entry_size = tag.entry_size;
        let total_entries_size = tag.size - core::mem::size_of::<Mb2MemoryMapTag>() as u32;
        let count = if entry_size == 0 { 0 } else { total_entries_size / entry_size };

        let first_entry = (tag_ptr as *const u8).add(core::mem::size_of::<Mb2MemoryMapTag>());

        Self {
            ptr: first_entry,
            count: count as usize,
            entry_size,
        }
    }
}

impl Iterator for MemoryMapIter {
    type Item = &'static Mb2MemoryMapEntry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count == 0 {
            return None;
        }

        let entry = unsafe { &*(self.ptr as *const Mb2MemoryMapEntry) };
        self.ptr = unsafe { self.ptr.add(self.entry_size as usize) };
        self.count -= 1;
        Some(entry)
    }
}

/// Memory statistics
#[derive(Debug, Clone, Copy)]
pub struct MemoryStats {
    pub total_ram: u64,          // Tổng dung lượng vật lý (top-of-RAM)
    pub usable_ram: u64,
    pub reserved_ram: u64,
    pub kernel_start: u64,
    pub kernel_end: u64,
    pub kernel_used: u64,
    pub free_ram: u64,
    pub mem_available_percent: u64,
}

impl MemoryStats {
    pub const fn new() -> Self {
        Self {
            total_ram: 0,
            usable_ram: 0,
            reserved_ram: 0,
            kernel_start: 0,
            kernel_end: 0,
            kernel_used: 0,
            free_ram: 0,
            mem_available_percent: 0,
        }
    }
}

/// Parse memory map từ Multiboot tag.
/// Trả về (usable, reserved, total_physical).
/// total_physical = địa chỉ cao nhất của vùng nhớ trong map
/// (đúng với "tổng mem vật lý" mà firmware báo cáo).
pub fn parse_memory_map(tag_ptr: *const Mb2MemoryMapTag) -> (u64, u64, u64) {
    let mut total_usable = 0u64;
    let mut total_reserved = 0u64;
    let mut top_addr = 0u64;

    if tag_ptr.is_null() {
        return (0, 0, 0);
    }

    unsafe {
        for entry in MemoryMapIter::from_tag(tag_ptr) {
            let end = entry.base_addr.saturating_add(entry.length);
            if end > top_addr {
                top_addr = end;
            }
            match entry.typ {
                MEMORY_AVAILABLE => {
                    total_usable += entry.length;
                }
                _ => {
                    total_reserved += entry.length;
                }
            }
        }
    }

    // "Tổng mem vật lý" = usable + reserved (đúng bằng đỉnh RAM).
    // top_addr chỉ là đỉnh địa chỉ, dùng làm sanity check.
    let total_physical = if top_addr > total_usable + total_reserved {
        top_addr
    } else {
        total_usable + total_reserved
    };

    (total_usable, total_reserved, total_physical)
}

/// Kích thước kernel thật = (_kernel_end - _kernel_start)
pub fn get_kernel_size() -> u64 {
    let start = unsafe { core::ptr::addr_of!(_kernel_start) as u64 };
    let end   = unsafe { core::ptr::addr_of!(_kernel_end)   as u64 };
    end.saturating_sub(start)
}

/// Lấy địa chỉ bắt đầu / kết thúc kernel
pub fn get_kernel_range() -> (u64, u64) {
    let start = unsafe { core::ptr::addr_of!(_kernel_start) as u64 };
    let end   = unsafe { core::ptr::addr_of!(_kernel_end)   as u64 };
    (start, end)
}

/// Calculate complete memory statistics
pub fn calculate_memory_stats(tag_ptr: *const Mb2MemoryMapTag) -> MemoryStats {
    let mut stats = MemoryStats::new();

    // Parse memory map
    let (usable, reserved, total_physical) = parse_memory_map(tag_ptr);
    stats.usable_ram   = usable;
    stats.reserved_ram = reserved;
    stats.total_ram    = total_physical;

    // Kernel footprint thật
    let (ks, ke) = get_kernel_range();
    stats.kernel_start = ks;
    stats.kernel_end   = ke;
    stats.kernel_used  = ke.saturating_sub(ks);

    // Overhead runtime động (allocator, buffers, driver VM...)
    let overhead: u64 = 4 * 1024 * 1024; // 4MB
    let used = stats.kernel_used.saturating_add(overhead);
    stats.free_ram = stats.usable_ram.saturating_sub(used);

    // Percentage
    if stats.total_ram > 0 {
        stats.mem_available_percent = (stats.free_ram * 100) / stats.total_ram;
    }

    stats
}

/// Format memory size to human readable string (buffer tĩnh)
pub fn format_memory_size(bytes: u64) -> &'static str {
    const BUFFER_SIZE: usize = 32;
    static mut BUFFER: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];

    let (value, unit) = if bytes >= 1024 * 1024 * 1024 {
        (bytes / (1024 * 1024 * 1024), "GB")
    } else if bytes >= 1024 * 1024 {
        (bytes / (1024 * 1024), "MB")
    } else if bytes >= 1024 {
        (bytes / 1024, "KB")
    } else {
        (bytes, "B")
    };

    unsafe {
        let mut pos = 0;
        let mut num = value;
        let mut digits = [0u8; 20];
        let mut digit_count = 0;

        if num == 0 {
            digits[0] = b'0';
            digit_count = 1;
        } else {
            while num > 0 {
                digits[digit_count] = b'0' + (num % 10) as u8;
                digit_count += 1;
                num /= 10;
            }
        }

        for i in 0..digit_count {
            BUFFER[pos] = digits[digit_count - 1 - i];
            pos += 1;
        }

        let unit_bytes = unit.as_bytes();
        for &b in unit_bytes {
            BUFFER[pos] = b;
            pos += 1;
        }

        BUFFER[pos] = 0;
        core::str::from_utf8_unchecked(&BUFFER[..pos])
    }
}

/// Get memory tag from Multiboot info
pub unsafe fn find_memory_tag(mb_info_ptr: u64) -> *const Mb2MemoryMapTag {
    if mb_info_ptr == 0 {
        return core::ptr::null();
    }

    let addr = mb_info_ptr as *const u32;
    let total_size = addr.read_volatile();
    let mut current = mb_info_ptr + 8;
    let end = mb_info_ptr + total_size as u64;

    while current < end {
        let tag_ptr = current as *const u32;
        let tag_type = tag_ptr.read_volatile();
        let tag_size = tag_ptr.add(1).read_volatile();

        if tag_type == 0 {
            break;
        }

        // Tag type 6 = Multiboot2 Memory Map
        if tag_type == 6 {
            return current as *const Mb2MemoryMapTag;
        }

        current = (current + tag_size as u64 + 7) & !7;
    }

    core::ptr::null()
}