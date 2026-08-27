#!/usr/bin/env bash
set -euo pipefail

# Cargo invokes this as: runner <path-to-kernel-binary> [args...]
KERNEL_BIN="$1"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

ISO_ROOT="iso_root"
BOOT_DIR="$ISO_ROOT/boot"
GRUB_DIR="$BOOT_DIR/grub"
GRUB_I386_DIR="$GRUB_DIR/i386-pc"

# 1. Tạo cây thư mục cho GRUB
mkdir -p "$GRUB_I386_DIR"

# 2. Copy toàn bộ module của GRUB vào ISO
cp -r grub_binaries/i386-pc/* "$GRUB_I386_DIR/"

SOURCES_DIR="app/src/desktop.ys"
OUTPUT_NAME="desktop.abp"

DRIVER_SOURCES_DIR="driver/src/driver.yd"
DRIVER_OUTPUT_NAME="beep.drv"

# Compile app nếu có - DÙNG app/compiler.py
if [ -f "app/compiler.py" ] && [ -f "$SOURCES_DIR" ]; then
    echo "Compiling app..."
    python3 app/compiler.py "$SOURCES_DIR" "initrd_root/$OUTPUT_NAME"
fi

# Compile driver nếu có - DÙNG driver/driver_compiler.py (QUAN TRỌNG)
if [ -f "driver/driver_compiler.py" ] && [ -f "$DRIVER_SOURCES_DIR" ]; then
    echo "Compiling driver..."
    # SỬA: dùng driver_compiler.py chứ không phải app/compiler.py
    python3 driver/driver_compiler.py "$DRIVER_SOURCES_DIR" "initrd_root/$DRIVER_OUTPUT_NAME"
fi

# 3. Copy Kernel binary
cp "$KERNEL_BIN" "$BOOT_DIR/kernel"

# 4. Đóng gói thư mục initrd_root thành initrd.tar
if [ -d "initrd_root" ]; then
    echo "Packing initrd.tar from initrd_root/..."
    if [ -f "splash.bmp" ]; then
        cp splash.bmp initrd_root/splash.bmp
    fi
    if [ -f "splash_800x600.bmp" ]; then
        cp splash_800x600.bmp initrd_root/splash_800x600.bmp
    fi
    if [ -f "splash_1024x768.bmp" ]; then
        cp splash_1024x768.bmp initrd_root/splash_1024x768.bmp
    fi
    tar -cvf "$BOOT_DIR/initrd.tar" -C initrd_root .
fi

# 5. Copy GRUB splash image (nếu có)
if [ -f "grub_splash.bmp" ]; then
    cp grub_splash.bmp "$BOOT_DIR/grub_splash.bmp"
fi

# 6. Tạo file cấu hình grub.cfg
cat > "$GRUB_DIR/grub.cfg" <<'EOF'
set timeout=3
set default=0

insmod all_video
insmod vbe
insmod gfxterm
insmod png
insmod jpeg
insmod bitmap

set gfxmode=800x600x32,1024x768x32,auto
set gfxpayload=keep

if [ -f /boot/grub_splash.bmp ]; then
    insmod bitmap
    background_image /boot/grub_splash.bmp
fi

terminal_output gfxterm
terminal_input console

set color_normal=white/black
set color_highlight=black/light-gray

menuentry "OpenYanase Kernel" {
    multiboot2 /boot/kernel
    module2 /boot/initrd.tar "initrd"
    boot
}

menuentry "OpenYanase Kernel (Verbose)" {
    multiboot2 /boot/kernel
    module2 /boot/initrd.tar "initrd"
    boot
}

menuentry "System Information" {
    echo "OpenYanase Kernel v0.1"
    echo "Built with Rust"
    echo "Press any key to continue..."
    read
}

menuentry "Reboot" {
    reboot
}

menuentry "Shutdown" {
    halt
}
EOF

# 7. Tạo GRUB splash image nếu chưa có
if [ ! -f "grub_splash.bmp" ] && [ -f "splash.bmp" ]; then
    echo "Converting splash.bmp to GRUB compatible format..."
    if command -v convert &> /dev/null; then
        convert splash.bmp -resize 800x600 -colors 256 grub_splash.bmp
    else
        cp splash.bmp grub_splash.bmp
        echo "Warning: ImageMagick not found. GRUB may not display splash correctly."
    fi
fi

# 8. Đóng gói ISO bằng xorriso
echo "Creating ISO..."
xorriso/xorriso.exe -as mkisofs \
    -b boot/grub/i386-pc/eltorito.img \
    -no-emul-boot -boot-load-size 4 -boot-info-table \
    -V "OpenYanaseKernel" \
    -preparer "OpenYanase OS" \
    -publisher "OpenYanase" \
    -o kernel.iso \
    iso_root

# 9. Kiểm tra ISO
if [ -f "kernel.iso" ]; then
    echo "ISO created successfully: kernel.iso"
    ls -lh kernel.iso
    
    echo ""
    echo "ISO contents:"
    xorriso/xorriso.exe -indev kernel.iso -ls
fi

qemu-system-i386 \
    -cdrom kernel.iso \
    -serial stdio \
    -m 256M \
    -audiodev dsound,id=audio0 \
    -machine pcspk-audiodev=audio0 \
    -device AC97,audiodev=audio0