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

也可使用`--build-os-only` `--build-user-only`等选项指定编译目标

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