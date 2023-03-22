#!/bin/bash

# Check running param
gdb_flag=false

for i in "$@"; do
  case $i in
  "--wait-gdb")
    gdb_flag=true
    ;;
  *)
    echo "args:"
    echo "    --wait-gdb  Start QEMU and wait for GDB request."
    exit 1
    ;;
  esac
done

# Check running path
if [ ! -e "./scripts" ]; then
  echo "[ERROR] Please run under the root dir."
  return 1
fi

# Build os kernel
echo "[INFO] Building or-os kernel"
cargo build --release

result=$?
if [ $result != 0 ]; then
  echo "[FATAL] Failed to build kernel. Return with code $result"
  exit 2
fi

# Make os kernel image
echo "[INFO] Making kernel image"
rust-objcopy --strip-all target/riscv64gc-unknown-none-elf/release/os -O binary target/riscv64gc-unknown-none-elf/release/os.bin

result=$?
if [ $result != 0 ]; then
  echo "[FATAL] Failed to make kernel image. Return with code $result"
  exit 2
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
