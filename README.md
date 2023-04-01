# OrCore

这是一个基于Rust语言编写的适用于RISC-V硬件平台的OS Kernel.

## 运行环境

要编译运行测试该项目，你需要以下组件：

- QEMU(Support RISC-V 64)
- Rust for RISC-V
- GDB

更多环境配置相关信息请参阅: http://rcore-os.cn/rCore-Tutorial-Book-v3/chapter0/5setup-devel-env.html

## 运行

**注意：** 所有脚本都应在`/OrCore`目录下运行.

要编译内核与ULP，请运行以下指令：

~~~shell
./build_rs.sh
~~~

也可使用`--build-os-only` `--build-user-only`等选项指定编译目标和功能

要启动QEMU模拟器并加载对应的OS Kernel镜像，请运行以下指令：

~~~shell
./start-qemu.sh
~~~

如果要在启动模拟器的同时使用GDB进行调试，请运行以下指令：

~~~shell
./start-qemu.sh --wait-gdb
# 新建命令行窗口，运行以下指令
./start-gdb.sh
~~~

## 注意事项

1. user部分的应用程序在半系统模式下试运行时，需要按照以下代码块中的指引注释掉两行代码，否则将导致程序在半系统中无法正常运行。
    ```config
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
   
