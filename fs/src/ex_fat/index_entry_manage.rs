use crate::config;
use crate::ex_fat::boot_sector::BootSector;
use crate::ex_fat::model::up_case_table::{FileNameHash, UpCaseTable};
use crate::ex_fat::cluster_chain::ClusterManager;
use crate::ex_fat::model::cluster_id::ClusterId;
use crate::ex_fat::model::index_entry::{Attributes, IndexEntry, IndexEntryType};
use crate::ex_fat::model::unicode_str::UnicodeString;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cmp::min;
use spin::rwlock::RwLockWriteGuard;
use spin::RwLock;
use crate::ex_fat::{FileDirMetadata, MetadataType};

pub struct IndexEntryManager {
    /// 根目录元数据
    root_dir_start_cluster: ClusterId,
    /// 大写表
    up_case_table: UpCaseTable,
    /// 簇管理器
    cluster_manager: Arc<RwLock<ClusterManager>>,

    /// 每簇能存储的目录项数
    entry_per_cluster: usize,
}

impl IndexEntryManager {
    pub fn new(boot_sector: &BootSector, up_case_table: UpCaseTable, cluster_manager: Arc<RwLock<ClusterManager>>) -> Self {
        // 获取根目录元数据
        let root_dir_start_cluster = ClusterId(boot_sector.first_cluster_of_root_directory);

        // 每簇能存储的目录项数
        let entry_per_cluster = ((1usize << boot_sector.sectors_per_cluster_shift) * config::SECTOR_BYTES) >> 5;

        IndexEntryManager {
            root_dir_start_cluster,
            up_case_table,
            cluster_manager,
            entry_per_cluster,
        }
    }

