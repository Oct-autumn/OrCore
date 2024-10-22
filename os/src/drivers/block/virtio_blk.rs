use fs::block_device::BlockDevice;

pub struct VirtIOBlock {

}

impl BlockDevice for VirtIOBlock {
    fn read_block(&self, block_num: usize, buf: &mut [u8]) {
        todo!()
    }

    fn write_block(&self, block_num: usize, buf: &[u8]) {
        todo!()
    }

    fn num_blocks(&self) -> u64 {
        0
    }
}