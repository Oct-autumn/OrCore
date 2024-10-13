mod virtio_blk;
mod sdcard;

use alloc::sync::Arc;
use lazy_static::lazy_static;
use fs::BlockDevice;
use crate::{print, println};

#[cfg(all(feature = "board_qemu", not(feature = "board_k210")))]
type BlockDeviceImpl = virtio_blk::VirtIOBlock;

#[cfg(feature = "board_k210")]
type BlockDeviceImpl = sdcard::SDCardWrapper;

lazy_static! {
    pub static ref BLOCK_DEVICE: Arc<dyn BlockDevice> = Arc::new(BlockDeviceImpl::new());
}