    /// 内部函数：查找指定名字的目录项集合
    ///
    /// 无内部检查，调用前需确保参数有效
    fn find_entry_set_by_name(&self, parent_dir: &MetadataType, name: &UnicodeString) -> Option<Vec<IndexEntry>> {
        // 流程：
        // 1. 确定簇链起始簇号及是否连续
        // 2. 遍历簇链中每个扇区，查找目标文件名的哈希值

        // 1. 确定簇链起始簇号及是否连续
        // 起始簇号、是否连续、目录占用的簇数
        let start_cluster_id: ClusterId;
        let is_fragment: bool;
        let dir_cluster_count: u32;

        match parent_dir {
            MetadataType::FileOrDir(metadata) => {
                assert!(metadata.attributes.contains(Attributes::Directory), "Cannot search in File"); // 不能对文件进行查找
                // 取得文件夹的起始簇号
                start_cluster_id = metadata.first_cluster.clone();
                is_fragment = metadata.is_fragment;
                dir_cluster_count = {
                    let bytes_per_cluster = self.entry_per_cluster << 5;
                    ((metadata.size + bytes_per_cluster - 1) / bytes_per_cluster) as u32
                };
            }
            MetadataType::Root => {
                // 取得根目录的起始簇号
                start_cluster_id = self.root_dir_start_cluster.clone();
                is_fragment = true;
                dir_cluster_count = 0;
            }
        }

        let cluster_manager = self.cluster_manager.read();
        // 返回值
        let mut result: Option<Vec<IndexEntry>> = None;
        // 目标文件名的哈希值
        let target_hash = {
            let mut hash = FileNameHash(0);
            hash.add_uni_string(&self.up_case_table, name);
            hash
        };
        // 暂存当前文件名
        let mut now_name = UnicodeString::new();
        // 暂存当前文件名长度
        let mut now_name_len = 0;
        // 暂存当前簇号
        let mut now_cluster_id = start_cluster_id;
        // 暂存当前文件目录项起始位置
        let mut now_entry_set = Vec::new();

        // 流程控制变量：是否直接查找下一个85H目录项
        let mut find_next_85h = true;
        // 流程控制变量：是否结束查找
        let mut end_search = false;

        // 遍历簇链
        'outer: while !now_cluster_id.is_invalid() {
            // 遍历簇链中每个扇区
            for sector_offset in 0..cluster_manager.sectors_per_cluster {
                let Some(sector) = cluster_manager.get_cluster_sector(&now_cluster_id, sector_offset) else {
                    panic!("Failed to get cluster sector");
                };

                // 遍历扇区中每个目录项
                sector.read().read(0, |entries: &[IndexEntry; config::SECTOR_BYTES >> 5]| {
                    for (index, entry) in entries.iter().enumerate() {
                        match entry.entry_type {
                            IndexEntryType::ExfatUnused => {
                                // 未使用的目录项，表示后面没有目录项了
                                end_search = true;
                                break;
                            }
                            IndexEntryType::ExfatFile => {
                                // 文件目录项
                                // 清除当前目录项集合
                                now_entry_set.clear();
                                // 添加当前目录项
                                now_entry_set.push(entry.clone());
                                // 清除当前文件名
                                now_name.clear();
                                // 重置查找下一个85H目录项标记
                                find_next_85h = false;
                            }
                            IndexEntryType::ExfatStream => {
                                // 流扩展目录项
                                // 比对文件名哈希值
                                if unsafe { entry.custom_defined.stream.file_name_hash } != target_hash {
                                    // 哈希值不匹配
                                    // 标记为查找下一个85H目录项
                                    find_next_85h = true;
                                    continue;
                                }
                                // 哈希比对通过，更新当前文件名长度
                                now_name_len = unsafe { entry.custom_defined.stream.file_name_length };
                                // 添加当前目录项
                                now_entry_set.push(entry.clone());
                            }
                            IndexEntryType::ExfatName => {
                                // 名称目录项
                                // 若标记为查找下一个85H目录项，则跳过
                                if find_next_85h {
                                    continue;
                                }

                                // 读取文件名
                                let name_bytes = unsafe { entry.custom_defined.name.name };
                                let len = min(name_bytes.len(), now_name_len as usize); // 读入的文件名长度
                                for i in 0..len {
                                    now_name.push(name_bytes[i]);
                                }
                                // 添加当前目录项
                                now_entry_set.push(entry.clone());

                                if now_name.len() == now_name_len as usize {
                                    // 文件名读取完毕，比对文件名
                                    if now_name == *name {
                                        // 文件名匹配
                                        // 返回目录项位置
                                        result = Some(now_entry_set.clone());
                                        end_search = true;
                                        break;
                                    }
                                }
                            }
                            _ => {
                                // 无关目录项，跳过
                            }
                        }
                    }
                });

                // 若终止搜索，则跳出循环
                if end_search { break 'outer; }
            }

            // 前往下一簇
            if is_fragment {
                // 非连续簇链，需要通过FAT表获取下一簇
                if let Some(next_cluster_id) = cluster_manager.get_next_cluster(&now_cluster_id) {
                    if !next_cluster_id.is_invalid() {
                        // 有下一簇，继续搜索
                        now_cluster_id = next_cluster_id;
                        continue;
                    } else {
                        // 无下一簇，结束搜索
                        break;
                    }
                }
            } else {
                // 连续簇链，直接前往下一簇
                now_cluster_id.0 += 1;
                if now_cluster_id.0 >= start_cluster_id.0 + dir_cluster_count {
                    // 已经到达最后一簇，结束搜索
                    break;
                }
            }
        }

        result
    }

    /// 查：查找指定名字的元数据
    pub fn find_metadata_by_name(&self, parent_dir: &MetadataType, target_uni_name: &UnicodeString) -> Option<MetadataType> {
        let entry_set = self.find_entry_set_by_name(parent_dir, target_uni_name)?;

        if let Some(metadata) = FileDirMetadata::from_entry_set(&entry_set) {
            Some(MetadataType::FileOrDir(metadata))
        } else {
            None
        }
    }


