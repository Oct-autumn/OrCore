# OrCore

This is an operating system kernel for RISC-V hardware written based on Rust.

## Environment

To run and test, you need those components:

- QEMU(Support RISC-V 64)
- Rust for RISC-V
- GDB

For more information, visit site: https://rcore-os.cn/rCore-Tutorial-Book-v3

## Run

All the scripts should run in the path `OrCore/os`.

To build kernel and start the QEMU:

~~~shell
bash ../qemu-start.sh
~~~

To build kernel and start the QEMU with GDB:

~~~shell
bash ../qemu-start.sh --wait-gdb
~~~

~~~shell
sh ../qemu-gdb-start.sh
~~~