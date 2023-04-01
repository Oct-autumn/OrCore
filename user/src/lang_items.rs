//! user/src/lang_items.rs <br>
//! impl of lang items.

/* panic()  Func   handle the KernelPanic
 */

use core::panic::PanicInfo;

use crate::println;
use crate::sys_call::sys_exit;

/// KernelPanic func
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        println!(
            "\x1b[91mPanic!\t{}:{} {}\x1b[0m",
            location.file(),
            location.line(),
            info.message().unwrap()
        );
    } else {
        println!("\x1b[91Panic!\t{}\x1b[0m", info.message().unwrap());
    }
    loop {}
}