    /// 内部函数：查找指定名字的目录项位置
    ///
    /// 无内部检查，调用前需确保参数有效
    fn find_first_entry_pos(&self, parent_dir: &MetadataType, target_uni_name: &UnicodeString) -> Option<(ClusterId, usize, usize)> {
        // 1. 确定簇链起始簇号及是否连续
        // 起始簇号、是否连续、目录占用的簇数
        let start_cluster_id: ClusterId;
        let is_fragment: bool;
        let dir_cluster_count: u32;

        match parent_dir {
            MetadataType::FileOrDir(metadata) => {
                assert!(metadata.attributes.contains(Attributes::Directory), "Cannot search in File"); // 不能对文件进行查找
                // 取得文件夹的起始簇号
                start_cluster_id = metadata.first_cluster.clone();
                is_fragment = metadata.is_fragment;
                dir_cluster_count = {
                    let bytes_per_cluster = self.entry_per_cluster << 5;
                    ((metadata.size + bytes_per_cluster - 1) / bytes_per_cluster) as u32
                };
            }
            MetadataType::Root => {
                // 取得根目录的起始簇号
                start_cluster_id = self.root_dir_start_cluster.clone();
                is_fragment = true;
                dir_cluster_count = 0;
            }
        }

        let cluster_manager = self.cluster_manager.read();
        // 返回值
        let mut result: Option<(ClusterId, usize, usize)> = None;
        // 目标文件名的哈希值
        let target_hash = {
            let mut hash = FileNameHash(0);
            hash.add_uni_string(&self.up_case_table, target_uni_name);
            hash
        };
        // 暂存当前文件名
        let mut now_name = UnicodeString::new();
        // 暂存当前文件名长度
        let mut now_name_len = 0;
        // 暂存当前簇号
        let mut now_cluster_id = start_cluster_id;
        // 暂存当前文件目录项起始位置
        let mut now_first_entry_pos = (ClusterId::eof(), 0, 0);
        // 暂存当前文件目录项条数
        let mut now_entry_count = 0;

        // 流程控制变量：是否直接查找下一个85H目录项
        let mut find_next_85h = true;
        // 流程控制变量：是否结束查找
        let mut end_search = false;

        // 遍历簇链
        'outer: while !now_cluster_id.is_invalid() {
            // 遍历簇链中每个扇区
            for sector_offset in 0..cluster_manager.sectors_per_cluster {
                let Some(sector) = cluster_manager.get_cluster_sector(&now_cluster_id, sector_offset) else {
                    panic!("Failed to get cluster sector");
                };

                // 遍历扇区中每个目录项
                sector.read().read(0, |entries: &[IndexEntry; config::SECTOR_BYTES >> 5]| {
                    for (index, entry) in entries.iter().enumerate() {
                        match entry.entry_type {
                            IndexEntryType::ExfatUnused => {
                                // 未使用的目录项，表示后面没有目录项了
                                end_search = true;
                                break;
                            }
                            IndexEntryType::ExfatFile => {
                                // 文件目录项
                                // 记录当前目录项位置
                                now_first_entry_pos = (now_cluster_id.clone(), sector_offset, index);
                                // 记录当前文件的目录项条数
                                now_entry_count = unsafe { entry.custom_defined.file.secondary_count + 1 };
                                // 清除当前文件名
                                now_name.clear();
                                // 重置查找下一个85H目录项标记
                                find_next_85h = false;
                            }
                            IndexEntryType::ExfatStream => {
                                // 流扩展目录项
                                // 比对文件名哈希值
                                if unsafe { entry.custom_defined.stream.file_name_hash } != target_hash {
                                    // 哈希值不匹配
                                    // 标记为查找下一个85H目录项
                                    find_next_85h = true;
                                    continue;
                                }
                                // 哈希比对通过，更新当前文件名长度
                                now_name_len = unsafe { entry.custom_defined.stream.file_name_length };
                            }
                            IndexEntryType::ExfatName => {
                                // 名称目录项
                                // 若标记为查找下一个85H目录项，则跳过
                                if find_next_85h {
                                    continue;
                                }

                                // 读取文件名
                                let name_bytes = unsafe { entry.custom_defined.name.name };
                                let len = min(name_bytes.len(), now_name_len as usize); // 读入的文件名长度
                                for i in 0..len {
                                    now_name.push(name_bytes[i]);
                                }

                                if now_name.len() == now_name_len as usize {
                                    // 文件名读取完毕，比对文件名
                                    if now_name == *target_uni_name {
                                        // 文件名匹配
                                        // 返回目录项位置
                                        result = Some(now_first_entry_pos);
                                        end_search = true;
                                        break;
                                    }
                                }
                            }
                            _ => {
                                // 无关目录项，跳过
                            }
                        }
                    }
                });

                // 若终止搜索，则跳出循环
                if end_search { break 'outer; }
            }

            // 前往下一簇
            if is_fragment {
                // 非连续簇链，需要通过FAT表获取下一簇
                if let Some(next_cluster_id) = cluster_manager.get_next_cluster(&now_cluster_id) {
                    if !next_cluster_id.is_invalid() {
                        // 有下一簇，继续搜索
                        now_cluster_id = next_cluster_id;
                        continue;
                    } else {
                        // 无下一簇，结束搜索
                        break;
                    }
                }
            } else {
                // 连续簇链，直接前往下一簇
                now_cluster_id.0 += 1;
                if now_cluster_id.0 >= start_cluster_id.0 + dir_cluster_count {
                    // 已经到达最后一簇，结束搜索
                    break;
                }
            }
        }

        result
    }


