#!/usr/bin/env bash
set -euo pipefail

PROFILE="${1:-debug}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

cargo build --target-dir target $( [ "$PROFILE" = "release" ] && echo "--release" )

KERNEL_BIN="target/i686-unknown-none/$PROFILE/kernel"
ISO_ROOT="iso_root"
BOOT_DIR="$ISO_ROOT/boot"
ISOLINUX_DIR="$ISO_ROOT/isolinux"

mkdir -p "$BOOT_DIR" "$ISOLINUX_DIR"
cp syslinux_binaries/*.bin syslinux_binaries/*.c32 "$ISOLINUX_DIR/"
cp "$KERNEL_BIN" "$BOOT_DIR/kernel"

cat > "$ISOLINUX_DIR/isolinux.cfg" <<'EOF'
DEFAULT kernel
LABEL kernel
    KERNEL mboot.c32
    APPEND /boot/kernel
EOF

xorriso/xorriso.exe -as mkisofs \
    -b isolinux/isolinux.bin \
    -c isolinux/boot.cat \
    -no-emul-boot -boot-load-size 4 -boot-info-table \
    -o kernel.iso \
    iso_root

echo "Built $ROOT/kernel.iso"