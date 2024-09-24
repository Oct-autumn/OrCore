//！ os/src/loader/mod.rs
//！ 本模块用于将app加载进入内存（所有App同时全部加载）并负责内存栈的初始化构建

/// 获取App数量
pub fn get_num_app() -> usize {
    extern "C" {
        fn _num_app();
    }
    unsafe { (_num_app as usize as *const usize).read_volatile() }
}

/// 获取用户App的数据
pub fn get_app_data(app_id: usize) -> &'static [u8] {
    extern "C" {
        fn _num_app();
    }
    let num_app_ptr = _num_app as usize as *const usize;
    let num_app = get_num_app();
    assert!(app_id < num_app, "app_id out of range");

    // 获取App代码的起始位置
    let app_start_data = unsafe { core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1) };
    let app_start = app_start_data[app_id];
    let app_len = app_start_data[app_id + 1] - app_start_data[app_id];

    unsafe { core::slice::from_raw_parts(app_start as *const u8, app_len) }
}
