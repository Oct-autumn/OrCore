# Check running path
if [ ! -e "./Cargo.toml" ]; then
  echo "[ERROR] Please run under the Cargo dir."
  return 1
fi

~/rv64toolchain/bin/riscv64-unknown-elf-gdb \
    -ex 'file target/riscv64gc-unknown-none-elf/release/os' \
    -ex 'set arch riscv:rv64' \
    -ex 'target remote localhost:1234'