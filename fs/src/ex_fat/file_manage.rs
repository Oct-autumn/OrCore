use crate::block_device::BlockDevice;
use crate::config;
use crate::ex_fat::boot_sector::BootSector;
use crate::ex_fat::cluster_chain::ClusterManager;
use crate::ex_fat::FileDirMetadata;
use crate::ex_fat::model::cluster_id::ClusterId;
use crate::ex_fat::model::index_entry::Attributes;
use alloc::sync::Arc;
use core::cmp::{max, min};
use spin::RwLock;

pub struct FileManager {
    /// 簇管理器
    cluster_manager: Arc<RwLock<ClusterManager>>,

    /// 簇大小，单位：字节
    bytes_per_cluster: usize,
}

impl FileManager {
    pub fn new(boot_sector: &BootSector, cluster_manager: Arc<RwLock<ClusterManager>>) -> Self {
        Self {
            cluster_manager,
            bytes_per_cluster: (1 << boot_sector.sectors_per_cluster_shift) * config::SECTOR_BYTES,
        }
    }

    /// 从文件指定位置读取数据到缓冲区
    pub fn read_at(&self, file_meta_data: &FileDirMetadata, offset: usize, buf: &mut [u8]) -> usize {
        // 流程：
        // 1. 读取文件的簇链表头
        // 2. 计算起始偏移量对应的簇号、簇内扇区偏移量、扇区内偏移量
        // 3. 读取数据，直到读取完毕或者缓冲区满

        assert!(!file_meta_data.attributes.contains(Attributes::Directory), "Cannot read content from Directory");

        if offset >= file_meta_data.size {
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
                let sector = cluster_manager.get_cluster_sector(&current_pos.0, current_pos.1).unwrap();

                sector.read().read(0, |data: &[u8; config::SECTOR_BYTES]| {
                    // 计算当前扇区剩余可读取的字节数：buf剩余空间、当前扇区剩余空间、文件剩余空间 三者的最小值
                    let read_len = min(min(buf_len - read_bytes, config::SECTOR_BYTES - current_pos.2), file_meta_data.size - current_offset);
                    // 读取数据
                    buf[read_bytes..read_bytes + read_len].copy_from_slice(&data[current_pos.2..current_pos.2 + read_len]);

                    read_bytes += read_len;
                    current_offset += read_len;

                    // 退出条件：读取完毕或者缓冲区满
                    if read_bytes >= buf_len || current_offset >= file_meta_data.size {
                        return;
                    }
                });

                if read_bytes >= buf_len || current_offset >= file_meta_data.size {
                    break;
                }

                // 更新当前位置
                current_pos.2 = 0;
                current_pos.1 += 1;
                if current_pos.1 >= cluster_manager.sectors_per_cluster {
                    current_pos.1 = 0;
                    // 前往下一簇
                    if file_meta_data.is_fragment {
                        // 片段文件，需要通过cluster_manager查询簇号
                        current_pos.0 = cluster_manager.get_next_cluster(&current_pos.0).unwrap();
                    } else {
                        // 非片段文件，直接计算
                        current_pos.0.0 += 1;
                    }
                }
            }

            read_bytes
        } else {
            0
        }
    }

    /// 内部函数：为文件分配新簇
    fn expand_file(&self, file_meta_data: &mut FileDirMetadata, expand_cluster_count: usize) -> Option<()> {
        let mut cluster_manager = self.cluster_manager.write();

        // 为文件夹分配新簇
        let Some((new_cluster_id, is_fragment)) = cluster_manager.alloc_new_cluster(
            &(if file_meta_data.first_cluster.is_invalid() {
                // 若文件夹尚未分配簇，则hint为EOF
                ClusterId::eof()
            } else {
                // 文件夹已分配簇，则hint为当前簇链的末尾的下一簇
                ClusterId(file_meta_data.first_cluster.0 + 1)
            }), expand_cluster_count, file_meta_data.is_fragment) else {
            panic!("Failed to allocate new cluster");
        };

        // 原簇链为空，更新父目录的起始簇号
        // 原簇链非空，视情况更新FAT表
        //   - 若原簇链为连续簇链，而新簇为非连续簇链，则在FAT中注册之前的连续簇链，并将新簇链挂至原簇链的末尾
        //   - 若原簇链为非连续簇链，新簇也为非连续簇链，则将新簇链挂至原簇链的末尾

        if file_meta_data.first_cluster.is_invalid() {
            // 更新起始簇号
            file_meta_data.first_cluster = new_cluster_id.clone();
        } else {
            if file_meta_data.is_fragment != is_fragment {
                // 原簇链与新簇链的连续性不同（只可能是连续->非连续）
                // 在FAT表中注册之前的连续簇链
                cluster_manager.set_continued_cluster_chain(&file_meta_data.first_cluster,
                                                            (file_meta_data.size + self.bytes_per_cluster - 1) / self.bytes_per_cluster
                );
            }

            if is_fragment {
                // 新簇链为非连续簇链，挂至原簇链的末尾
                // 获取原簇链的末尾簇号
                let mut now_cluster_id = file_meta_data.first_cluster;
                loop {
                    let next_cluster_id = cluster_manager.get_next_cluster(&now_cluster_id).unwrap();
                        if next_cluster_id.is_eof() {
                            break;
                        } else {
                            now_cluster_id = next_cluster_id;
                            continue;
                        }
                }
                // 更新原簇链的末尾簇的FAT表项
                cluster_manager.set_next_cluster(&now_cluster_id, &new_cluster_id);
            }
        }

        file_meta_data.is_fragment = is_fragment;

        Some(())
    }

    /// 将缓冲区数据写入文件指定位置
    ///
    /// 注意：本方法会修改文件元数据，包括文件大小、簇链表等，调用后需要更新文件元数据
    pub fn write_at(&self, file_meta_data: &mut FileDirMetadata, offset: usize, buf: &[u8]) -> usize {
        // 流程：
        // 1. 计算文件新的大小，决定是否要分配新簇
        // 3. 写入数据，直到写入完毕（期间可能需要分配新簇）

        assert!(!file_meta_data.attributes.contains(Attributes::Directory), "Cannot write content into Directory");
        assert!(!file_meta_data.attributes.contains(Attributes::ReadOnly), "Cannot write content into ReadOnly file");


        let new_size = max(file_meta_data.size, offset + buf.len());
        {
            let original_cluster_count = (file_meta_data.size + self.bytes_per_cluster - 1) / self.bytes_per_cluster;
            let new_cluster_count = (new_size + self.bytes_per_cluster - 1) / self.bytes_per_cluster;

            if new_cluster_count > original_cluster_count && self.expand_file(file_meta_data, new_cluster_count - original_cluster_count).is_none() {
                // 分配失败
                return 0;
            }
        }

        if let Some(start_pos) = self.translate_offset(file_meta_data, offset) {
            let mut cluster_manager = self.cluster_manager.write();
            let mut write_bytes = 0;
            let mut current_pos = start_pos;

            // 逐扇区从偏移量处开始写入数据，直到写入完毕
            loop {                
                let sector = cluster_manager.get_cluster_sector(&current_pos.0, current_pos.1).unwrap();

                sector.write().modify(0, |data: &mut [u8; config::SECTOR_BYTES]| {
                    // 计算当前扇区剩余可写入的字节数：buf剩余空间、当前扇区剩余空间 两者的最小值
                    let write_len = min(buf.len() - write_bytes, config::SECTOR_BYTES - current_pos.2);
                    // 写入数据
                    data[current_pos.2..current_pos.2 + write_len].copy_from_slice(&buf[write_bytes..write_bytes + write_len]);

                    write_bytes += write_len;
                });

                if write_bytes >= buf.len() {
                    break;
                }
                
                // 更新写入指针
                current_pos.2 = 0;
                current_pos.1 += 1;
                if current_pos.1 >= cluster_manager.sectors_per_cluster {
                    current_pos.1 = 0;
                    // 前往下一簇
                    if file_meta_data.is_fragment {
                        // 片段文件，需要通过cluster_manager查询簇号
                        current_pos.0 = cluster_manager.get_next_cluster(&current_pos.0).unwrap();
                    } else {
                        // 非片段文件，直接计算
                        current_pos.0.0 += 1;
                    }
                }
            }

            // 更新文件大小
            file_meta_data.size = new_size;

            write_bytes
        } else {
            0
        }
    }

    /// 清空文件内容
    /// 
    /// 注意：本方法会修改文件元数据，包括文件大小、簇链表等，调用后需要更新文件元数据
    pub fn clear_file(&self, file_metadata: &mut FileDirMetadata) -> Option<()> {
        assert!(!file_metadata.attributes.contains(Attributes::Directory)); // 不能清空目录
        
        // 释放簇链
        self.cluster_manager.write().free_cluster_chain(
            &file_metadata.first_cluster,
            (file_metadata.size + self.bytes_per_cluster - 1) / self.bytes_per_cluster,
            file_metadata.is_fragment,
        )?;

        file_metadata.size = 0;
        file_metadata.first_cluster = ClusterId::eof();
        
        Some(())
    }

    /// 计算文件内偏移量对应的簇号、簇内扇区偏移量、扇区内偏移量
    ///
    /// 内部函数，可以保证传入的偏移量不会超出文件大小，即文件已分配的簇足够覆盖偏移量
    fn translate_offset(&self, file_meta_data: &FileDirMetadata, offset: usize) -> Option<(ClusterId, usize, usize)> {
        // 计算起始偏移量对应的簇号、簇内扇区偏移量、扇区内偏移量
        let cluster_offset = offset / self.bytes_per_cluster;
        let mut target_cluster = file_meta_data.first_cluster;

        if file_meta_data.is_fragment {
            // 片段文件，需要通过cluster_manager查询簇号
            for _ in 0..cluster_offset {
                target_cluster = self.cluster_manager.read().get_next_cluster(&target_cluster).unwrap();
            }
        } else {
            // 非片段文件，直接计算
            target_cluster.0 += cluster_offset as u32;
        }

        let sector_offset = (offset % self.bytes_per_cluster) / config::SECTOR_BYTES;
        let byte_offset = offset % config::SECTOR_BYTES;

        Some((target_cluster, sector_offset, byte_offset))
    }
}