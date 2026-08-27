// src/user.rs
use core::arch::global_asm;
use crate::println;
use crate::idt;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum UserRole {
    Root = 0,
    RegularUser = 1,
}

pub struct UserContext {
    pub uid: u32,
    pub role: UserRole,
    pub username: &'static str,
}

static mut CURRENT_USER: UserContext = UserContext {
    uid: 0,
    role: UserRole::Root,
    username: "root",
};

pub fn get_current_user() -> &'static UserContext {
    unsafe { &CURRENT_USER }
}

pub fn set_current_user(uid: u32, role: UserRole, username: &'static str) {
    unsafe {
        // Disable interrupts khi chuyển user
        idt::disable_interrupts();
        CURRENT_USER = UserContext { uid, role, username };
        println!("kernel: User context switched to '{}' (UID: {}, Role: {:?})", username, uid, role);
        idt::enable_interrupts();
    }
}

pub fn check_permission(required_role: UserRole) -> bool {
    let current = get_current_user();
    match required_role {
        UserRole::Root => current.role == UserRole::Root,
        UserRole::RegularUser => true,
    }
}

pub const USER_STACK_SIZE: usize = 16 * 1024;
pub static mut USER_STACK: [u8; USER_STACK_SIZE] = [0; USER_STACK_SIZE];

global_asm!(
    r#"
    .global jump_to_user_mode
    .extern user_entry

    jump_to_user_mode:
        cli
        mov ax, 0x23
        mov ds, ax
        mov es, ax
        mov fs, ax
        mov gs, ax

        push 0x23               /* SS */
        push rsi                /* RSP */
        pushfq                  /* RFLAGS */
        pop rax
        or rax, 0x200           /* Enable Interrupts */
        push rax
        push 0x1B               /* CS */
        push rdi                /* RIP */
        iretq
    "#
);

extern "C" {
    pub fn jump_to_user_mode(code_ptr: usize, stack_top: usize) -> !;
}

pub fn enter_user_space() {
    println!("kernel: Preparing user-space memory layout...");
    unsafe {
        let stack_top = USER_STACK.as_ptr() as usize + USER_STACK_SIZE;
        let user_code = user_mode_test_app as usize;

        println!("kernel: User stack allocated at 0x{:x}", stack_top);
        println!("kernel: Switching CPU privilege level to Ring 3...");
        jump_to_user_mode(user_code, stack_top);
    }
}

#[no_mangle]
pub extern "C" fn user_mode_test_app() -> ! {
    loop {
        unsafe { core::arch::asm!("nop"); }
    }
}