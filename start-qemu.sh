#!/bin/bash

# Check running param
gdb_flag=false

KERNEL_SOURCE_PATH="./os"
USER_LIB_SOURCE_PATH="./user"

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

# Start QEMU and load kernel image
echo "[INFO] Starting QEMU"
if [ $gdb_flag == true ]; then
  echo "[WARN] Start at GDB mode"
  qemu-system-riscv64 \
    -machine virt \
    -nographic \
    -bios ./bootloader/rustsbi-qemu.bin \
    -device loader,file='os/target/riscv64gc-unknown-none-elf/release/os.bin',addr=0x80200000 \
    -s -S
elif [ $gdb_flag == false ]; then
  qemu-system-riscv64 \
    -machine virt \
    -nographic \
    -bios ./bootloader/rustsbi-qemu.bin \
    -device loader,file='os/target/riscv64gc-unknown-none-elf/release/os.bin',addr=0x80200000
fi