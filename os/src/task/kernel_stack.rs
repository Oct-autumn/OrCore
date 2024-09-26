use crate::{
    config,
    error::Result,
    mem::{memory_set::SegPermission, KERNEL_SPACE},
};

use super::pid::PidHandle;

/// **功能：** 获取应用内核栈的位置 <br>
/// **参数：** <br>
///         - `pid` 进程ID <br>
/// **返回：** <br>
///         - `(usize, usize)` 应用内核栈的底部和顶部虚拟地址 <br>
pub fn kernel_stack_position(app_id: usize) -> (usize, usize) {
    let top = config::TRAMPOLINE - app_id * (config::KERNEL_STACK_SIZE + config::PAGE_SIZE);
    let bottom = top - config::KERNEL_STACK_SIZE;
    (bottom, top)
}

pub struct KernelStack {
    pid: usize,
}

impl Drop for KernelStack {
    fn drop(&mut self) {
        let (bottom, _) = kernel_stack_position(self.pid);
        KERNEL_SPACE
            .lock()
            .remove_area(bottom.into())
            .map_err(|e| {
                // 内核栈释放失败，直接panic
                panic!("Error when drop KernelStack: {:?}", e);
            })
            .unwrap();
    }
}

impl KernelStack {
    /// 创建一个新的内核栈
    pub fn new(pid_handle: &PidHandle) -> Result<Self> {
        let pid = pid_handle.0;
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(pid);
        // 为内核栈分配空间并映射进内核页表
        KERNEL_SPACE.lock().insert_framed_area(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            SegPermission::R | SegPermission::W,
        )?;
        Ok(Self { pid })
    }

    /// 获取栈顶地址
    pub fn get_top(&self) -> usize {
        let (_, top) = kernel_stack_position(self.pid);
        top
    }

    /// 将一个值压入栈顶
    #[allow(unused)]
    pub fn push_on_top<T>(&self, value: T) -> *mut T
    where
        T: Sized,
    {
        let kernel_stack_top = self.get_top();
        let ptr_mut = (kernel_stack_top - core::mem::size_of::<T>()) as *mut T;
        unsafe {
            *ptr_mut = value;
        }
        ptr_mut
    }
}
