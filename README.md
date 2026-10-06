<p align="center">
  <img src="banner.png" alt="OpenYanase Kernel v2 Logo" width="160px" height="160px">
</p>

<h1 align="center">OpenYanase Kernel v2</h1>

<p align="center">
  <img src="https://img.shields.io/badge/language-Rust-orange.svg?style=flat-square&logo=rust" alt="Language: Rust">
  <img src="https://img.shields.io/badge/architecture-x86__64-blue.svg?style=flat-square" alt="Architecture: x86_64">
  <img src="https://img.shields.io/badge/boot-Multiboot2%20%2F%20GRUB-green.svg?style=flat-square" alt="Boot: Multiboot2 / GRUB">
  <img src="https://img.shields.io/badge/kernel-no__std-red.svg?style=flat-square" alt="no_std">
  <img src="https://img.shields.io/badge/compiler-ybcc-yellow.svg?style=flat-square" alt="Compiler: ybcc">
  <img src="https://img.shields.io/badge/license-GPLv3-3DA639.svg?style=flat-square&logo=gnu" alt="License: GPLv3">
</p>

<p align="center">
  <b>A 64-bit monolithic microkernel-style OS written in pure Rust — featuring a Zero Trust capability model, Ring 1 driver isolation, dual VM runtimes (YBC + KVM), a custom Yanase-Lang compiler, and a full graphics stack.</b>
</p>

---

## ✨ Highlights

- 🦀 **100% Rust `#![no_std]`** kernel — no binary blobs, all assembly via `global_asm!`
- 🔐 **Zero Trust capability model** — every entity holds a `CapToken` minted by the kernel; tokens cannot be forged or escalated
- 🛡️ **Ring 1 driver isolation** — drivers run at CPL=1; all I/O port access is routed through `int 0x81` for kernel-side whitelisting
- 🧠 **Dual VM**: `YbcVm` for apps (stack-based bytecode) + `KernelVm` for drivers (capability-gated)
- 🧰 **Custom compiler (`ybcc`)** — lexer / parser / AST / resolver / codegen pipeline that compiles Yanase-Lang (`.yl`) into YBC bytecode (`.ybc`) or packed `.abp` packages
- 🖼️ **Graphics stack**: double-buffered `Surface` with `rep stosd`, ANSI truecolor console, PSF1/PSF2/YFN1 antialiased fonts, BMP decoder, window manager
- 💥 **Panic overlay on framebuffer** — dumps all GPRs/CRs/stack to screen when running in graphics mode
- ⚡ **ACPI S5 shutdown** + 5 fallback reset paths (ACPI reset, PCI 0xCF9, PS/2, port 0x92, triple fault)
- 🧩 **ABP package format** — tar + `manifest.txt` + `main.ybc` for apps, drivers, and daemons

---

## 🏗 Architecture

### Execution Model — 3 Layers, 3 Address Spaces

| Layer | VM | Ring | Syscall gate | Purpose |
|-------|-----|------|--------------|---------|
| **App** | `YbcVm` (`ybc_vm.rs`) | 0 | `int 0x80` (DPL=3) | Userland `.abp` apps |
| **Driver** | `KernelVm` (`kvm.rs`) | 1 (CPL=1) | `int 0x81` (DPL=1) | Isolated `.drv` drivers |
| **Daemon** | `YbcVm` persistent | 0 | `int 0x80` | Background services @ 100 Hz |

### Boot Flow

```
_start (32-bit) → save EAX/EBX → A20 gate
  → zero page tables → identity map 4 GB (2 MB huge pages)
  → PAE + CR3 + EFER.LME → GDT → CR0.PG
  → long_mode_start: enable SSE/FPU (required for bare metal)
  → kmain(magic, mb_info)
     ├─ serial + banner
     ├─ heap (512 KB bump allocator in BSS)
     ├─ Multiboot2 magic check
     ├─ GOP framebuffer detect
     ├─ IDT + GDT + TSS
     ├─ int 0x80 (DPL=3) + int 0x81 (DPL=1)
     ├─ PIT @ 1000 Hz / PS/2 mouse / PS/2 keyboard
     ├─ STI
     ├─ Multiboot tag walk (initrd module + memory map)
     ├─ Font bake (YFN1 → PSF2 → PSF1 → fallback bitmap)
     ├─ Boot text overlay → splash BMP (~945 ms)
     ├─ Console + GFX init
     ├─ ACPI parse (RSDP → FADT → DSDT → SLP_TYPA/B)
     ├─ VFS mount: ram0:// (initrd RO) + ram1:// (ramfs RW)
     ├─ Driver autoload (globalsys/drivers/*.drv)
     ├─ Daemon autoload (globalsys/services/*.abp)
     ├─ Init script (/globalsys/init.rc)
     └─ Prompt + main loop (100 Hz daemon/driver tick, 60 Hz gfx)
```

