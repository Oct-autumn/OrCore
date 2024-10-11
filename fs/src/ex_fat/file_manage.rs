use crate::ex_fat::index_entry_manage::FileMetaData;
use crate::ex_fat::persistent_layer::model::cluster_id::ClusterId;
use crate::ex_fat::persistent_layer::ClusterManager;
use crate::{config, BlockDevice};
use alloc::sync::Arc;
use core::cmp::{max, min};
use spin::RwLock;

pub struct FileManager {
    /// 簇管理器
    cluster_manager: Arc<RwLock<ClusterManager>>,
    /// 存储设备接口指针
    device: Arc<dyn BlockDevice>,
}

impl FileManager {
    pub fn new(cluster_manager: Arc<RwLock<ClusterManager>>, device: Arc<dyn BlockDevice>) -> Self {
        Self {
            cluster_manager,
            device,
        }
    }

    /// 从文件指定位置读取数据到缓冲区
    pub fn read_at(&self, file_meta_data: &FileMetaData, offset: usize, buf: &mut [u8]) -> usize {
        // 流程：
        // 1. 读取文件的簇链表头
        // 2. 计算起始偏移量对应的簇号、簇内扇区偏移量、扇区内偏移量
        // 3. 读取数据，直到读取完毕或者缓冲区满

        if offset >= file_meta_data.file_size as usize {
            // 偏移量超出文件大小，无法读取
            return 0;
        }

        let cluster_manager = self.cluster_manager.read();
        let buf_len = buf.len();

        if let Some(start_pos) = self.translate_offset(file_meta_data, offset) {
            let mut read_bytes = 0;
            let mut current_pos = start_pos;
            let mut current_offset = offset;

            // 逐扇区从偏移量处开始读取数据，直到读取完毕或者缓冲区满
            loop {
                let sector = cluster_manager.get_cluster_sector(&current_pos.0, current_pos.1 as u32).unwrap();

                sector.read().read(0, |data: &[u8; config::SECTOR_BYTES]| {
                    // 计算当前扇区剩余可读取的字节数：buf剩余空间、当前扇区剩余空间、文件剩余空间 三者的最小值
                    let read_len = min(min(buf_len - read_bytes, config::SECTOR_BYTES - current_pos.2), file_meta_data.file_size as usize - current_offset);
                    // 读取数据
                    buf[read_bytes..read_bytes + read_len].copy_from_slice(&data[current_pos.2..current_pos.2 + read_len]);

                    read_bytes += read_len;
                    current_offset += read_len;

                    // 退出条件：读取完毕或者缓冲区满
                    if read_bytes >= buf_len || current_offset >= file_meta_data.file_size as usize {
                        return;
                    }

                    // 更新当前位置
                    current_pos.2 = 0;
                    current_pos.1 += 1;
                    if current_pos.1 >= cluster_manager.sectors_per_cluster as usize {
                        current_pos.1 = 0;
                        current_pos.0 = cluster_manager.get_next_cluster(&current_pos.0).unwrap();
                    }
                });

                if read_bytes >= buf_len || current_offset >= file_meta_data.file_size as usize {
                    break;
                }
            }

            read_bytes
        } else {
            0
        }
    }

