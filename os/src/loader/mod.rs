//！ os/src/loader/mod.rs
//！ 本模块用于将app加载进入内存（所有App同时全部加载）并负责内存栈的初始化构建

use core::str;

use alloc::vec::Vec;
use lazy_static::lazy_static;

use crate::{
    error::{process, Error, ErrorKind, MsgType, Result},
    new_error, println,
};

extern "C" {
    fn _num_app();
}

lazy_static! {
    static ref APP_NAMES: Vec<&'static str> = {
        let mut v = Vec::new();
        let num_app = get_num_app();
        extern "C" {
            fn _app_names();
        }
        // 获取App名称的起始位置
        let mut start = _app_names as usize as *const u8;

        unsafe {
            for _ in 0..num_app {
                // 截取名称长度
                let mut end = start;
                while end.read_volatile() != '\0' as u8 {
                    end = end.add(1)
                }
                // 将元数据转换为字符串
                v.push(str::from_utf8(core::slice::from_raw_parts(start, end as usize - start as usize)).unwrap());
                // 更新起始位置
                start = end.add(1);
            }
        }
        v
    };
}

/// 列出链入内核的App
pub fn list_apps() {
    println!("**** App in Kernel List [id. name] ****");
    for (index, name) in APP_NAMES.iter().enumerate() {
        println!("{}. {}", index, name);
    }
    println!("***************************************");
}

/// 获取App数量
pub fn get_num_app() -> usize {
    unsafe { (_num_app as usize as *const usize).read_volatile() }
}

/// 获取用户App的数据
pub fn get_elf_data_by_id(app_id: usize) -> &'static [u8] {
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

/// 通过App名获取其数据
pub fn get_elf_data_by_name(name: &str) -> Result<&'static [u8]> {
    let res = (0..get_num_app())
        .find(|&index| APP_NAMES[index] == name)
        .map(|id| get_elf_data_by_id(id));
    if res.is_none() {
        Err(new_error!(
            ErrorKind::Process(process::ErrorKind::InvalidPath),
            MsgType::StaticStr("Invalid prog path")
        ))
    } else {
        Ok(res.unwrap())
    }
}
