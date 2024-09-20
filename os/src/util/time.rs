#![allow(unused)]

use riscv::register::time;

use crate::{
    config::{CLOCK_FREQ, TICKS_PER_SEC},
    sbi_call::set_timer,
};

const NANOSECONDS_PER_SECOND: usize = 1_000_000_000;
const MICROSECONDS_PER_SECOND: usize = 1_000_000;
const MILLISECONDS_PER_SECOND: usize = 1_000;

/// 原始时钟周期数据
pub fn get_time_raw() -> usize {
    time::read()
}

/// 以毫秒为单位获取系统运行时间
pub fn get_time_msec() -> usize {
    time::read() / (CLOCK_FREQ / MILLISECONDS_PER_SECOND)
}

/// 以微秒为单位获取系统运行时间
pub fn get_time_usec() -> usize {
    time::read() / (CLOCK_FREQ / MICROSECONDS_PER_SECOND)
}

/// 以纳秒为单位获取系统运行时间（不精准）
///
/// 实验性函数
pub fn get_time_nsec() -> usize {
    (time::read() as f64 / (CLOCK_FREQ as f64 / NANOSECONDS_PER_SECOND as f64)) as usize
}

/// 重设下次时钟中断
///
/// 重设下次时钟中断，使得时钟中断在x周期后触发
/// x = 时钟频率 / 每秒中断次数
pub fn reset_next_timer() {
    set_timer(get_time_raw() + CLOCK_FREQ / TICKS_PER_SEC);
}
