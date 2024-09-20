pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    let time = crate::util::time::get_time_usec();
    unsafe {
        *ts = TimeVal {
            sec: time / 1_000_000,
            usec: time % 1_000_000,
        };
    }
    0
}

#[repr(C)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}
