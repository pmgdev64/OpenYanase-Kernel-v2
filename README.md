# OpenYanase Kernel v2

**OpenYanase Kernel v2** is the next-generation evolution of OpenYanase Kernel v1. Rebuilt and heavily enhanced in Rust, v2 introduces a dual-boot architecture (Legacy BIOS x86 & UEFI x86_64), its own custom compiler toolchain (**YBC Compiler**), and an integrated Runtime/VM environment.

---

## 🏗 Key Features & Architecture

* **Dual Boot Subsystems**:
  * **Kernel Legacy (`i686-unknown-none`)**: Legacy sub-system architecture undergoing refactoring; builds into `kernel.iso`. Early driver implementations (e.g., AC97, PC Speaker) are deprecated or currently being restructured into a unified driver model.
  * **Kernel UEFI (`x86_64-unknown-none`)**: Uses the bare-metal `x86_64-unknown-none` target (`x86_64-unknown-uefi` has been completely deprecated and discontinued). Builds into `openyanase.iso`. Handles ACPI memory parsing, GOP framebuffer graphics, windowing engine, system calls, and YBC VM execution.
* **YBC Compiler (`tools/ybc_compiler`)**: A dedicated compiler toolchain featuring Lexer, Parser, AST, Resolver, and Codegen modules that compile custom language source code into bytecode.
* **Graphics & Window System**: Features custom surface drawing, font rendering engines, BMP image parsing, and desktop UI composition.

---

## 📁 Repository Structure

* `kernel_legacy/`: Source code targeting 32-bit Legacy BIOS environments (builds `kernel.iso`).
* `kernel_uefi/`: Source code targeting 64-bit UEFI environments via `x86_64-unknown-none` (builds `openyanase.iso`).
* `tools/ybc_compiler/`: The YBC language compiler infrastructure.

---

## 🛠 Building & Running

### Prerequisites
* **Host Environment**: Windows with **MSYS2** (MINGW64 / UCRT64 shell)
* Rust Nightly toolchains (`x86_64-unknown-none`, `i686-unknown-none`)
* `cargo` build system
* `QEMU` emulator

### Launch UEFI Kernel (`openyanase.iso`)
```bash
cd kernel_uefi
cargo run
```

### Launch Legacy Kernel (`kernel.iso`)
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
