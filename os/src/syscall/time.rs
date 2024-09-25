use crate::{mem::page_table, task};

#[repr(C)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// **功能：** 获取当前时间。 <br>
/// **参数：**  <br>
///         - `time` 表示存放时间的结构体指针；<br>
///         - `tz` 表示时区。（在本OS中不会使用）<br>
/// **返回值：**<br>
///         - 成功：0<br>
///         - 失败：-1（结构体指针异常）<br>
/// **syscall ID：** 169
pub fn sys_get_time(ts_va: usize, _tz: usize) -> isize {
    let ts_res = page_table::translate_into(task::current_process_token(), ts_va);

    if ts_res.is_err() {
        -1
    } else {
        let ts = ts_res.unwrap();
        let time = crate::util::time::get_time_usec();

        *ts = TimeVal {
            sec: time / 1_000_000,
            usec: time % 1_000_000,
        };

        0
    }
}
