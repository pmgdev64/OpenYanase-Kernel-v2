#!/usr/bin/env bash
set -euo pipefail

KERNEL_BIN="$1"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

ISO_ROOT="iso_root"
rm -rf "$ISO_ROOT"
mkdir -p "$ISO_ROOT/boot/grub"

# 1. Copy Kernel vào iso_root/boot
cp "$KERNEL_BIN" "$ISO_ROOT/boot/kernel.bin"

# 2. Tạo initrd với font, splash và system.abp (Lua script)
HAS_INITRD=false
if [ -d "initrd_root" ]; then
    echo "[+] Packing initrd_root -> $ISO_ROOT/boot/initrd.tar..."
    tar --format=ustar -cf "$ISO_ROOT/boot/initrd.tar" -C initrd_root .
    HAS_INITRD=true
else
    # Nếu không có initrd_root, tạo mới với các file mẫu
    echo "[+] Creating initrd_root with sample files..."
    mkdir -p initrd_root
    
    # Tạo font.psf mẫu (nếu chưa có)
    if [ ! -f "initrd_root/font.psf" ]; then
        echo "[!] Warning: font.psf not found, creating dummy font..."
        # Tạo PSF1 header (cần có file thật, nhưng tạm thời bỏ qua)
        # Bạn nên copy font.psf thật vào initrd_root/
    fi
    
    # Tạo system.abp (TAR chứa boot.lua)
    echo "[+] Creating system.abp..."
    mkdir -p abp_temp
    cat > abp_temp/boot.lua << 'EOF'
-- OpenYanase Boot Script
println("╔═══════════════════════════════════════════╗")
println("║     OpenYanase Lua Boot Script           ║")
println("╚═══════════════════════════════════════════╝")

println("Kernel: " .. get_kernel_version())
local w, h = get_resolution()
println("Display: " .. w .. "x" .. h)
println("Boot time: " .. get_time() .. " ms")

-- Countdown demo
println("\nCountdown:")
for i = 5, 1, -1 do
    println("  " .. i .. "...")
    sleep(500)
end
println("  GO!")
sleep(500)

-- Keyboard test
println("\nPress any key to continue...")
local key = read_key()
while key == nil do
    key = read_key()
    sleep(16)
end
println("You pressed: " .. key)

-- Simple calculator
println("\nSimple Calculator:")
println("  5 + 3 = " .. (5 + 3))
println("  10 * 2 = " .. (10 * 2))
println("  100 / 4 = " .. (100 / 4))

println("\nBoot script completed!")
println("═══════════════════════════════════════════")
EOF

    # Đóng gói vào TAR
    tar --format=ustar -cf "initrd_root/system.abp" -C abp_temp .
    rm -rf abp_temp
    
    # Pack initrd
    echo "[+] Packing initrd_root -> $ISO_ROOT/boot/initrd.tar..."
    tar --format=ustar -cf "$ISO_ROOT/boot/initrd.tar" -C initrd_root .
    HAS_INITRD=true
fi

# 3. Tự động tìm và copy thư mục module x86_64-efi vào đĩa ISO
X86_64_DIR=$(find grub_binaries -type d -name "x86_64-efi" | head -n 1)
if [ -n "$X86_64_DIR" ]; then
    cp -r "$X86_64_DIR" "$ISO_ROOT/boot/grub/"
else
    echo "Error: Không tìm thấy thư mục x86_64-efi trong grub_binaries!" >&2
    exit 1
fi

# Copy các file font .pf2 nếu có
find grub_binaries -type f -name "*.pf2" -exec cp {} "$ISO_ROOT/boot/grub/" \; 2>/dev/null || true

# 4. Tạo file grub.cfg CHÍNH trên iso_root
cat > "$ISO_ROOT/boot/grub/grub.cfg" <<EOF
set timeout=0
set default=0

insmod all_video

menuentry "openYanase Kernel (UEFI ISO)" {
    multiboot2 /boot/kernel.bin
$(if [ "$HAS_INITRD" = true ]; then echo "    module2 /boot/initrd.tar"; fi)
    boot
}
EOF

# 5. Gom các file .efi vào staging để đóng gói efiboot.img
STAGING="efi_stage"
rm -rf "$STAGING"
mkdir -p "$STAGING/EFI/BOOT"
mkdir -p "$STAGING/boot/grub"

if [ -d "grub_binaries" ]; then
    # Tìm và copy toàn bộ file .efi vào EFI/BOOT
    find grub_binaries -type f \( -iname "*.efi" \) -exec cp {} "$STAGING/EFI/BOOT/" \;

    cd "$STAGING/EFI/BOOT"
    BOOT_FILE=""
    HAS_GRUB=false

    for f in *; do
        fname="${f,,}"
        if [[ "$fname" == "bootx64.efi" ]]; then
            BOOT_FILE="$f"
        elif [[ "$fname" == "grubx64.efi" ]]; then
            HAS_GRUB=true
        fi
    done

    if [ -n "$BOOT_FILE" ]; then
        if [ "$HAS_GRUB" = false ]; then
            cp "$BOOT_FILE" "grubx64.efi"
        fi
    else
        echo "Error: Không tìm thấy file bootx64.efi trong grub_binaries!" >&2
        exit 1
    fi
    cd "$ROOT"
else
    echo "Error: Thư mục grub_binaries không tồn tại!" >&2
    exit 1
fi

# Tạo grub.cfg mồi (stub) cho efiboot.img
cat > "$STAGING/EFI/BOOT/grub.cfg" << 'EOF'
search --no-floppy --set=root --file /boot/kernel.bin
set prefix=($root)/boot/grub
configfile /boot/grub/grub.cfg
EOF
cp "$STAGING/EFI/BOOT/grub.cfg" "$STAGING/boot/grub/grub.cfg"

# 6. Tính toán chính xác dung lượng efiboot.img tránh đĩa bị đầy
STAGE_SIZE_KB=$(du -sk "$STAGING" | cut -f1)
IMG_SIZE_MB=$(( (STAGE_SIZE_KB / 1024) + 2 ))
if [ "$IMG_SIZE_MB" -lt 3 ]; then
    IMG_SIZE_MB=3
fi

EFI_IMG="efiboot.img"
rm -f "$EFI_IMG" "$ISO_ROOT/$EFI_IMG"

if ! command -v mformat >/dev/null 2>&1 || ! command -v mcopy >/dev/null 2>&1; then
    echo "[!] Lỗi: Chưa cài 'mtools'. Trên MSYS2 hãy chạy: pacman -S mtools" >&2
    exit 1
fi

echo "[+] Creating ${IMG_SIZE_MB}MB efiboot.img..."
dd if=/dev/zero of="$EFI_IMG" bs=1M count="$IMG_SIZE_MB" status=none
mformat -i "$EFI_IMG" -v "EFIBOOT" ::
mcopy -s -i "$EFI_IMG" "$STAGING"/* ::/

mv "$EFI_IMG" "$ISO_ROOT/efiboot.img"
rm -rf "$STAGING"

# 7. Dùng xorriso tạo ISO
ISO_FILE="openYanase.iso"
echo "[+] Packing clean ISO with xorriso ($ISO_FILE)..."

xorriso -as mkisofs -R -J -V "OPENYANASE" \
    -e efiboot.img -no-emul-boot \
    -o "$ISO_FILE" "$ISO_ROOT"

# 8. Cấu hình OVMF & Chạy QEMU
OVMF_PATH="OVMF.fd"
if [ ! -f "$OVMF_PATH" ]; then
    if [ -f "/mingw64/share/qemu/edk2-x86-64-secure-code.fd" ]; then
        OVMF_PATH="/mingw64/share/qemu/edk2-x86-64-secure-code.fd"
    elif [ -f "/usr/share/qemu/OVMF.fd" ]; then
        OVMF_PATH="/usr/share/qemu/OVMF.fd"
    elif [ -f "/usr/share/ovmf/x64/OVMF.fd" ]; then
        OVMF_PATH="/usr/share/ovmf/x64/OVMF.fd"
    fi
fi

echo "[+] Launching QEMU via ISO CDROM..."
qemu-system-x86_64 \
    -bios "$OVMF_PATH" \
    -cdrom "$ISO_FILE" \
    -m 2048M \
    -serial stdio \
    -vga std