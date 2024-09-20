# OrCore

这是一个基于Rust语言编写的适用于RISC-V硬件平台的OS Kernel.

## 运行环境

要编译运行测试该项目，你需要以下组件：

- QEMU(Support RISC-V 64) / k210开发板
- Rust for RISC-V
- GDB

更多环境配置相关信息请参阅: http://rcore-os.cn/rCore-Tutorial-Book-v3/chapter0/5setup-devel-env.html

## 运行

os文件夹中的Makefile已经包含了编译user文件夹中的用户程序的命令，因此只需要在os文件夹中运行make命令即可编译整个项目。

~~~shell
    cd os
    # 编译并在QEMU模拟器上运行
    make run
    # 编译并在k210开发板上运行
    make run BOARD=k210
~~~

* 你还可以添加编译参数，如`make run LOG="INFO"`可以设置内核的日志输出级别为INFO。

## 注意事项

1. user部分的应用程序在半系统模式下试运行时，需要按照以下代码块中的指引注释掉两行代码，否则将导致程序在半系统中无法正常运行。
    ```toml
    # user/.cargo/config
    [target.riscv64gc-unknown-none-elf]
    rustflags = [
        "-Clink-arg=-Tsrc/linker.ld",     # 当使用半系统模拟时，注释掉
        "-Cforce-frame-pointers=yes"
    ]
    ```
    ```rust
    // user/src/lib.rs
    //...
    pub extern "C" fn _start() -> ! {
        //...
        clear_bss();  //当使用半系统模拟时，注释掉
        //...
    }
    //...
    ```
   