---

## 🔐 Zero Trust Capability Model

The security centerpiece of v2. Tokens are **minted only by the kernel**, revoked on unload, max 32 slots.

| Preset | I/O port range | Permissions | Steps/slice |
|--------|----------------|-------------|-------------|
| `APP_DEFAULT` | — | draw + print | 5000 |
| `DRIVER_PS2` | `0x60–0x64` | io_port + irq | 2000 |
| `DRIVER_TIMER` | `0x40–0x43` | io_port + irq | 1000 |
| `DRIVER_BLOCK` | `0x1F0–0x1F7`, `0x170–0x177` | io_port + irq | 3000 |

Drivers that trip the watchdog **3 times → Faulted → auto-unload**. The IRQ routing table allows only one driver to hold a given IRQ.

---

## 📦 Package Format — ABP

```
myapp.abp (tar)
├── manifest.txt   # "name=MyApp\nentry=main.ybc\nheap=65536\n"
└── main.ybc       # bytecode
```

Autoload scan:

- `globalsys/drivers/*.drv` → `KernelVm` (Ring 1)
- `globalsys/services/*.abp` → daemon (persistent `YbcVm`)
- `globalsys/*.abp`, `apps/*.abp`, `globalsys/apps/*.abp` → app (`YbcVm` run-to-completion)

---

## 🧠 YBC Bytecode VM

Stack-based VM implemented in the kernel (`src/ybc_vm.rs`) and mirrored by the compiler's codegen (`tools/ybc_compiler/src/codegen.rs`). Opcode table is identical on both sides.

| Opcode | Value | Operand | Description |
|--------|-------|---------|-------------|
| `Nop` | 0 | — | No-op |
| `PushInt` | 1 | `i64` | Push integer literal |
| `PushStr` | 2 | `u16` | Push interned string (offset into string pool) |
| `Pop` | 3 | — | Discard top of stack |
| `Add` / `Sub` / `Mul` / `Div` | 4–7 | — | Arithmetic (wrapping; div-by-zero faults) |
| `Lt` / `Gt` / `Eq` | 8–10 | — | Comparison → 0/1 |
| `Not` | 11 | — | Logical NOT |
| `JmpIfFalse` | 12 | `u32` | Conditional jump (absolute) |
| `Jmp` | 13 | `u32` | Unconditional jump |
| `CallSys` | 14 | `u16`, `u8` | syscall ID + argc (≤ 5) |
| `LoadLocal` / `StoreLocal` | 15–16 | `u8` | 32 local slots |
| `Dup` | 17 | — | Duplicate top |
| `Halt` | 18 | — | Terminate VM, top of stack = exit code |
| `NewObject` | 19 | `u16` | Allocate instance with class ID |
| `GetField` / `SetField` | 20–21 | `u8` | Field slot (16 max) |
| `CallMethod` | 22 | `u32`, `u8` | target pc + argc |
| `Ret` | 23 | — | Return from method |
| `NewArray` | 24 | — | Pop size, push handle |
| `GetIndex` / `SetIndex` | 25–26 | — | Array or buffer access |
| `ArrayLen` | 27 | — | Push length |

Additional runtime limits:

- **256-element** array / buffer max
- **32-object** pool per VM instance
- **16-frame** call stack, per-frame **32 locals**
- **Interned string pool** — 1024 slots + 512-entry direct cache

---

## 🧰 Yanase-Lang & `ybcc` Compiler

`tools/ybc_compiler/` is a full compiler for **Yanase-Lang** (`.yl`), written in pure Rust on the host (not `no_std`). Binary name: **`ybcc`**.

### Compiler Pipeline

```
.yl source
   │
   ├─ lexer.rs      ── tokens  (keywords: class, extends, fn, let, if, else,
   │                            while, return, import, as, true, false, new,
   │                            package, break, continue)
   ├─ parser.rs     ── AST     (recursive descent, precedence:
   │                            or > and > cmp > add > mul > unary > postfix)
   ├─ module.rs     ── resolve imports across search roots
   ├─ resolver.rs   ── merge class inheritance chains, qualify names
   └─ codegen.rs    ── emit bytecode (matches kernel's ybc_vm.rs opcodes)
                       │
                       ├─ .ybc  (raw bytecode, no -pack)
                       └─ .abp  (tar + manifest.txt + main.ybc, with -pack)
```

