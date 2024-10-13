//! os/src/console.rs <br>
//! declare of console output

/* print()     Func    print sth <br>
 * print!      Macro   print <br>
 * println!    Macro   `println!` <br>
 */

use core::fmt::{self, Write};

use lazy_static::lazy_static;

use crate::{sbi_call::console_putchar, sync::SpinLock};

struct Stdout; // Unit-like structs

lazy_static! {
    static ref STDOUT_LOCK: SpinLock<()> = SpinLock::new(());
}

impl Write for Stdout {
    // impl of Write::write_str for Stdout
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() {
            console_putchar(b as usize);
        }
        Ok(())
    }
}

pub fn print(args: fmt::Arguments) {
    let _guard = STDOUT_LOCK.lock();
    Stdout.write_fmt(args).unwrap();
}

/// print something on the console
#[macro_export]
macro_rules! print {
    ($fmt:literal $(, $($arg:tt)+)?) => {
        $crate::console::print(format_args!($fmt $(, $($arg)+)?));
    }
}

/// print something on the console with a new line (\n)
#[macro_export]
macro_rules! println {
    ($fmt:literal $(, $($arg:tt)+)?) => {
        $crate::console::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
    };
    () => { $crate::console::print(format_args!("\n")); };
}
