# os/src/trap/trap.S
# 用于保存和恢复寄存器的汇编方法__alltraps和__restore
# 进入与退出系统调用时，需要对寄存器组进行保存和恢复

.altmacro   # 启用备用宏模式，详见https://www.acrc.bris.ac.uk/acrc/RedHat/rhel-as-en-4/altmacro.html
.macro SAVE_GP n    # 定义宏SAVE_GP，用于保存通用寄存器
    sd x\n, \n*8(sp)
.endm
.macro LOAD_GP n    # 定义宏LOAD_GP，用于恢复通用寄存器
    ld x\n, \n*8(sp)
.endm

    .section .text.trampoline
    .globl __alltraps
    .globl __restore
    .align 2
# 保存通用寄存器
__alltraps:
    # 交换sp与sscratch，使得sscratch指向用户栈，sp指向内核栈
    csrrw sp, sscratch, sp

    ## 下面开始保存通用寄存器
    # 保存x1
    sd x1, 1*8(sp)                                  
    # 跳过rb寄存器，我们后面再保存它
    # 保存x3
    sd x3, 3*8(sp)
    # 跳过tp(x4)寄存器，应用程序不使用它
    # 保存x5-x31
    .set n, 5
    .rept 27
        SAVE_GP %n
        .set n, n+1
    .endr

    # 保存sstatus/sepc
    csrr t0, sstatus
    csrr t1, sepc
    sd t0, 32*8(sp)
    sd t1, 33*8(sp)

    # 从sscratch读取用户栈地址并保存到内核栈
    csrr t2, sscratch
    sd t2, 2*8(sp)
    
    # 从TrapContext中读取kernel_satp
    ld t0, 34*8(sp)
    # 从TrapContext中读取TrapContext::trap_handler指针
    ld t1, 36*8(sp)
    # 切换到内核栈
    ld sp, 35*8(sp)

    # 切换到内核页表
    csrw satp, t0
    # 刷新TLB
    sfence.vma

    # 跳转到trap_handler
    jr t1

# 恢复通用寄存器
# 该func调用的两种情形：
# 第一种：从trap_handler返回至U Mode
# 第二种：通过该func开始运行App
__restore:
    # a0寄存器中保存了TrapContext的指针
    # a1寄存器中保存了应用程序内存空间的页表的token
    
    # 切换到用户页表
    csrw satp, a1
    # 刷新TLB
    sfence.vma

    csrw sscratch, a0

    mv sp, a0
    
    ld t0, 32*8(sp)
    ld t1, 33*8(sp)
    csrw sstatus, t0
    csrw sepc, t1
    ld x1, 1*8(sp)
    ld x3, 3*8(sp)
    # 恢复通用寄存器（跳过x0，sp(x2)，tp(x4)）
    .set n, 5
    .rept 27
        LOAD_GP %n
        .set n, n+1
    .endr
    # 恢复用户栈地址
    ld sp, 2*8(sp)
    sret