### Language Features

- **Package declaration**: `package my.app;`
- **Imports with alias**: `import utils.math;` / `import graphics.window as win;`
- **Single inheritance**: `class Circle extends Shape { ... }`
- **Methods** (with implicit `this` in slot 0) and **fields**
- **Statements**: `let`, `if/else` (and `else if` chains), `while`, `break`, `continue`, `return`
- **Expressions**:
  - Integer literals (decimal + `0x` hex), string literals (with `\n`)
  - Array literals `[a, b, c]`, index access `arr[i]`, `arr[i] = v`
  - Field access `obj.field`, field assign `obj.field = v`
  - Method calls `obj.method(args)`
  - Instantiation `new ClassName()`
  - BinOps: `+ - * / < > <= >= == != && ||`, unary `-`, unary `!`
- **Raw syscall escape hatch**: `syscall(<id>, args...)`
- **Short-circuit `&&` / `||`** (compiled via `Dup` + `JmpIfFalse` + `Pop`)
- **`<=` / `>=`** desugared to `!(a > b)` / `!(a < b)`

### Example (`hello.yl`)

```swift
package demo.hello;

import utils.math;

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

fn main() {
    let width = 800;
    let height = 600;

    gfx.init();
    gfx.clear(0, 0);
    gfx.fill_rect(0, 0, width, height, 0xFF1E90FF);

    let counter = 0;
    while (counter < 10) {
        if (counter == 5) {
            io.write_line("Halfway completed!");
        } else {
            io.write_line("Processing...");
        }
        counter = counter + 1;
    }

    let c = new Circle();
    c.radius = 32;
    let area = c.area();
    io.write_line("Circle area computed");

    gfx.flush();
    time.sleep_ms(1000);

    return 0;
}
```

### Standard Library (excerpt)

All stdlib functions are mapped to kernel syscalls in `tools/ybc_compiler/src/stdlib.rs`.

| Module | Functions |
|--------|-----------|
| `io` | `write_line`, `println`, `write_str`, `write`, `write_int`, `write_char`, `read_char`, `read_key`, `wait_key`, `read_line`, `write_port`, `read_port` |
| `console` | `clear`, `backspace` |
| `gfx` | `init`, `paint_block`, `clear`, `draw_pixel`, `draw_line`, `draw_rect`, `fill_rect`, `draw_char`, `draw_str`, `draw_text`, `flush`, `get_mouse_x`, `get_mouse_y`, `is_mouse_pressed`, `draw_cursor` |
| `screen` | `width`, `height`, `flush` |
| `time` | `now_ticks`, `now_secs`, `sleep_ms`, `sleep` |
| `proc` / `process` | `pause_for`, `terminate`, `self_id`, `get_pid`, `get_uptime`, `get_memory`, `run`, `exit` |
| `fs` | `list_dir`, `read_file`, `cd`, `get_cwd`, `is_dir` |
| `userfs` | `read_file`, `write_file`, `delete_file`, `exists` |
| `session` | `get`, `set`, `clear` |
| `ipc` | `create_port`, `send`, `recv` |
| `system` | `exit`, `pid`, `width`, `height`, `beep`, `exec`, `exec_arr`, `reboot`, `shutdown` |
| `daemon` | `kill`, `find`, `send`, `recv`, `msg_kind`, `msg_a`, `msg_b`, `msg_c`, `reply`, `self_pid` |

### Compiling

```bash
cd tools/ybc_compiler
cargo build --release

# Raw bytecode output
./target/release/ybcc hello.yl hello.ybc

# Packed .abp package (tar + manifest.txt + main.ybc)
./target/release/ybcc hello.yl hello.abp -pack --name=Hello

# Extra module search roots
./target/release/ybcc hello.yl hello.ybc --include=lib,src,stdlib
```

