Route_To_GDB="$HOME/rv64toolchain/bin/riscv64-unknown-elf-gdb"

# Check running path
if [ ! -e "./bootloader" ]; then
  echo "[ERROR] Please run under the root dir."
  return 1
fi

$Route_To_GDB \
    -ex 'file ./os/target/riscv64gc-unknown-none-elf/release/os' \
    -ex 'set arch riscv:rv64' \
    -ex 'target remote localhost:1234'