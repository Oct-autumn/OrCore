//! os/src/main.rs
//! The main source code
#![feature(panic_info_message)] //Enable feature `message()` in panic_info
#![no_std]  //Delete std-lib, use rust-core-lib
#![no_main] //Remove main() func

use core::arch::global_asm;

use crate::rust_sbi::shutdown;

mod lang_items;
mod rust_sbi;
mod console;
mod kernel_log;

global_asm!(include_str!("entry.asm"));

#[no_mangle]
pub fn rust_main() -> ! {
    init_bss();
    println!("Hello");
    info!("Hello");
    debug!("Hello");
    error!("Hello");
    warn!("Hello");
    trace!("Hello");
    panic!("Panic");
    shutdown();
    //loop {}
}

fn init_bss() {
    // init the .bss section
    // use the agreement in C lang to find the section address
    extern "C" {
        fn sbss();
        fn ebss();
    }
    // iterator to init the section
    (sbss as usize..ebss as usize).for_each(|a| {
        unsafe { (a as *mut u8).write_volatile(0) }
    });
}
