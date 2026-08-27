// src/syscall.rs
use crate::idt;
use crate::serial;
use crate::graphics::gfx;
use core::arch::global_asm;

global_asm!(
    r#"
    .global syscall_interrupt_stub
    .extern handle_syscall

    syscall_interrupt_stub:
        cld
        push rcx
        push rdx
        push rsi
        push rdi
        push r8
        push r9
        push r10
        push r11

        // args theo System V: rdi, rsi, rdx, r10, r8 ; id trong rax
        mov rcx, rax        // arg1 = syscall id
        // rdi,rsi,rdx,r10,r8 giữ nguyên làm arg2..arg6 cho hàm Rust
        sub rsp, 8          // align 16 byte trước call
        call handle_syscall
        add rsp, 8

        pop r11
        pop r10
        pop r9
        pop r8
        pop rdi
        pop rsi
        pop rdx
        pop rcx
        iretq
    "#
);

extern "C" {
    fn syscall_interrupt_stub();
}

pub fn init_syscall() {
    // Vector 0x80, DPL=3 để user-space được phép gọi int 0x80 (flags 0xEE thay vì 0x8E)
    idt::set_gate_dpl(0x80, syscall_interrupt_stub as usize as u64, 0xEE);
}

/// Giới hạn cứng cho mọi syscall nhận con trỏ từ Ring 3 — không tin bất kỳ giá trị nào
const MAX_PRINT_LEN: u64 = 4096;
const SANDBOX_MAX_TICK_SLEEP: u64 = 60_000; // chặn app tự treo kernel quá lâu

#[no_mangle]
pub extern "C" fn handle_syscall(id: u64, a1: u64, a2: u64, a3: u64, _a4: u64, _a5: u64) -> i64 {
    match id {
        1 => sys_print(a1, a2),
        2 => sys_draw_rect(a1 as i64, a2 as i64, a3 as i64, 0, 0), // TODO: mở rộng arg qua stack nếu cần >3 args ổn định
        3 => sys_get_tick(),
        4 => sys_sleep(a1),
        5 => sys_exit(a1 as i32),
        _ => {
            serial::serial_write_str("SYSCALL: unknown id, killing process\r\n");
            sys_exit(1)
        }
    }
}

fn sys_print(ptr: u64, len: u64) -> i64 {
    if len == 0 || len > MAX_PRINT_LEN {
        return -1;
    }
    // BẮT BUỘC: validate ptr nằm trong vùng nhớ user đã cấp cho process hiện tại
    // (placeholder — cần nối với process/memory manager thật khi có per-process paging)
    if !crate::process::validate_user_range(ptr, len) {
        serial::serial_write_str("SYSCALL: print ptr out of sandbox range, killing\r\n");
        return sys_exit(1);
    }

    let slice = unsafe { core::slice::from_raw_parts(ptr as *const u8, len as usize) };
    if let Ok(s) = core::str::from_utf8(slice) {
        crate::println!("{}", s);
    }
    0
}

fn sys_draw_rect(x: i64, y: i64, w: i64, _h: i64, _color: i64) -> i64 {
    // Clamp cứng để app không thể vẽ tràn ra ngoài vùng cửa sổ được cấp
    let x = x.clamp(0, 4096);
    let y = y.clamp(0, 4096);
    let w = w.clamp(0, 4096);
    unsafe { gfx::mark_dirty(); }
    let _ = (x, y, w);
    0
}

fn sys_get_tick() -> i64 {
    crate::timer::get_ticks() as i64
}

fn sys_sleep(ms: u64) -> i64 {
    let ms = ms.min(SANDBOX_MAX_TICK_SLEEP);
    crate::timer::sleep(ms);
    0
}

fn sys_exit(_code: i32) -> i64 {
    serial::serial_write_str("SYSCALL: process exited\r\n");
    loop { unsafe { core::arch::asm!("hlt"); } }
}