//! os/src/kernel_log.rs <br>
//! declare of kernel_log output

/* info!    Marco   print kernel log as INFO level
 * error!   Marco   print kernel log as ERROR level
 * warn!    Marco   print kernel log as WARNING level
 * debug!   Marco   print kernel log as DEBUG level
 * trace!   Marco   print kernel log as TRACE level
 */

/// print something on the console with log level INFO
#[macro_export]
macro_rules! info {
    ($fmt:literal $(, $($arg:tt)+)?) => {
        $crate::console::print(format_args!(concat!("\x1b[34m[Kernel | INFO] \t", $fmt, "\x1b[0m\n") $(, $($arg)+)?));
    }
}

/// print something on the console with log level ERROR
#[macro_export]
macro_rules! error {
    ($fmt:literal $(, $($arg:tt)+)?) => {
        $crate::console::print(format_args!(concat!("\x1b[31m[Kernel | ERROR]\t", $fmt, "\x1b[0m\n") $(, $($arg)+)?));
    }
}

/// print something on the console with log level WARN
#[macro_export]
macro_rules! warn {
    ($fmt:literal $(, $($arg:tt)+)?) => {
        $crate::console::print(format_args!(concat!("\x1b[93m[Kernel | WARN] \t", $fmt, "\x1b[0m\n") $(, $($arg)+)?));
    }
}

/// print something on the console with log level DEBUG
#[macro_export]
macro_rules! debug {
    ($fmt:literal $(, $($arg:tt)+)?) => {
        $crate::console::print(format_args!(concat!("\x1b[32m[Kernel | DEBUG]\t", $fmt, "\x1b[0m\n") $(, $($arg)+)?));
    }
}

/// print something on the console with log level TRACE
#[macro_export]
macro_rules! trace {
    ($fmt:literal $(, $($arg:tt)+)?) => {
        $crate::console::print(format_args!(concat!("\x1b[90m[Kernel | TRACE]\t", $fmt, "\x1b[0m\n") $(, $($arg)+)?));
    }
}