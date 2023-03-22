#!/bin/bash

KERNEL_SOURCE_PATH="./os"
USER_LIB_SOURCE_PATH="./user"

# Check running param
os_build_flag=true
user_build_flag=true

for i in "$@"; do
  case $i in
  "--build-os-only")
    user_build_flag=false
    ;;
  "--build-user-only")
    os_build_flag=false
    ;;
  *)
    echo "Build the rust code."
    echo ""
    echo "Usage: build-rs.sh [OPTIONS]"
    echo ""
    echo "Options:"
    echo "      --build-os-only    Build the os kernel only."
    echo "      --build-user-only  Build the user lib and programs only."
    exit 1
    ;;
  esac
done

# Check build target
if [ $os_build_flag == false ] && [ $user_build_flag == false ]; then
  echo "Build the rust code ( os and user )."
  echo ""
  echo "Usage: build-rs.sh [OPTIONS]"
  echo ""
  echo "Options:"
  echo "      --build-os-only    Build the os kernel only."
  echo "      --build-user-only  Build the user lib and programs only."
  exit 1
fi

# Check running path
if [ ! -e "./scripts" ]; then
  echo "[ERROR] Please run under the root dir."
  return 1
fi

# Check to
if [ $os_build_flag == true ]; then
  cd $KERNEL_SOURCE_PATH || exit 100

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
  rust-objcopy --strip-all ./target/riscv64gc-unknown-none-elf/release/os -O binary ./target/riscv64gc-unknown-none-elf/release/os.bin

  result=$?
  if [ $result != 0 ]; then
    echo "[FATAL] Failed to make kernel image. Return with code $result"
    exit 2
  fi

  cd ../ || exit 100
fi

if [ $user_build_flag == true ]; then
  cd $USER_LIB_SOURCE_PATH || exit 100

  # Build ULP
  echo "[INFO] Building and making image for ULP"
  make build

  result=$?
  if [ $result != 0 ]; then
    echo "[FATAL] Failed to build ULP. Return with code $result"
    exit 3
  fi

  cd ../ || exit 100
fi
