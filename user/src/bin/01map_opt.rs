//! user/src/bin/01map_opt.rs
//! 实验：分配与释放内存

#![no_std] //Delete std-lib, use rust-core-lib
#![no_main] //Remove main() func

#[macro_use]
extern crate user_lib;

#[no_mangle]
fn main() -> i32 {
    let start: usize = 0x10000000;
    let len: usize = 0x1000;
    let prot: usize = 0b011;
    assert_eq!(0, user_lib::mmap(start, len, prot));
    for i in start..(start + len) {
        let addr: *mut u8 = i as *mut u8;
        unsafe {
            *addr = i as u8;
        }
    }
    for i in start..(start + len) {
        let addr: *mut u8 = i as *mut u8;
        unsafe {
            assert_eq!(*addr, i as u8);
        }
    }
    println!("mmap test passed!");

    assert_eq!(0, user_lib::munmap(start, len));
    let addr: *mut u8 = start as *mut u8;
    println!("munmap test passed if kernel killed the process!");
    unsafe {
        assert_eq!(*addr, start as u8);
    }
    0
}