    /// 增：创建新文件/文件夹
    pub fn create_entries(&mut self, parent_dir: &mut MetadataType, to_create: &mut MetadataType) -> Option<()> {
        // 流程：
        // 1. 在上层目录中查找文件名是否已存在
        // 2. 计算文件名哈希值，更新到metadata，生成目录项
        // 3. 遍历目录项，找到空闲目录项
        // 4. 若找不到空闲目录项，或空闲空间不足以存放目录项，则为文件夹分配新簇
        // 5. 从空闲目录项开始写入文件信息

        // 1. 在上层目录中查找文件名是否已存在
        if self.find_first_entry_pos(&parent_dir, match to_create {
            MetadataType::FileOrDir(file_metadata) => &file_metadata.name,
            MetadataType::Root => {
                // 根目录不允许创建
                panic!("Cannot create root directory");
            }
        }).is_some() {
            // 文件名已存在
            panic!("File name already exists");
        }

        // 2. 计算文件名哈希值并更新进入metadata，生成目录项
        let entry_set = match to_create {
            MetadataType::FileOrDir(file_metadata) => {
                let mut hash = FileNameHash(0);
                hash.add_uni_string(&self.up_case_table, &file_metadata.name);
                file_metadata.name_hash = hash;
                file_metadata.to_entry_set()
            }
            _ => unreachable!()
        };

        let mut cluster_manager = self.cluster_manager.write();

        // 3. 遍历目录项，找到空闲目录项
        let parent_start_cluster_id: ClusterId;
        let mut parent_is_fragment: bool;
        let parent_dir_cluster_count: u32;

        match parent_dir {
            MetadataType::FileOrDir(metadata) => {
                // 取得文件夹的起始簇号
                parent_start_cluster_id = metadata.first_cluster.clone();
                parent_is_fragment = metadata.is_fragment;
                parent_dir_cluster_count = {
                    let bytes_per_cluster = self.entry_per_cluster << 5;
                    ((metadata.size + bytes_per_cluster - 1) / bytes_per_cluster) as u32
                };
            }
            MetadataType::Root => {
                // 取得根目录的起始簇号
                parent_start_cluster_id = self.root_dir_start_cluster.clone();
                parent_is_fragment = true;
                parent_dir_cluster_count = 0;
            }
        }

        let mut now_cluster_id = parent_start_cluster_id.clone();

        // 空闲目录项的位置
        let mut position: Option<(ClusterId, usize, usize)> = None;

        // 查找空闲目录项
        {
            // 流程控制变量：是否结束查找
            let mut end_search = false;
            // 遍历簇链
            'outer: while !now_cluster_id.is_invalid() {
                // 遍历簇链中每个扇区
                for sector_offset in 0..cluster_manager.sectors_per_cluster {
                    let Some(sector) = cluster_manager.get_cluster_sector(&now_cluster_id, sector_offset) else {
                        panic!("Failed to get cluster sector");
                    };

                    // 遍历扇区中每个目录项
                    sector.read().read(0, |entries: &[IndexEntry; config::SECTOR_BYTES >> 5]| {
                        for (index, entry) in entries.iter().enumerate() {
                            match entry.entry_type {
                                IndexEntryType::ExfatUnused => {
                                    // 未使用的目录项，表示后面没有目录项了
                                    position = Some((now_cluster_id.clone(), sector_offset, index));
                                    end_search = true;
                                    break;
                                }
                                _ => {
                                    // 无关目录项，跳过
                                }
                            }
                        }
                    });

                    // 若终止搜索，则跳出循环
                    if end_search { break 'outer; }
                }

                // 前往下一簇
                if parent_is_fragment {
                    // 非连续簇链，需要通过FAT表获取下一簇
                    if let Some(next_cluster_id) = cluster_manager.get_next_cluster(&now_cluster_id) {
                        if !next_cluster_id.is_invalid() {
                            // 有下一簇，继续搜索
                            now_cluster_id = next_cluster_id;
                            continue;
                        } else {
                            // 无下一簇，结束搜索
                            break;
                        }
                    }
                } else {
                    // 连续簇链，直接前往下一簇
                    now_cluster_id.0 += 1;
                    if now_cluster_id.0 >= parent_start_cluster_id.0 + parent_dir_cluster_count {
                        // 已经到达最后一簇，结束搜索
                        break;
                    }
                }
            }
        }

        // 4. 若找不到空闲目录项，或空闲空间不足以存放目录项，则为文件夹分配新簇
        if position.is_none() || {
            let (_, sector_offset, offset) = position.unwrap();
            let now_entry_offset = sector_offset * (config::SECTOR_BYTES >> 5) + offset;
            entry_set.len() > (self.entry_per_cluster - now_entry_offset)
        } {
            // 为文件夹分配新簇
            let Some((new_cluster_id, is_fragment)) = cluster_manager.alloc_new_cluster(
                &(if parent_start_cluster_id.is_invalid() {
                    // 若文件夹尚未分配簇，则hint为EOF
                    ClusterId::eof()
                } else {
                    // 文件夹已分配簇，则hint为当前簇链的末尾的下一簇
                    ClusterId(now_cluster_id.0 + 1)
                }), 1, parent_is_fragment) else {
                panic!("Failed to allocate new cluster");
            };

            if position.is_none() {
                // 未找到空闲位置，说明目录项需要写入新簇
                position = Some((new_cluster_id.clone(), 0, 0));
            }

            // 原簇链为空，更新父目录的起始簇号
            // 原簇链非空，视情况更新FAT表
            //   - 若原簇链为连续簇链，而新簇为非连续簇链，则在FAT中注册之前的连续簇链，并将新簇链挂至原簇链的末尾
            //   - 若原簇链为非连续簇链，新簇也为非连续簇链，则将新簇链挂至原簇链的末尾

            let MetadataType::FileOrDir(metadata) = parent_dir else {
                unreachable!()
            };

            if parent_start_cluster_id.is_eof() {
                // 更新起始簇号
                metadata.first_cluster = new_cluster_id.clone();
            } else {
                if parent_is_fragment != is_fragment {
                    // 原簇链与新簇链的连续性不同（只可能是连续->非连续）
                    // 在FAT表中注册之前的连续簇链
                    cluster_manager.set_continued_cluster_chain(&parent_start_cluster_id, parent_dir_cluster_count as usize);
                }

                if is_fragment {
                    // 新簇链为非连续簇链，挂至原簇链的末尾
                    cluster_manager.set_next_cluster(&now_cluster_id, &new_cluster_id);
                }
            }

            metadata.is_fragment = is_fragment;
            parent_is_fragment = is_fragment;
        }

        // 4. 从空闲目录项开始写入文件信息
        {
            // 流程控制变量：是否结束查找
            let mut end_procedure = false;
            let (mut now_cluster_id, mut sector_offset, mut inner_offset) = position.unwrap();
            let mut entry_index = 0;
            while entry_index < entry_set.len() {
                let Some(sector) = cluster_manager.get_cluster_sector(&now_cluster_id, sector_offset) else {
                    panic!("Failed to get cluster sector");
                };

                sector.write().modify(0, |entries: &mut [IndexEntry; config::SECTOR_BYTES >> 5]| {
                    while inner_offset < (config::SECTOR_BYTES >> 5) {
                        if entry_index >= entry_set.len() {
                            end_procedure = true;
                            break;
                        }

                        entries[inner_offset] = entry_set[entry_index].clone();
                        entry_index += 1;
                        inner_offset += 1;
                    }
                });

                if end_procedure { break; }

                if inner_offset >= (config::SECTOR_BYTES >> 5) {
                    // 扇区写满，前往下一扇区
                    inner_offset = 0;
                    sector_offset += 1;
                    if sector_offset >= cluster_manager.sectors_per_cluster {
                        // 簇写满，前往下一簇
                        sector_offset = 0;
                        // 根据是否连续簇链，决定下一簇号的获取方式
                        if parent_is_fragment {
                            let Some(next_cluster_id) = cluster_manager.get_next_cluster(&now_cluster_id) else {
                                unreachable!();
                            };
                            now_cluster_id = next_cluster_id;
                        } else {
                            now_cluster_id.0 += 1;
                        }
                    }
                }
            }
        }

        //更新文件夹大小
        match parent_dir {
            MetadataType::FileOrDir(metadata) => {
                metadata.size += entry_set.len() << 5;
            }
            _ => { /* do nothing */ }
        }

        Some(())
    }

