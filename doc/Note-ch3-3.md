本小节我们实现了一个协作式多道程序调度的OS。在ch3-1的基础上，我们实现了用户应用程序间的上下文切换。这允许应用程序在执行中途主动让出CPU，切换到其它任务执行。这是一个简单的多道程序设计，但是在实际应用中，我们需要内核拥有更大的主动权，比如时钟中断、进程调度等。这些功能将在后续章节中逐步实现。

值得注意的是：本小节修改了`trap.S`的汇编代码，删除了`mv sp, a0`这行代码。这原本是用来接受用户程序的栈指针的，但是在多道程序设计中，我们需要内核中的TaskManager来管理用户程序的栈，因此这行代码被删除了。
> TaskManager在执行`__switch`时会将新任务的栈指针通过TaskContext恢复到`sp`寄存器中，不需要额外的操作。