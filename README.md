# OpenYanase Kernel v2

**OpenYanase Kernel v2** is the next-generation evolution of OpenYanase Kernel v1. Rebuilt and heavily enhanced in Rust, v2 introduces a dual-boot architecture (Legacy BIOS x86 & UEFI x86_64), its own custom compiler toolchain (**YBC Compiler**), and an integrated Runtime/VM environment.

---

## 🏗 Key Features & Architecture

* **Dual Boot Subsystems**:
  * **Kernel Legacy (`i686-unknown-none`)**: Legacy sub-system architecture undergoing refactoring; builds into `kernel.iso`. Early driver implementations (e.g., AC97, PC Speaker) are deprecated or currently being restructured into a unified driver model.
  * **Kernel UEFI (`x86_64-unknown-none`)**: Uses the bare-metal `x86_64-unknown-none` target (`x86_64-unknown-uefi` has been completely deprecated and discontinued). Builds into `openyanase.iso`. Handles ACPI memory parsing, GOP framebuffer graphics, windowing engine, system calls, and YBC VM execution.
* **YBC Compiler (`tools/ybc_compiler`)**: A dedicated compiler toolchain featuring Lexer, Parser, AST, Resolver, and Codegen modules that compile custom language source code (`.yl`) into VM bytecode (`.ybc`).
* **Graphics & Window System**: Features custom surface drawing, font rendering engines, BMP image parsing, and desktop UI composition.

---

## 📁 Repository Structure

* `kernel_legacy/`: Source code targeting 32-bit Legacy BIOS environments (builds `kernel.iso`).
* `kernel_uefi/`: Source code targeting 64-bit UEFI environments via `x86_64-unknown-none` (builds `openyanase.iso`).
* `tools/ybc_compiler/`: The YBC language compiler infrastructure.

---

## 🛠 Building & Setup Instructions

### Prerequisites
* **Host Environment**: Windows with **MSYS2** (MINGW64 / UCRT64 shell)
* **Rust Toolchain**: Rust MSVC host toolchain (`x86_64-pc-windows-msvc`) with targets:
  * `x86_64-unknown-none`
  * `i686-unknown-none`
* **Required MSYS2 Packages**: Install `xorriso` via pacman:
  ```bash
  pacman -S mingw-w64-x86_64-xorriso
  ```
* **Tools**: `cargo` build system and `QEMU` emulator.

---

### 📂 Directory Setup

After cloning the repository, manually create the following directories inside `kernel_legacy/` and/or `kernel_uefi/` (*Note: `iso_root` is generated automatically by the build script during packaging*):

1. **`grub_binaries/`**:
   * For **`kernel_legacy`**: Include the GRUB `i386-pc` binaries.
   * For **`kernel_uefi`**: Include the GRUB `x86_64-efi` binaries along with the required EFI bootloader file.
2. **`initrd_root/`**:
   * Add `splash.bmp` (Image format: **32bpp**, resolution **800x600** with pre-rendered warning & credits overlay text).
   * Add a PSF font file renamed to **`font.psf`**. You can download a compatible 8x16 PSF font from the [ercanersoy/PSF-Fonts](https://github.com/ercanersoy/PSF-Fonts) repository (e.g., `default8x16.psf`) and rename it to `font.psf`:
     ```bash
     curl -L -o initrd_root/font.psf [https://raw.githubusercontent.com/ercanersoy/PSF-Fonts/master/default8x16.psf](https://raw.githubusercontent.com/ercanersoy/PSF-Fonts/master/default8x16.psf)
     ```

---

## 🚀 Execution Commands

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

---

## 📝 Yanase-Lang (`.yl`) Scripting & Compiler

OpenYanase Kernel v2 features a custom scripting language (**Yanase-Lang**) that compiles into YBC bytecode to run inside the kernel VM.

### 1. Syntax Overview (`example.yl`)

```swift
// Module imports
import utils.math;
import graphics.window as win;

// Class declaration (supports single inheritance)
class Shape {
    x;
    y;

    fn move(dx, dy) {
        x = x + dx;
        y = y + dy;
    }
}

class Circle extends Shape {
    radius;

    fn area() {
        return radius * radius * 3;
    }
}

// Main entry point
fn main() {
    let width = 800;
    let height = 600;

    let counter = 0;
    while (counter < 10) {
        if (counter == 5) {
            print("Halfway completed!");
        } else {
            print("Processing...");
        }
        counter = counter + 1;
    }

    // Built-in kernel syscalls
    draw_rect(0, 0, width, height);
    sleep(1000);

    return 0;
}
```

### 2. Built-in Syscalls
* `print(msg)`
* `draw_rect(x, y, w, h)`
* `get_tick()`
* `sleep(ms)`
* `exit()`

### 3. Compiling `.yl` to Bytecode (`.ybc`)

```bash
cd tools/ybc_compiler

# Basic compilation
cargo run -- <entry.yl> <output.ybc>

# Compile with extra module search directories
cargo run -- <entry.yl> <output.ybc> --include=dir1,dir2
```

---

## 📜 License & Credits
Developed and maintained by **PmgTeam**.