    /// 内部函数：删除指定位置的目录项
    ///
    /// 无内部检查，调用前需确保参数有效
    fn delete_entries_by_pos(cluster_manager: &mut RwLockWriteGuard<ClusterManager>, first_entry_pos: (ClusterId, usize, usize), is_fragment: bool) -> Option<()> {
        let mut delete_count = 0;

        let (mut now_cluster_id, mut sector_offset, mut inner_offset) = first_entry_pos;

        let len = {
            let sector = cluster_manager.get_cluster_sector(&now_cluster_id, sector_offset).unwrap();

            let ret = sector.read().read(0, |entries: &[IndexEntry; config::SECTOR_BYTES >> 5]| {
                assert_eq!(entries[inner_offset].entry_type, IndexEntryType::ExfatFile);    // pos指向的目录项必须是文件目录项
                unsafe { entries[inner_offset].custom_defined.file.secondary_count + 1 }
            });

            ret
        };

        let mut end_procedure = false;

        while delete_count < len {
            let sector = cluster_manager.get_cluster_sector(&now_cluster_id, sector_offset).unwrap();

            sector.write().modify(0, |entries: &mut [IndexEntry; config::SECTOR_BYTES >> 5]| {
                while inner_offset < (config::SECTOR_BYTES >> 5) {
                    if delete_count >= len {
                        end_procedure = true;
                        break;
                    }

                    entries[inner_offset].entry_type.deleted(true);

                    delete_count += 1;
                    inner_offset += 1;
                }
            });

            if end_procedure { break; }

            if inner_offset >= (config::SECTOR_BYTES >> 5) {
                // 扇区写满，前往下一扇区
                inner_offset = 0;
                sector_offset += 1;
                if sector_offset >= cluster_manager.sectors_per_cluster {
                    // 簇写满，前往下一簇
                    sector_offset = 0;
                    // 根据是否连续簇链，决定下一簇号的获取方式
                    if is_fragment {
                        let Some(next_cluster_id) = cluster_manager.get_next_cluster(&now_cluster_id) else {
                            unreachable!();
                        };
                        now_cluster_id = next_cluster_id;
                    } else {
                        now_cluster_id.0 += 1;
                    }
                }
            }
        }

        Some(())
    }

