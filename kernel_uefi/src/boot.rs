// src/boot.rs
use core::arch::global_asm;

global_asm!(
r#"
    // Intel syntax is default, no need to specify
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
    .short 1
    .long 20
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
    .code32
    .global _start
_start:
    cli

    mov ebp, eax
    mov ebx, ebx

    mov edi, offset pml4_table
    xor eax, eax
    mov ecx, 6144
    rep stosd

    mov edi, offset pml4_table
    mov dword ptr [edi], offset pdpt_table + 3

    mov edi, offset pdpt_table
    mov dword ptr [edi], offset pd_table_0 + 3
    mov dword ptr [edi + 8], offset pd_table_1 + 3
    mov dword ptr [edi + 16], offset pd_table_2 + 3
    mov dword ptr [edi + 24], offset pd_table_3 + 3

    mov edi, offset pd_table_0
    mov eax, 0x00000083
    mov ecx, 2048
.map_4gb:
    mov dword ptr [edi], eax
    mov dword ptr [edi + 4], 0
    add eax, 0x00200000
    add edi, 8
    loop .map_4gb

    lgdt [gdt64_ptr]

    mov eax, cr4
    or eax, 1 << 5
    mov cr4, eax

    mov eax, offset pml4_table
    mov cr3, eax

    mov ecx, 0xC0000080
    rdmsr
    or eax, 1 << 8
    wrmsr

    mov eax, cr0
    or eax, 1 << 31 | 1 << 0
    mov cr0, eax

    ljmp 0x08, offset long_mode_start

    .code64
long_mode_start:
    mov rdi, rbp
    mov rsi, rbx

    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    mov rsp, offset stack_top
    mov rbp, rsp

    call kmain

.hang:
    hlt
    jmp .hang

    .section .rodata
    .align 16
gdt64:
    .quad 0
    .quad 0x00AF9A000000FFFF
    .quad 0x00CF92000000FFFF
gdt64_ptr:
    .word . - gdt64 - 1
    .long gdt64

    .section .bss
    .align 4096
pml4_table:
    .space 4096
pdpt_table:
    .space 4096
pd_table_0:
    .space 4096
pd_table_1:
    .space 4096
pd_table_2:
    .space 4096
pd_table_3:
    .space 4096

    .align 16
stack_bottom:
    .space 64 * 1024
stack_top:
"#
);