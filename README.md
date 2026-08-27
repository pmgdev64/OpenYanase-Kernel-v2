# OpenYanase Kernel v2

**OpenYanase Kernel v2** is a custom operating system written in Rust, featuring a dual-boot architecture (Legacy BIOS x86 & UEFI x86_64), its own custom compiler toolchain (**YBC Compiler**), and an integrated Runtime/VM environment.

---

## 🏗 Key Features & Architecture

* **Dual Boot Subsystems**:
  * **Kernel Legacy (`i686-unknown-none`)**: Supports VBE graphics, AC97 audio, PIC/GDT/IDT management, virtual memory, and custom driver runtime.
  * **Kernel UEFI (`x86_64-uefi-none`)**: Handles ACPI memory parsing, GOP framebuffer graphics, windowing engine, system calls, and YBC VM execution.
* **YBC Compiler (`tools/ybc_compiler`)**: A dedicated compiler toolchain featuring Lexer, Parser, AST, Resolver, and Codegen modules that compile custom language source code into bytecode.
* **Graphics & Window System**: Features custom surface drawing, font rendering engines, BMP image parsing, and desktop UI composition.

---

## 📁 Repository Structure

* `kernel_legacy/`: Source code targeting 32-bit Legacy BIOS environments.
* `kernel_uefi/`: Source code targeting 64-bit UEFI environments.
* `tools/ybc_compiler/`: The YBC language compiler infrastructure.

---

## 🛠 Building & Running

### Prerequisites
* Rust Nightly toolchains (`x86_64-unknown-uefi`, `i686-unknown-none`)
* `cargo` build system
* `QEMU` emulator

### Launch UEFI Kernel
```bash
cd kernel_uefi
cargo run
```

### Launch Legacy Kernel
```bash
cd kernel_legacy
cargo run
```

### Run YBC Compiler
```bash
cd tools/ybc_compiler
cargo run -- <entry.yl> <output.ybc> [--include=dir1,dir2]
```

---

## 📜 License & Credits
Developed and maintained by **PmgTeam**.
