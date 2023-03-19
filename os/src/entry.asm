# os/src/entry.asm

    .section .text.entry
    .globl _start
_start:
    la sp, boot_stack_top           # set StackPointer at the stack_top
    call rust_main                  # transfer control to kernel Func

    .section .bss.stack
    .globl boot_stack_lower_bound   # mark the stack lower bound
boot_stack_lower_bound:
    .space 4096 * 16                # set the stack space as 4096*16Byte = 64KB
    .globl boot_stack_top           # mark the top position of the stack when booting
boot_stack_top: