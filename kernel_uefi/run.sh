#!/usr/bin/env bash
set -euo pipefail

KERNEL_BIN="$1"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

# ============================================
# 1. CHECK COMPILER
# ============================================
COMPILER_DIR="../tools/ybc_compiler"
COMPILER_BIN=""

if [ -f "$COMPILER_DIR/target/debug/ybcc.exe" ]; then
    COMPILER_BIN="$COMPILER_DIR/target/debug/ybcc.exe"
elif [ -f "$COMPILER_DIR/target/debug/ybcc" ]; then
    COMPILER_BIN="$COMPILER_DIR/target/debug/ybcc"
elif [ -f "$COMPILER_DIR/target/release/ybcc.exe" ]; then
    COMPILER_BIN="$COMPILER_DIR/target/release/ybcc.exe"
elif [ -f "$COMPILER_DIR/target/release/ybcc" ]; then
    COMPILER_BIN="$COMPILER_DIR/target/release/ybcc"
else
    echo "[!] Error: Compiler not found!"
    echo "[!] Please build it first:"
    echo "    cd ../tools/ybc_compiler"
    echo "    cargo build"
    exit 1
fi

echo "[+] Using compiler: $COMPILER_BIN"

# ============================================
# 2. APP LAYOUT
#    build_abp <ten> <file_nguon> <ten_ybc> <bat_buoc 1|0>
#      -> $BIN_DIR/<ten>.abp  (manifest.txt + main.ybc)
#    Hien co:
#      system : ../app/src/main.yl   (desktop)      - bat buoc
#      shell  : ../app/src/shell.yl  (shell app)    - tuy chon, thieu thi bo qua
# ============================================
APP_DIR="../app"
LIB_DIR="$APP_DIR/lib"
BIN_DIR="$APP_DIR/bin"
OPTIONAL_FLAG="--include=$LIB_DIR"

mkdir -p "$BIN_DIR"
BUILT_ABPS=()
BUILT_YBCS=()

build_abp() {
    local name="$1" src="$2" ybc_base="$3" required="$4"
    local ybc="$BIN_DIR/$ybc_base.ybc"
    local abp="$BIN_DIR/$name.abp"
    local tmp="abp_temp_$name"

    # ---- check source ----
    if [ ! -f "$src" ]; then
        if [ "$required" = "1" ]; then
            echo "[!] Error: Source file not found: $src"
            echo "[!] Please create your source file at: $src"
            echo ""
            echo "Example main.yl:"
            echo "fn main() {"
            echo '    print("Hello from OpenYanase!\n");'
            echo "}"
            exit 1
        fi
        echo "[-] Skip $name (no $src)"
        return 0
    fi

    # ---- compile ----
    echo "[+] Compiling $src -> $ybc..."
    rm -f "$ybc"
    "$COMPILER_BIN" "$src" "$ybc" "$OPTIONAL_FLAG"

    if [ ! -f "$ybc" ]; then
        echo "[!] Error: Compilation failed ($name)"
        exit 1
    fi
    echo "[+] Compilation successful: $ybc"

    # ---- create .abp ----
    echo "[+] Creating $abp..."
    rm -rf "$tmp"
    mkdir -p "$tmp"

    cat > "$tmp/manifest.txt" << EOF
name=$name
entry=main.ybc
heap=65536
EOF

    cp "$ybc" "$tmp/main.ybc"

    local abs_abp
    abs_abp="$(cd "$BIN_DIR" && pwd)/$name.abp"
    ( cd "$tmp" && tar --format=ustar -cf "$abs_abp" manifest.txt main.ybc )

    if [ ! -f "$abp" ]; then
        echo "[!] Error: Failed to create $name.abp"
        exit 1
    fi

    rm -rf "$tmp"
    echo "[+] Created: $abp"

    BUILT_ABPS+=("$abp")
    BUILT_YBCS+=("$ybc")
}

build_abp system "$APP_DIR/src/main.yl"  main  1
build_abp shell  "$APP_DIR/src/shell.yl" shell 0

