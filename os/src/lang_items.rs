//! os/src/lang_items.rs <br>
//! impl of lang items.

/* panic()  Func   handle the KernelPanic
 */

use core::panic::PanicInfo;

use alloc::format;
use alloc::string::String;

use crate::println;
use crate::sbi_call::shutdown;
use crate::util::time::get_time_usec;

/// KernelPanic func
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        println!(
            "\x1b[91mPanic!\t{}s\t{}:{} {}\x1b[0m",
            get_time_usec() as f64 / 1_000_000.0,
            location.file(),
            location.line(),
            info.message()
        );
    } else {
        println!(
            "\x1b[91Panic!\t{}\n{}\x1b[0m",
            info.message(),
            external_info()
        );
    }

    shutdown()
}

/// 额外信息
fn external_info() -> String {
    let mut info = String::new();

    info.push_str(format!("< OrCore build: {} >", env!("CARGO_PKG_VERSION")).as_str());

    info
}