| Flag | Description |
|------|-------------|
| `<entry.yl> <output>` | Entry file and output path |
| `--include=dir1,dir2` | Extra module search roots (defaults to entry file's directory) |
| `-pack` | Produce `.abp` instead of raw `.ybc` |
| `--name=AppName` | App name for `manifest.txt` (only with `-pack`) |

---

## 🖥️ Graphics Stack

- **Double buffer** `Surface` using `rep stosd` for fills and `copy_nonoverlapping` for blits
- **Console** — ANSI-aware: SGR (1/22/30–37/39/40–47/49/90–97/100–107 + `38;5`/`48;5` + `38;2`/`48;2` truecolor), erase (J/K), cursor positioning (H/f), 500-line scrollback
- **Fonts**: PSF1 / PSF2 / **YFN1** (antialiased 8-bit alpha) + bitmap fallback
- **BMP**: 1/4/8/16/24/32 bpp, fast path for 24/32-bit with 16.16 fixed-point scaling
- **Window manager**: two demo windows, drag & drop, close button, taskbar with clock + FPS, shadows, active/inactive state
- **Mouse cursor** 12×19 with small-region restore (no full-buffer copy per frame)

---

## 💾 Storage / VFS

- **`ram0://`** — InitrdFS, read-only, TAR parsed from the Multiboot module
- **`ram1://`** — static RamDiskFs (16 files + 8 directories)
- **RamFs** (`ramfs.rs`) — 8 files × 256 bytes, incremental `write_byte`, used for shell config
- **Path resolver** supports `scheme://`, `..`, `.`

---

## 🎛️ Syscall Interface

### `int 0x80` (DPL=3) — Userland

- **I/O & console**: print, println, write_char, console_clear, console_backspace, console_ctl
- **Graphics**: draw_rect, gfx_clear, gfx_draw_text, gfx_flush, gfx_draw_image (BMP)
- **Time**: get_tick, get_time, sleep
- **Process**: get_pid, exit, exec (launcher whitelist)
- **Mouse**: get_mouse_x/y, is_mouse_pressed
- **Keyboard**: key_read (non-blocking), read_key_blocking
- **Userfs (400–403)**: config read / write_byte / clear / size
- **Session (500–502)**: get / set / clear UID
- **Power**: shutdown (113), reboot (114)

### `int 0x81` (DPL=1) — Ring 1 Drivers

18 `R1_*` syscalls: EXIT, WRITE_BYTE, WRITE_STR, GET_TICKS, GET_PID, SLEEP_MS, IO_PORT, DRV_PRINT, DRV_DRAW, DRV_CLAIM_IRQ, DRV_WAIT_EV, DRV_POLL_EV, DRV_SEND_EV, DRV_UNREG, DRV_COUNT, BEEP, DRV_REGISTER, DRV_READY.

Ring 0 ⇄ Ring 1 trampoline via `iretq`; `IOPL` is cleared to forbid direct `in/out` from Ring 1.

---

## 🗂️ Repository Structure

```
kernel/
├── src/
│   ├── boot.rs              # Multiboot2 header + 32→64 long mode setup
│   ├── main.rs              # kmain, heap, VFS mount, main loop, panic handler
│   ├── idt.rs / gdt.rs      # 256-vector IDT, 9-slot GDT + TSS
│   ├── cpu.rs               # Port I/O, CR, MSR helpers
│   ├── console.rs           # ANSI console + scrollback
│   ├── serial.rs            # COM1
│   ├── timer.rs             # PIT 1000 Hz
│   ├── keyboard.rs / mouse.rs  # PS/2 with global_asm stubs
│   ├── acpi.rs              # RSDP/FADT/DSDT + shutdown/reboot
│   ├── gop.rs / bmp.rs      # Framebuffer + BMP decoder
│   ├── graphics/            # Surface, Font (PSF/YFN), Window, Gfx
│   ├── vfs.rs / initrd.rs / ramfs.rs  # Storage stack
│   ├── ybc.rs / ybc_vm.rs   # YBC bytecode + VM for apps
│   ├── kvm.rs / kvm_guard.rs  # Kernel VM for drivers + capability token
│   ├── ring.rs / ring1_handler.rs / ring1_vm.rs  # Ring 1 trampoline + dispatcher
│   ├── driver.rs / driver_loader.rs  # Driver manager + autoload
│   ├── daemon.rs            # Background service + mailbox IPC
│   ├── abp.rs               # ABP package runner
│   ├── process.rs           # Process table for apps
│   ├── session.rs / user.rs # Session UID + user context
│   ├── shell.rs / init.rs   # Shell built-ins + init.rc runner
│   └── log.rs               # Dual serial+TTY logging with prebuffer
│
└── tools/ybc_compiler/
    └── src/
        ├── main.rs          # CLI entry (`ybcc`), -pack / --name / --include
        ├── lexer.rs         # Tokenizer
        ├── ast.rs           # AST types
        ├── parser.rs        # Recursive descent parser
        ├── module.rs        # Import resolution
        ├── resolver.rs      # Class chain merge + name qualification
        ├── codegen.rs       # Bytecode emitter (matches ybc_vm.rs)
        ├── stdlib.rs        # Module → syscall ID mapping
        └── tar_writer.rs    # .abp packaging
```

---

## 🛠 Building & Setup

### Prerequisites

- **Host**: Windows with **MSYS2** (MINGW64 / UCRT64 shell), or Linux
- **Rust**: toolchain with the `x86_64-unknown-none` target
  ```bash
  rustup target add x86_64-unknown-none
  ```
- **xorriso** (ISO creation):
  ```bash
  # MSYS2
  pacman -S mingw-w64-x86_64-xorriso
  # Linux
  sudo apt install xorriso
  ```
- **GRUB** `x86_64-efi` binaries (place into `grub_binaries/`)
- **QEMU** to run

### Directory Setup

After cloning, create the following directories:

1. **`grub_binaries/`** — GRUB `x86_64-efi` binaries + the EFI bootloader file
2. **`initrd_root/`** — with the following files:
   - `splash.bmp` (32bpp, 800×600 with pre-rendered warning + credits text)
   - `font.psf` — e.g. `default8x16.psf` from [ercanersoy/PSF-Fonts](https://github.com/ercanersoy/PSF-Fonts)
     ```bash
     curl -L -o initrd_root/font.psf \
       https://raw.githubusercontent.com/ercanersoy/PSF-Fonts/master/default8x16.psf
     ```
   - (optional) `.abp` packages under `initrd_root/globalsys/apps/`
   - (optional) `.drv` drivers under `initrd_root/globalsys/drivers/`
   - (optional) `init.rc` under `initrd_root/globalsys/`

### Build & Run

```bash
cargo run
```

The build script produces `openyanase.iso` and boots it through QEMU.

### Build the Compiler

```bash
cd tools/ybc_compiler
cargo build --release
# binary: target/release/ybcc
```

---

## 🐚 Shell

Built-in commands:

| Command | Description |
|---------|-------------|
| `help` | Show command list |
| `clear` / `cls` | Clear the screen |
| `info` | System information |
| `gfx` | Enter graphics mode (demo) |
| `tty` | Return to console mode |
| `echo <text>` | Echo text |
| `version` | Kernel version |
| `history` | Command history (32 entries) |
| `meminfo` | Memory map information |
| `cd` / `ls` / `cat` / `pwd` | VFS navigation |
| `drivers` / `lsdrv` | List loaded drivers |
| `drvstate <pid>` | Show driver detail by PID |
| `unloaddrv <pid>` | Unload a driver |
| `panic [gfx]` | Trigger kernel panic (overlay test) |
| `reboot` / `shutdown` | Restart / power off |

Key bindings: **Tab** (completion), **Up/Down** (history), **Page Up/Dn** (console scrollback), **F1** (graphics mode), **F2** (console mode).

Init script `/globalsys/init.rc`:

```
# comment
sleep 500
echo "Hello from init.rc"
hello.abp
```

- Comments start with `#`
- `sleep <ms>` is clamped to 10 000 ms
- Max 64 lines
- All other lines are dispatched to `shell::execute`

---

## 🧩 End-to-End Workflow

```
1. Write hello.yl
       │
2. tools/ybc_compiler → ybcc hello.yl hello.abp -pack --name=Hello
       │
       ├─ lexer    → tokens
       ├─ parser   → AST
       ├─ resolver → merge class chains
       ├─ codegen  → YBC bytecode (magic 0x59424331)
       └─ tar_writer → .abp { manifest.txt, main.ybc }
       │
3. Drop hello.abp into initrd_root/globalsys/apps/
       │
4. Rebuild ISO → boot → shell
       │
5. > hello
       │
       ├─ shell finds .abp in initrd
       ├─ abp::run_abp_file → extract main.ybc
       ├─ process::spawn_ybc → copy bytecode into process slot
       ├─ YbcVm::new → parse header (magic / code_len / string_pool_len)
       └─ YbcVm::run → interpret opcodes, dispatch syscalls via int 0x80
```

---

## 📜 License

OpenYanase Kernel v2 is free software: you can redistribute it and/or modify it under the terms of the **GNU General Public License** as published by the Free Software Foundation, either **version 3 of the License**, or (at your option) any later version.

This program is distributed in the hope that it will be useful, but **WITHOUT ANY WARRANTY**; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.

You should have received a copy of the GNU General Public License along with this program. If not, see <https://www.gnu.org/licenses/gpl-3.0.html>.

- SPDX-License-Identifier: `GPL-3.0-or-later`
- Full text: [LICENSE](LICENSE) (or <https://www.gnu.org/licenses/gpl-3.0.txt>)

---

## 🙏 Credits

Developed and maintained by **PmgTeam** — (c) 2024 **PmgDev64** and contributors.
