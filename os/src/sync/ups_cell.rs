//! os/src/sync/ups_cell.rs
//! 用于单核环境下的安全引用单元
//!
//! TODO: 改进实现多核支持

use core::cell::{RefCell, RefMut};

pub struct UPSafeCell<T> {
    inner: RefCell<T>,
}

unsafe impl<T> Sync for UPSafeCell<T> {}

impl<T> UPSafeCell<T> {
    /// 使用者应保证该结构体仅在单处理器下使用，否则将触发内存异常
    pub unsafe fn new(value: T) -> Self {
        Self {
            inner: RefCell::new(value),
        }
    }

    /// 当结构体已被借用时再次借用将触发Panic
    pub fn exclusive_access(&self) -> RefMut<'_, T> {
        self.inner.borrow_mut()
    }
}
