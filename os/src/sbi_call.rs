//! os/src/sbi_call.rs
//! The rust_sbi service impl

/* sbi_call()          Func    call sbi service
 * console_putchar()   Func    put a char into console
 * shutdown()          Func    shutdown the machine gracefully
 */
// Disable unused code warning for this file
#![allow(unused)]

use core::arch::asm;

use log::warn;

const SBI_SET_TIMER: usize = 0;
const SBI_CONSOLE_PUTCHAR: usize = 1;
const SBI_CONSOLE_GETCHAR: usize = 2;
const SBI_CLEAR_IPI: usize = 3;
const SBI_SEND_IPI: usize = 4;
const SBI_REMOTE_FENCE_I: usize = 5;
const SBI_REMOTE_SFENCE_VMA: usize = 6;
const SBI_REMOTE_SFENCE_VMA_ASID: usize = 7;

// system reset extension
const SRST_EXTENSION: usize = 0x53525354;
const SBI_SHUTDOWN: usize = 0;
const SBI_REBOOT: usize = 1;

// call RustSBI service
#[inline(always)]
fn sbi_call(eid: usize, fid: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    let mut ret;
    unsafe {
        asm!(
        "ecall",
        inlateout("x10") arg0 => ret,
        in("x11") arg1,
        in("x12") arg2,
        in("x16") fid,
        in("x17") eid,
        );
    }
    ret
}

/// put a char into console
/// # args
/// * `c` - char
pub fn console_putchar(c: usize) {
    sbi_call(SBI_CONSOLE_PUTCHAR, 0, c, 0, 0);
}

/// shutdown the machine gracefully
pub fn shutdown() -> ! {
    warn!("Shutdown the machine gracefully.");
    sbi_call(SRST_EXTENSION, SBI_SHUTDOWN, 0, 0, 0);
    panic!("It should shutdown!")
}

/// set timer interrupt
/// # args
/// * `time` - time to set
pub fn set_timer(time: usize) {
    sbi_call(SBI_SET_TIMER, 0, time, 0, 0);
}
