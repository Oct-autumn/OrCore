use super::{syscall, SYSCALL_GET_TIME};

#[repr(C)]
pub struct TimeVal {
    /// 系统启动后经过的秒数
    pub sec: usize,
    /// 系统启动后经过的微秒数
    pub usec: usize,
}

/// **功能：** 获取当前时间。 <br>
/// **参数：**  <br>
///         - `time` 表示存放时间的结构体指针；<br>
///         - `tz` 表示时区。（在本OS中不会使用）<br>
/// **返回值：** 0 <br>
/// **syscall ID：** 169
pub fn sys_get_time(time: *mut TimeVal, tz: usize) -> isize {
    syscall(SYSCALL_GET_TIME, [time as *const _ as usize, tz, 0])
}

/// **功能：** 以毫秒为单位获取当前时间。 <br>
/// **返回值：** 当前时间的毫秒数。
pub fn get_time_msec() -> usize {
    let mut tv = TimeVal { sec: 0, usec: 0 };
    sys_get_time(&mut tv, 0);
    tv.usec / 1_000 + tv.sec * 1_000
}

/// **功能：** 以微秒为单位获取当前时间。 <br>
/// **返回值：** 当前时间的微秒数。
pub fn get_time_usec() -> usize {
    let mut tv = TimeVal { sec: 0, usec: 0 };
    sys_get_time(&mut tv, 0);
    tv.usec + tv.sec * 1_000_000
}
