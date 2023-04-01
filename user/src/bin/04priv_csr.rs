//! user/src/bin/02power.rs
//! 实验：系统运算测试

#![no_std]  //Delete std-lib, use rust-core-lib
#![no_main] //Remove main() func

#[macro_use]
extern crate user_lib;

use riscv::register::sstatus::{self, SPP};

#[no_mangle]
fn main() -> i32 {
    println!("Try to access privileged CSR in U Mode");
    println!("Kernel should kill this application!");
    unsafe {
        sstatus::set_spp(SPP::User);    // 试图在U模式下越权操作，改写模式寄存器（CSR）
    }
    0
}