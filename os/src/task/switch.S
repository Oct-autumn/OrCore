# os/src/task/switch.S

.altmacro
.macro SAVE_SN n
    sd s\n, (\n+2)*8(a0)
.endm
.macro LOAD_SN n
    ld s\n, (\n+2)*8(a1)
.endm
    .section .text
    .globl __switch
__switch:
    # 阶段 [1] ：由汇编调用者（编译器）负责将参数传递到正确的寄存器中
    # __switch(
    #     current_task_cx_ptr: *mut TaskContext,    // 保存当前任务的上下文指针，位于 a0 寄存器内
    #     next_task_cx_ptr: *const TaskContext      // 保存下一个任务的上下文指针，位于 a1 寄存器内
    # )
    #
    # TaskContext 结构体定义如下：
    # pub struct TaskContext
    #     ra: usize,
    #     sp: usize,
    #     s: [usize; 12],
    
    # 阶段 [2] ：保存当前任务的上下文
    # 保存当前任务的内核栈指针
    sd sp, 8(a0)
    # 保存当前任务的CPU寄存器：ra、s0~s11
    sd ra, 0(a0)
    .set n, 0
    .rept 12
        SAVE_SN %n
        .set n, n + 1
    .endr
    # 阶段 [3] ：恢复下一个任务的上下文
    # 恢复下一个任务的CPU寄存器：ra、s0~s11
    ld ra, 0(a1)
    .set n, 0
    .rept 12
        LOAD_SN %n
        .set n, n + 1
    .endr
    # 恢复下一个任务的内核栈指针
    ld sp, 8(a1)
    # 阶段 [4] ：返回，执行流进入下一个任务
    ret