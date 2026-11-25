#!/bin/sh
# Builds Pulse for NovaOS (i686 ring-3 ELF32 executable)
# and installs it into NovaOS fixtures as PULSE.ELF, MONITOR.ELF,
# and generates PULSE.PKG and MONITOR.PKG.
set -e
cd "$(dirname "$0")"

NOVAOS_ROOT="${NOVAOS_DIR:-/home/firoze/projects/others/novaos}"
if [ ! -d "$NOVAOS_ROOT" ]; then
    echo "Error: NovaOS directory not found at $NOVAOS_ROOT" >&2
    exit 1
fi

RUST_SYSROOT="$NOVAOS_ROOT/tools/rust-sysroot"
LIBRUST="$RUST_SYSROOT/sysroot/lib/rustlib/i686-novaos/lib"
if [ ! -f "$LIBRUST/libcore.rlib" ]; then
    echo "Building NovaOS Rust sysroot..."
    "$RUST_SYSROOT/build-sysroot.sh"
fi

LIBC="$NOVAOS_ROOT/userland/libc"
CC="gcc -m32 -ffreestanding -fno-stack-protector -fno-pie -Wall -Wextra -I$LIBC/include"

nasm -f elf32 "$LIBC/crt0.asm" -o crt0.o
$CC -c "$LIBC/syscall.c" -o syscall.o
$CC -c "$LIBC/stdlib.c" -o stdlib.o
$CC -c "$LIBC/string.c" -o string.o

echo "Compiling Pulse System Monitor for NovaOS (pure Rust i686)..."
RUSTC_CMD="${RUSTC:-rustc}"
BOOTSTRAP_ENV="RUSTC_BOOTSTRAP=1"

env $BOOTSTRAP_ENV $RUSTC_CMD --edition 2021 -Z unstable-options \
    --target "$RUST_SYSROOT/i686-novaos.json" \
    --crate-type bin -C panic=abort -C opt-level=2 \
    --emit obj=pulse_main.o \
    -L "$LIBRUST" \
    --extern core="$LIBRUST/libcore.rlib" \
    --extern compiler_builtins="$LIBRUST/libcompiler_builtins.rlib" \
    -C link-dead-code=no \
    pulse.rs

ld -m elf_i386 -Ttext=0x08048000 --entry=_start -static \
    -o pulse.elf crt0.o pulse_main.o syscall.o string.o stdlib.o \
    "$LIBRUST/libcore.rlib" "$LIBRUST/libcompiler_builtins.rlib"

mkdir -p "$NOVAOS_ROOT/tools/fixtures"
cp pulse.elf "$NOVAOS_ROOT/tools/fixtures/PULSE.ELF"
cp pulse.elf "$NOVAOS_ROOT/tools/fixtures/MONITOR.ELF"

# Generate Package Manifests
cat << 'EOF' > "$NOVAOS_ROOT/tools/fixtures/PULSE.PKG"
[package]
id = "org.novaos.pulse"
name = "Pulse System Monitor"
version = "1.0.0"
category = "system"
description = "Sovereign real-time process manager, multi-core CPU, memory, and filesystem monitor"
exec = "PULSE.ELF"
icon = "monitor"
EOF

cp "$NOVAOS_ROOT/tools/fixtures/PULSE.PKG" "$NOVAOS_ROOT/tools/fixtures/MONITOR.PKG"

echo "Built and deployed tools/fixtures/PULSE.ELF, MONITOR.ELF, PULSE.PKG, MONITOR.PKG"
