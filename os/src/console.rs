//! os/src/console.rs <br>
//! declare of console output

/* print()     Func    print sth <br>
 * print!      Macro   print <br>
 * println!    Macro   `println!` <br>
 */

use core::fmt::{self, Write};

use crate::rust_sbi::console_putchar;

struct Stdout;  // Unit-like structs

impl Write for Stdout {
    // impl of Write::write_str for Stdout
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            console_putchar(c as usize);
        }
        Ok(())
    }
}

pub fn print(args: fmt::Arguments) {
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
    }
}