use crate::{mem::page_table, task};

pub fn sys_get_time(ts_va: usize, _tz: usize) -> isize {
    let ts: &mut TimeVal = page_table::translate_into(task::current_app_token(), ts_va);

    let time = crate::util::time::get_time_usec();

    *ts = TimeVal {
        sec: time / 1_000_000,
        usec: time % 1_000_000,
    };

    0
}

#[repr(C)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}