    /// 删：删除文件/文件夹的目录项
    pub fn delete_entries(&mut self, parent_dir: &MetadataType, target_uni_name: &UnicodeString) -> Option<()> {
        // 1. 在上层目录中查找文件目录项位置
        // 2. 删除目录项

        let pos_res = self.find_first_entry_pos(parent_dir, target_uni_name);

        let mut cluster_manager = self.cluster_manager.write();


        if pos_res.is_some() {
            // 删除目录项
            let is_fragment: bool;
            match parent_dir {
                MetadataType::FileOrDir(metadata) => {
                    assert!(metadata.attributes.contains(Attributes::Directory), "Cannot search in File"); // 不能对文件进行查找
                    is_fragment = metadata.is_fragment;
                }
                MetadataType::Root => {
                    is_fragment = true;
                }
            }
            Self::delete_entries_by_pos(&mut cluster_manager, pos_res.unwrap(), is_fragment)
        } else {
            // 未找到目录项
            panic!("File not found");
        }
    }

    /// 内部函数：修改指定位置的目录项
    ///
    /// 无内部检查，调用前需确保参数有效
    fn modify_entries_by_pos(cluster_manager: &mut RwLockWriteGuard<ClusterManager>, first_entry_pos: (ClusterId, usize, usize), is_fragment: bool, entry_set: Vec<IndexEntry>) -> Option<()> {
        let (mut now_cluster_id, mut sector_offset, mut inner_offset) = first_entry_pos;

        // 覆写第一个目录项

        let sector = cluster_manager.get_cluster_sector(&now_cluster_id, sector_offset).unwrap();
        sector.write().modify(0, |entries: &mut [IndexEntry; config::SECTOR_BYTES >> 5]| {
            entries[inner_offset] = entry_set[0].clone();
        });

        // 移动到下一个目录项
        inner_offset += 1;
        if inner_offset >= (config::SECTOR_BYTES >> 5) {
            // 扇区写满，前往下一扇区
            inner_offset = 0;
            sector_offset += 1;
            if sector_offset >= cluster_manager.sectors_per_cluster {
                // 簇写满，前往下一簇
                sector_offset = 0;
                // 根据是否连续簇链，决定下一簇号的获取方式
                if is_fragment {
                    let Some(next_cluster_id) = cluster_manager.get_next_cluster(&now_cluster_id) else {
                        unreachable!();
                    };
                    now_cluster_id = next_cluster_id;
                } else {
                    now_cluster_id.0 += 1;
                }
            }
        }

        let sector = cluster_manager.get_cluster_sector(&now_cluster_id, sector_offset).unwrap();
        sector.write().modify(0, |entries: &mut [IndexEntry; config::SECTOR_BYTES >> 5]| {
            entries[inner_offset] = entry_set[1].clone();
        });

        Some(())
    }


    /// 改：修改文件/文件夹目录项（不包括重命名）
    pub fn modify_entries(&mut self, parent_dir: &MetadataType, new_metadata: &MetadataType) -> Option<()> {
        // 1. 在上层目录中查找文件目录项位置
        // 2. 删除目录项

        let is_fragment: bool;
        let target_uni_name: UnicodeString;
        let entry_set: Vec<IndexEntry>;

        match parent_dir {
            MetadataType::FileOrDir(metadata) => {
                assert!(metadata.attributes.contains(Attributes::Directory), "Cannot search in File"); // 不能对文件进行查找
                is_fragment = metadata.is_fragment;
            }
            MetadataType::Root => {
                is_fragment = true;
            }
        }

        match new_metadata {
            MetadataType::FileOrDir(file_metadata) => {
                target_uni_name = file_metadata.name.clone();
                entry_set = file_metadata.to_entry_set();
            }
            _ => unreachable!()
        }

        let pos_res = self.find_first_entry_pos(parent_dir, &target_uni_name);

        let mut cluster_manager = self.cluster_manager.write();

        if pos_res.is_some() {
            // 修改目录项
            Self::modify_entries_by_pos(&mut cluster_manager, pos_res.unwrap(), is_fragment, entry_set)
        } else {
            // 未找到目录项
            panic!("File not found");
        }
    }

    /// 额外：列出目录下所有文件/文件夹
    pub fn list_metadata(&self, parent_dir: &MetadataType) -> Vec<FileDirMetadata> {
        // 流程：
        // 1. 确定簇链起始簇号及是否连续
        // 2. 遍历簇链中每个扇区，查找目标文件名的哈希值
        // 3. 读取文件名，生成元数据

        // 1. 确定簇链起始簇号及是否连续
        // 起始簇号、是否连续、目录占用的簇数
        let start_cluster_id: ClusterId;
        let is_fragment: bool;
        let dir_cluster_count: u32;

        match parent_dir {
            MetadataType::FileOrDir(metadata) => {
                assert!(metadata.attributes.contains(Attributes::Directory), "Cannot search in File"); // 不能对文件进行查找
                // 取得文件夹的起始簇号
                start_cluster_id = metadata.first_cluster.clone();
                is_fragment = metadata.is_fragment;
                dir_cluster_count = {
                    let bytes_per_cluster = self.entry_per_cluster << 5;
                    ((metadata.size + bytes_per_cluster - 1) / bytes_per_cluster) as u32
                };
            }
            MetadataType::Root => {
                // 取得根目录的起始簇号
                start_cluster_id = self.root_dir_start_cluster.clone();
                is_fragment = true;
                dir_cluster_count = 0;
            }
        }

        let cluster_manager = self.cluster_manager.read();
        // 返回值
        let mut result: Vec<FileDirMetadata> = Vec::new();
        // 暂存当前文件目录项
        let mut now_entry_set = Vec::new();
        // 暂存当前簇号
        let mut now_cluster_id = start_cluster_id;
        // 暂存当前文件目录项有多少条
        let mut now_entry_count = 0;

        // 流程控制变量：是否结束查找
        let mut end_search = false;

        // 遍历簇链
        'outer: while !now_cluster_id.is_invalid() {
            // 遍历簇链中每个扇区
            for sector_offset in 0..cluster_manager.sectors_per_cluster {
                let sector = cluster_manager.get_cluster_sector(&now_cluster_id, sector_offset).unwrap();

                // 遍历扇区中每个目录项
                sector.read().read(0, |entries: &[IndexEntry; config::SECTOR_BYTES >> 5]| {
                    for (index, entry) in entries.iter().enumerate() {
                        match entry.entry_type {
                            IndexEntryType::ExfatUnused => {
                                // 未使用的目录项，表示后面没有目录项了
                                end_search = true;
                                break;
                            }
                            IndexEntryType::ExfatFile => {
                                // 文件目录项
                                // 清除当前目录项集合
                                now_entry_set.clear();
                                // 记录当前文件的目录项条数
                                now_entry_count = unsafe { entry.custom_defined.file.secondary_count + 1 } as usize;
                                // 添加当前目录项
                                now_entry_set.push(entry.clone());
                            }
                            IndexEntryType::ExfatStream => {
                                // 流扩展目录项
                                // 添加当前目录项
                                now_entry_set.push(entry.clone());
                            }
                            IndexEntryType::ExfatName => {
                                // 名称目录项
                                // 添加当前目录项
                                now_entry_set.push(entry.clone());

                                if now_entry_set.len() >= now_entry_count {
                                    // 读取完毕，生成元数据
                                    if let Some(metadata) = FileDirMetadata::from_entry_set(&now_entry_set)
                                    {
                                        result.push(metadata);
                                    }
                                }
                            }
                            _ => {
                                // 无关目录项，跳过
                            }
                        }
                    }
                });

                // 若终止搜索，则跳出循环
                if end_search { break 'outer; }
            }

            // 前往下一簇
            if is_fragment {
                // 非连续簇链，需要通过FAT表获取下一簇
                if let Some(next_cluster_id) = cluster_manager.get_next_cluster(&now_cluster_id) {
                    if !next_cluster_id.is_invalid() {
                        // 有下一簇，继续搜索
                        now_cluster_id = next_cluster_id;
                        continue;
                    } else {
                        // 无下一簇，结束搜索
                        break;
                    }
                }
            } else {
                // 连续簇链，直接前往下一簇
                now_cluster_id.0 += 1;
                if now_cluster_id.0 >= start_cluster_id.0 + dir_cluster_count {
                    // 已经到达最后一簇，结束搜索
                    break;
                }
            }
        }

        result
    }

