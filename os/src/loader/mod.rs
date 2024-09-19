//！ os/src/loader/mod.rs
//！ 本模块用于将app加载进入内存（所有App同时全部加载）并负责内存栈的初始化构建
pub mod stack;

use crate::config::{APP_BASE_ADDRESS, APP_SIZE_LIMIT};
use core::arch::asm;

/// 获取App数量
pub fn get_num_app() -> usize {
    extern "C" {
        fn _num_app();
    }
    unsafe { (_num_app as usize as *const usize).read_volatile() }
}

/// 获取App的基地址
pub fn get_app_base(app_id: usize) -> usize {
    APP_BASE_ADDRESS + APP_SIZE_LIMIT * app_id
}

/// 加载所有App
pub fn load_apps() {
    extern "C" {
        fn _num_app();
    }
    let num_app_ptr = _num_app as usize as *const usize;
    let num_app = get_num_app();

    // 获取各App代码的起始位置
    // 将记录了各App代码起始位置的数组作为数据，直接读取（见link_app.S）
    let app_start = unsafe { core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1) };

    unsafe {
        asm!("fence.i"); // 内存屏障，保证数据一致性
    }

    // 加载各App
    for i in 0..num_app {
        let base_addr = get_app_base(i);
        // 清理目标内存区域（指定位置、长度，进行填零处理）
        unsafe {
            core::slice::from_raw_parts_mut(base_addr as *mut u8, APP_SIZE_LIMIT).fill(0);
        }
        // 从App代码段复制到目标内存区域
        let mem_src = unsafe {
            core::slice::from_raw_parts(app_start[i] as *const u8, app_start[i + 1] - app_start[i])
        };
        let mem_dst =
            unsafe { core::slice::from_raw_parts_mut(base_addr as *mut u8, mem_src.len()) };
        mem_dst.copy_from_slice(mem_src);
    }
}
