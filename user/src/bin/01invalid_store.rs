//! user/src/bin/01invalid_store.rs
//! 实验：越权的非法内存访问

#![no_std]  //Delete std-lib, use rust-core-lib
#![no_main] //Remove main() func

#[macro_use]
extern crate user_lib;

#[no_mangle]
fn main() -> i32 {
    println!("Into Test store_fault, we will insert an invalid store operation...");
    println!("Kernel should kill this application!");
    unsafe {
        core::ptr::null_mut::<u8>().write_volatile(0);
    }
    0
}