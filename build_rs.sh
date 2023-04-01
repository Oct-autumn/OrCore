#!/bin/bash

KERNEL_SOURCE_PATH="./os"
USER_LIB_SOURCE_PATH="./user"

# Check running param
os_build_flag=true
user_build_flag=true
os_log_level=0
ulib_log_level=0

print_help_info() {
  echo "Build the rust code."
  echo ""
  echo "Usage: build-rs.sh [OPTIONS]"
  echo ""
  echo "Options:"
  echo "      --build-os-only                 Build the os kernel only."
  echo "      --build-user-only               Build the user lib and programs only."
  echo "      --max-kernel-log-level-[LEVEL]  Build the os kernel with log level [ERROR | WARN | INFO | DEBUG | TRACE]"
  echo "      --NO-kernel-log                 Build the os kernel with no log"
  echo "      --max-ulib-log-level-[LEVEL]    Build the user lib and programs with log level [ERROR | WARN | INFO | DEBUG | TRACE]"
  echo "      --NO-ulib-log                   Build the user lib and programs with no log"
}

for i in "$@"; do
  case $i in
  "--build-os-only")
    user_build_flag=false
    ;;
  "--build-user-only")
    os_build_flag=false
    ;;
  "--NO-kernel-log")
    os_log_level=0
    ;;
  "--max-kernel-log-level-ERROR")
    os_log_level=1
    ;;
  "--max-kernel-log-level-WARN")
    os_log_level=2
    ;;
  "--max-kernel-log-level-INFO")
    os_log_level=3
    ;;
  "--max-kernel-log-level-DEBUG")
    os_log_level=4
    ;;
  "--max-kernel-log-level-TRACE")
    os_log_level=5
    ;;
  "--NO-ulib-log")
    ulib_log_level=0
    ;;
  "--max-ulib-log-level-ERROR")
    ulib_log_level=1
    ;;
  "--max-ulib-log-level-WARN")
    ulib_log_level=2
    ;;
  "--max-ulib-log-level-INFO")
    ulib_log_level=3
    ;;
  "--max-ulib-log-level-DEBUG")
    ulib_log_level=4
    ;;
  "--max-ulib-log-level-TRACE")
    ulib_log_level=5
    ;;
  *)
    print_help_info
    exit 1
    ;;
  esac
done

# Check build target
if [ $os_build_flag == false ] && [ $user_build_flag == false ]; then
  print_help_info
  exit 1
fi

# Check running path
if [ ! -e "./bootloader" ]; then
  echo "[ERROR] Please run under the root dir."
  exit 1
fi

# Build ULP
if [ $user_build_flag == true ]; then
  cd $USER_LIB_SOURCE_PATH || exit 100

  echo "[INFO] Building and making image for ULP"

  ulib_log_level

  case $os_log_level in
  0)
    unset LOG
    ;;
  1)
    export LOG="ERROR"
    ;;
  2)
    export LOG="WARN"
    ;;
  3)
    export LOG="INFO"
    ;;
  4)
    export LOG="DEBUG"
    ;;
  5)
    export LOG="TRACE"
    ;;
  esac

  make build

  unset LOG

  result=$?
  if [ $result != 0 ]; then
    echo "[FATAL] Failed to build ULP. Return with code $result"
    exit 3
  fi

  cd ../ || exit 100
fi

# Build os kernel
if [ $os_build_flag == true ]; then
  cd $KERNEL_SOURCE_PATH || exit 100

  echo "[INFO] Building or-os kernel"

  case $os_log_level in
  0)
    unset LOG
    ;;
  1)
    export LOG="ERROR"
    ;;
  2)
    export LOG="WARN"
    ;;
  3)
    export LOG="INFO"
    ;;
  4)
    export LOG="DEBUG"
    ;;
  5)
    export LOG="TRACE"
    ;;
  esac

  cargo build --release
  unset LOG

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
