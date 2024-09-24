//! user/src/bin/01invalid_store.rs
//! 实验：系统调用字符输出

#![no_std] //Delete std-lib, use rust-core-lib
#![no_main] //Remove main() func

#[macro_use]
extern crate user_lib;

#[no_mangle]
fn main() -> i32 {
    println!("\"Hello,world!\" from user program.");
    0
}
