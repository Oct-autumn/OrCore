#!/bin/bash

# Check running param
gdb_flag=false

for i in "$@"; do
  case $i in
  "--wait-gdb")
    gdb_flag=true
    ;;
  *)
    printf "--wait-gdb  Start QEMU and wait for GDB request.\n"
    return 1
    ;;
  esac
done

# Check running path
if [ ! -e "./Cargo.toml" ]; then
  echo "[FATAL] Please run under the Cargo dir."
  return 1
fi

# Build os kernel
echo "[INFO] Building or-os kernel"
cargo build --release

result=$?
if [ $result != 0 ]; then
  echo "[FATAL] Failed to build kernel. Return with code $result"
  return 2
fi

# Make os kernel image
echo "[INFO] Making kernel image"
rust-objcopy --strip-all target/riscv64gc-unknown-none-elf/release/os -O binary target/riscv64gc-unknown-none-elf/release/os.bin

result=$?
if [ $result != 0 ]; then
  echo "[FATAL] Failed to make kernel image. Return with code $result"
  return 2
fi

# Start QEMU and load kernel image
echo "[INFO] Starting QEMU"
if [ $gdb_flag == true ]; then
  echo "[WARN] Start at GDB mode"
  qemu-system-riscv64 \
    -machine virt \
    -nographic \
    -bios ../bootloader/rustsbi-qemu.bin \
    -device loader,file=target/riscv64gc-unknown-none-elf/release/os.bin,addr=0x80200000 \
    -s -S
elif [ $gdb_flag == false ]; then
  qemu-system-riscv64 \
    -machine virt \
    -nographic \
    -bios ../bootloader/rustsbi-qemu.bin \
    -device loader,file=target/riscv64gc-unknown-none-elf/release/os.bin,addr=0x80200000
fi
