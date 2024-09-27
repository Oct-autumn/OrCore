# os/src/entry.asm

    .section .text.entry
    .globl _start
_start:
    # RustSBI会将处理器ID放入a0寄存器
    # 各内核启动栈栈顶的位置为：boot_stack_lower_bound + 64KiB * (处理器ID + 1)
    add t0, a0, 1                   # t0 = a0 + 1
    slli t0, t0, 16                 # K210 上每个启动栈大小为 64KiB（0x10000），所以这里将处理器ID左移 16 位
    la sp, boot_stack_lower_bound   # sp = boot_stack_lower_bound
    add sp, sp, t0                  # sp = sp + t0

    call rust_main                  # transfer control to kernel Func

    .section .bss.stack
    .globl boot_stack_lower_bound   # mark the stack lower bound
boot_stack_lower_bound:
    .space 4096 * 16 * 2            # set the stack space as 4096*16Byte = 64KB
    .globl boot_stack_top           # mark the top position of the stack when booting
boot_stack_top: