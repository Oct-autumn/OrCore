//! user/src/bin/03time_opt.rs
//! 实验：时间系统调用

#![no_std] //Delete std-lib, use rust-core-lib
#![no_main] //Remove main() func

#[macro_use]
extern crate user_lib;

use user_lib::{get_time_usec, yield_next};

#[no_mangle]
fn main() -> i32 {
    // TODO: 调用异常！
    println!(
        "Time since boot: {:.6}s",
        get_time_usec() as f64 / 1_000_000.0
    );
    sleep(1_000_000 * 3); // 睡眠三秒钟
    println!(
        "Time since boot: {:.6}s",
        get_time_usec() as f64 / 1_000_000.0
    );
    0
}

/// 睡眠函数
///
/// 使当前任务进入睡眠状态，直到经过指定的时间间隔
///
/// # 参数
/// - `interval`：睡眠时间，单位为微秒
fn sleep(interval: usize) {
    let wakeup_time = get_time_usec() + interval;
    while get_time_usec() < wakeup_time {
        yield_next();
    }
}
