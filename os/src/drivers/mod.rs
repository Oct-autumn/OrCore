mod block;

pub use block::BLOCK_DEVICE;
use crate::{print, println};

pub fn test_block_device() {
    let block_device = BLOCK_DEVICE.clone();
    let mut buf = [0u8; 512];
    block_device.read_block(0, &mut buf);
    println!("Read block 0:");
    for i in 0..512 {
        print!("{:02x} ", buf[i]);
        if i % 32 == 31 {
            println!();
        }
    }
}