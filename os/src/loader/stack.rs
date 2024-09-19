use crate::{
    config::{KERNEL_STACK_SIZE, MAX_APP_NUM, USER_STACK_SIZE},
    trap::TrapContext,
};

use super::get_app_base;

// 定义并初始化内核栈和用户栈
#[repr(align(4096))]
#[derive(Clone, Copy)]
struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE],
}

impl KernelStack {
    // 获取内核栈的栈顶指针
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + KERNEL_STACK_SIZE
    }

    /// 将TrapContext压入内核栈
    /// 返回TrapContext的指针
    pub fn push_context(&self, trap_cx: TrapContext) -> usize {
        let trap_cx_ptr = (self.get_sp() - core::mem::size_of::<TrapContext>()) as *mut TrapContext;
        unsafe {
            *trap_cx_ptr = trap_cx;
        }
        trap_cx_ptr as usize
    }

    // 对应的出栈操作见os/src/trap/trap.S的__restore函数
}

#[repr(align(4096))]
#[derive(Clone, Copy)]
struct UserStack {
    data: [u8; USER_STACK_SIZE],
}

impl UserStack {
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + USER_STACK_SIZE
    }
}

// 定义并初始化多个内核栈和用户栈
#[link_section = ".bss.kernel_stack"] // 将内核栈放在.bss.kernel_stack段
static KERNEL_STACK: [KernelStack; MAX_APP_NUM] = [KernelStack {
    data: [0; KERNEL_STACK_SIZE],
}; MAX_APP_NUM];

#[link_section = ".bss.user_stack"] // 将用户栈放在.bss.user_stack段
static USER_STACK: [UserStack; MAX_APP_NUM] = [UserStack {
    data: [0; USER_STACK_SIZE],
}; MAX_APP_NUM];

/// 构造用于第一次进入App的Trap上下文
/// 返回TrapContext的指针
pub fn init_app_cx(app_id: usize) -> usize {
    KERNEL_STACK[app_id].push_context(TrapContext::app_init_context(
        get_app_base(app_id),
        USER_STACK[app_id].get_sp(),
    ))
}