    /// 将缓冲区数据写入文件指定位置
    ///
    /// 注意：本方法会修改文件元数据，包括文件大小、簇链表等，调用后需要更新文件元数据
    pub fn write_at(&self, file_meta_data: &mut FileMetaData, offset: usize, buf: &[u8]) -> usize {
        // 流程：
        // 1. 读取文件的簇链表头
        // 2. 计算起始偏移量对应的簇号、簇内扇区偏移量、扇区内偏移量
        // 3. 写入数据，直到写入完毕（期间可能需要分配新簇）

        if offset == 0 && file_meta_data.first_cluster.is_none() {
            // 文件为空，需要分配第一个簇
            let mut cluster_manager = self.cluster_manager.write();
            if let Some(new_cluster) = cluster_manager.alloc_new_cluster() {
                // 分配成功，更新文件元数据
                file_meta_data.first_cluster = Some(new_cluster);
            } else {
                // 分配失败，写入终止
                panic!("Failed to allocate new cluster");
            }
        } else if offset >= file_meta_data.file_size as usize {
            // 偏移量超出文件大小，无法写入
            return 0;
        }

        let buf_len = buf.len();

        if let Some(start_pos) = self.translate_offset(file_meta_data, offset) {
            let mut cluster_manager = self.cluster_manager.write();
            let mut write_bytes = 0;
            let mut current_pos = start_pos;

            // 逐扇区从偏移量处开始写入数据，直到写入完毕
            loop {
                let sector = cluster_manager.get_cluster_sector(&current_pos.0, current_pos.1 as u32).unwrap();

                sector.write().modify(0, |data: &mut [u8; config::SECTOR_BYTES]| {
                    // 计算当前扇区剩余可写入的字节数：buf剩余空间、当前扇区剩余空间 两者的最小值
                    let write_len = min(buf_len - write_bytes, config::SECTOR_BYTES - current_pos.2);
                    // 写入数据
                    data[current_pos.2..current_pos.2 + write_len].copy_from_slice(&buf[write_bytes..write_bytes + write_len]);

                    write_bytes += write_len;

                    // 更新当前位置
                    current_pos.2 = 0;
                    current_pos.1 += 1;
                    if current_pos.1 >= cluster_manager.sectors_per_cluster as usize {
                        current_pos.1 = 0;
                        let next_cluster = cluster_manager.get_next_cluster(&current_pos.0);
                        if next_cluster.is_some() && !next_cluster.unwrap().is_end_of_file() {
                            current_pos.0 = next_cluster.unwrap();
                        } else {
                            // 当前簇已经没有下一个簇，需要分配新簇
                            if let Some(new_cluster) = cluster_manager.alloc_and_append_cluster(&current_pos.0) {
                                // 分配成功，更新当前簇的下一个簇
                                current_pos.0 = new_cluster;
                            } else {
                                // 分配失败，写入终止
                                panic!("Failed to allocate new cluster");
                            }
                        }
                    }
                });

                if write_bytes >= buf_len {
                    break;
                }
            }

            // 更新文件大小（如果写入的数据超出文件大小）
            file_meta_data.file_size = max(file_meta_data.file_size, (offset + write_bytes) as u64);

            write_bytes
        } else {
            0
        }
    }

    /// 清空文件内容
    pub fn clear_file(&self, file_meta_data: &FileMetaData) -> Option<()> {
        let mut cluster_manager = self.cluster_manager.write();

        // 簇链头
        let first_cluster_id = file_meta_data.first_cluster;

        if let Some(cluster_id) = first_cluster_id {
            // 释放簇链
            cluster_manager.free_cluster_chain(cluster_id);
            Some(())
        } else {
            panic!("None first cluster id");
        }
    }

    /// 计算文件内偏移量对应的簇号、簇内扇区偏移量、扇区内偏移量
    ///
    /// 内部函数，可以保证传入的偏移量不会超出文件大小，即文件已分配的簇足够覆盖偏移量
    fn translate_offset(&self, file_meta_data: &FileMetaData, offset: usize) -> Option<(ClusterId, usize, usize)> {
        // 计算起始偏移量对应的簇号、簇内扇区偏移量、扇区内偏移量
        let cluster_size = self.cluster_manager.read().sectors_per_cluster as usize * config::SECTOR_BYTES;
        let cluster_offset = offset / cluster_size;
        let mut target_cluster = file_meta_data.first_cluster.unwrap();

        if file_meta_data.is_fragment {
            // 片段文件，需要通过cluster_manager查询簇号
            for _ in 0..cluster_offset {
                target_cluster = self.cluster_manager.read().get_next_cluster(&target_cluster).unwrap();
            }
        } else {
            // 非片段文件，直接计算
            target_cluster.0 += cluster_offset as u32;
        }

        let sector_offset = (offset % cluster_size) / config::SECTOR_BYTES;
        let byte_offset = offset % config::SECTOR_BYTES;

        Some((target_cluster, sector_offset, byte_offset))
    }
}