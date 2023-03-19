//! os/src/lang_items.rs <br>
//! impl of lang items.

/* panic()  Func   handle the KernelPanic
 */

use core::panic::PanicInfo;

use crate::println;
use crate::rust_sbi::shutdown;

/// KernelPanic func
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        println!(
            "Kernel Panic at {}:{} {}",
            location.file(),
            location.line(),
            info.message().unwrap()
        );
    } else {
        println!("Kernel Panic: {}", info.message().unwrap());
    }
    shutdown()
}