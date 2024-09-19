# os/src/trap/trap.S
# 用于保存和恢复寄存器
# 进入与退出系统调用时，需要对寄存器组进行保存和恢复

.altmacro   # 启用备用宏模式，详见https://www.acrc.bris.ac.uk/acrc/RedHat/rhel-as-en-4/altmacro.html
.macro SAVE_GP n    # 定义宏SAVE_GP，用于保存通用寄存器
    sd x\n, \n*8(sp)
.endm
.macro LOAD_GP n    # 定义宏LOAD_GP，用于恢复通用寄存器
    ld x\n, \n*8(sp)
.endm

    .section .text
    .globl __alltraps
    .globl __restore
    .align 2
# 保存通用寄存器
__alltraps:
    csrrw sp, sscratch, sp                          # 交换sp与sscratch，使得sscratch指向用户栈，sp指向内核栈
    # 划取内核栈空间，用于保存TrapContext
    addi sp, sp, -34*8
    # 下面开始保存通用寄存器
    sd x1, 1*8(sp)                                  # 保存x1
    # 跳过rb寄存器，我们后面再保存它
    sd x3, 3*8(sp)                                  # 保存x3
    # 跳过tp(x4)寄存器，应用程序不使用它
    # 保存x5-x31
    .set n, 5
    .rept 27
        SAVE_GP %n
        .set n, n+1
    .endr

    # 保存sstatus/sepc/sscratch
    csrr t0, sstatus
    csrr t1, sepc
    sd t0, 32*8(sp)
    sd t1, 33*8(sp)

    # 从sscratch读取用户栈地址并保存到内核栈
    csrr t2, sscratch
    sd t2, 2*8(sp)

    # 设置trap_handler的输入参数（cx: &mut TrapContext）
    mv a0, sp
    call trap_handler

# 恢复通用寄存器
# 该func调用的两种情形：
# 第一种：从trap_handler返回至U Mode
# 第二种：通过该func开始运行App
__restore:
    mv sp, a0
    # 现在sp->内核栈，sscratch->用户栈
    # 从内核栈恢复sstatus/sepc/sscratch
    ld t0, 32*8(sp)
    ld t1, 33*8(sp)
    ld t2, 2*8(sp)
    csrw sstatus, t0
    csrw sepc, t1
    csrw sscratch, t2
    # 恢复通用寄存器（跳过x0，sp(x2)，tp(x4)）
    ld x1, 1*8(sp)
    ld x3, 3*8(sp)
    .set n, 5
    .rept 27
        LOAD_GP %n
        .set n, n+1
    .endr
    # 释放内核栈空间
    addi sp, sp, 34*8
    # 交换sp与sscratch，使得sp指向用户栈，sscratch指向内核栈
    csrrw sp, sscratch, sp
    sret