    /// 将簇内目录项偏移转换为簇内扇区偏移和扇区内目录项偏移
    fn translate_entry_offset_pos(&self, offset: usize) -> (usize, usize) {
        (offset / (config::SECTOR_BYTES >> 5), offset % (config::SECTOR_BYTES >> 5))
    }

    /// 整理簇链中的目录项
    /// 
    /// 返回删除了多少个目录项、剩余多少个目录项
    /// 
    /// TODO: 未完成
    pub fn tidy_up(&self, wrapped_parent_dir_metadata: &Option<MetadataType>, dir_metadata: &mut MetadataType) -> Option<(usize, usize)> {
        // 流程：
        // 确定文件夹的起始簇号及是否连续
        // 遍历簇链中的每个扇区，将目录项整理到一起
        // 维护一个指针dst，指向当前要写入的目录项位置（第一个被标记为deleted的目录项）
        // 维护另一个指针src，指向当前遍历的目录项位置
        // 若dst不为None，则将src指向的目录项复制到dst指向的目录项位置，并将src指向的目录项标记为deleted
        // src移动的一定比dst快，直到遇到0x00目录项
        // 之后，dst指针继续向后遍历，将后续的所有deleted目录项标记为unused

        let start_cluster_id: ClusterId;
        let is_fragment: bool;
        let dir_cluster_count: u32;

        match dir_metadata {
            MetadataType::FileOrDir(metadata) => {
                assert!(metadata.attributes.contains(Attributes::Directory), "Cannot tidy up in File"); // 不能对文件进行整理
                // 取得文件夹的起始簇号
                start_cluster_id = metadata.first_cluster.clone();
                is_fragment = metadata.is_fragment;
                dir_cluster_count = {
                    let bytes_per_cluster = self.entry_per_cluster << 5;
                    ((metadata.size + bytes_per_cluster - 1) / bytes_per_cluster) as u32
                };
            }
            MetadataType::Root => {
                // 取得根目录的起始簇号
                start_cluster_id = self.root_dir_start_cluster.clone();
                is_fragment = true;
                dir_cluster_count = 0;
            }
        }
        
        let mut cluster_manager = self.cluster_manager.write();
        
        let mut counter = 0;
        let mut delete_count = 0;
        
        let mut dst_pos: Option<(ClusterId, usize, usize)> = None;
        let mut src_pos: (ClusterId, usize, usize) = (start_cluster_id, 0, 0);
        
        let mut end_move = false;
        
        // 在src尚未遇到0x00目录项之前，移动目录项
        
        Some((delete_count, counter))
    }
}