# ============================================
# 5. UPDATE INITRD (giữ nguyên resource)
# ============================================
INITRD_ROOT="initrd_root"
mkdir -p "$INITRD_ROOT/globalsys"

LOOSE_YBCS=()
for f in "${BUILT_YBCS[@]}"; do
    cp "$f" "$INITRD_ROOT/globalsys/"
    LOOSE_YBCS+=("$INITRD_ROOT/globalsys/$(basename "$f")")
done
for f in "${BUILT_ABPS[@]}"; do
    cp "$f" "$INITRD_ROOT/globalsys/"
done

# init script (nếu có file init.rc cạnh run.sh thì dùng bản đó)
if [ -f "$ROOT/init.rc" ]; then
    cp "$ROOT/init.rc" "$INITRD_ROOT/globalsys/init.rc"
    echo "[+] init.rc -> initrd_root/globalsys/init.rc"
fi

echo "[+] Updated initrd_root with:"
for f in "${BUILT_YBCS[@]}" "${BUILT_ABPS[@]}"; do
    echo "    - $(basename "$f")"
done

# ============================================
# 6. PACK INITRD
# ============================================
ISO_ROOT="iso_root"
rm -rf "$ISO_ROOT"
mkdir -p "$ISO_ROOT/boot/grub"

echo "[+] Packing initrd_root -> $ISO_ROOT/boot/initrd.tar..."
tar --format=ustar -cf "$ISO_ROOT/boot/initrd.tar" -C "$INITRD_ROOT" .
HAS_INITRD=true

for f in "${LOOSE_YBCS[@]}"; do
    rm -f "$f"
done

# ============================================
# 7. COPY KERNEL
# ============================================
if [ ! -f "$KERNEL_BIN" ]; then
    echo "[!] Error: Kernel not found at $KERNEL_BIN"
    exit 1
fi
cp "$KERNEL_BIN" "$ISO_ROOT/boot/kernel.bin"

# ============================================
# 8. COPY GRUB MODULES
# ============================================
GRUB_DIR="grub_binaries"
if [ ! -d "$GRUB_DIR" ]; then
    echo "[!] Error: grub_binaries directory not found"
    exit 1
fi

X86_64_DIR=$(find "$GRUB_DIR" -type d -name "x86_64-efi" | head -n 1)
if [ -n "$X86_64_DIR" ]; then
    cp -r "$X86_64_DIR" "$ISO_ROOT/boot/grub/"
else
    echo "Error: Không tìm thấy thư mục x86_64-efi trong grub_binaries!" >&2
    exit 1
fi

find "$GRUB_DIR" -type f -name "*.pf2" -exec cp {} "$ISO_ROOT/boot/grub/" \; 2>/dev/null || true

# ============================================
# 9. CREATE GRUB.CFG
# ============================================
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

# ============================================
# 10. CREATE EFIBOOT.IMG
# ============================================
STAGING="efi_stage"
rm -rf "$STAGING"
mkdir -p "$STAGING/EFI/BOOT"
mkdir -p "$STAGING/boot/grub"

find "$GRUB_DIR" -type f \( -iname "*.efi" \) -exec cp {} "$STAGING/EFI/BOOT/" \;

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

cat > "$STAGING/EFI/BOOT/grub.cfg" << 'EOF'
search --no-floppy --set=root --file /boot/kernel.bin
set prefix=($root)/boot/grub
configfile /boot/grub/grub.cfg
EOF
cp "$STAGING/EFI/BOOT/grub.cfg" "$STAGING/boot/grub/grub.cfg"

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

# ============================================
# 11. CREATE ISO
# ============================================
ISO_FILE="openYanase.iso"
echo "[+] Packing clean ISO with xorriso ($ISO_FILE)..."

xorriso -as mkisofs -R -J -V "OPENYANASE" \
    -e efiboot.img -no-emul-boot \
    -o "$ISO_FILE" "$ISO_ROOT"

# ============================================
# 12. RUN QEMU
# ============================================
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