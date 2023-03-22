//! user/src/bin/02power.rs
//! 实验：系统运算测试

#![no_std]  //Delete std-lib, use rust-core-lib
#![no_main] //Remove main() func

#[macro_use]
extern crate user_lib;

use core::arch::asm;

#[no_mangle]
fn main() -> i32 {
    println!("Try to execute privileged instruction in U Mode");
    println!("Kernel should kill this application!");
    unsafe {
        asm!("sret");   // 在S模式下使用sret指令将返回原先指令的下一条指令，此处为U模式下的越权指令
    }
    0
}