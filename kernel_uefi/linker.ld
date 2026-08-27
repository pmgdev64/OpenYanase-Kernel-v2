ENTRY(_start)

SECTIONS
{
    /* 
     * Đổi Base Address từ 1M lên 2M (0x200000). 
     * UEFI Firmware thường chiếm dụng hoặc bảo lưu rất nhiều vùng nhớ dưới 1MB. 
     * Nạp ở mốc 2M giúp an toàn vượt qua vùng nhớ của Boot Services và 
     * tối ưu tốt nếu sau này bạn dùng Huge Page (2MB) cho Kernel.
     */
    . = 2M;

    /* Section chứa Multiboot2 Header phải nằm đầu tiên */
    .boot : ALIGN(4K)
    {
        KEEP(*(.multiboot))
    }

    /* 
     * BẮT BUỘC ALIGN(4K) cho các section còn lại trên x86_64.
     * Kích thước 1 Page mặc định là 4KB (4096 bytes). 
     * Việc tách riêng các vùng nhớ giúp bạn dễ dàng thiết lập quyền hạn trong Page Table sau này:
     * - .text: Read, Execute
     * - .rodata: Read-only
     * - .data / .bss: Read, Write
     */
    .text : ALIGN(4K)
    {
        *(.text .text.*)
    }

    .rodata : ALIGN(4K)
    {
        *(.rodata .rodata.*)
    }

    .data : ALIGN(4K)
    {
        *(.data .data.*)
    }

    .bss : ALIGN(4K)
    {
        *(COMMON)
        *(.bss .bss.*)
        
        /* 
         * Theo chuẩn x86_64 System V ABI, Stack Pointer phải được căn lề 16-byte 
         * trước khi thực thi bất kỳ hàm C/Rust nào.
         */
        . = ALIGN(16);
        stack_bottom = .;
        . += 64K;
        stack_top = .;
    }

    /DISCARD/ :
    {
        *(.comment)
        *(.eh_frame)
        *(.note.*)
    }
}