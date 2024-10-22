// TODO: Debug
//#![no_std]
pub mod config;
pub mod ex_fat;
mod io_error;
pub mod block_device;

extern crate alloc;
extern crate core;

pub use block_device::BlockDevice;
pub use ex_fat::ExFAT;