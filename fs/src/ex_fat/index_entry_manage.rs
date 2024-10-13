use crate::config;
use crate::ex_fat::persistent_layer::model::cluster_id::ClusterId;
use crate::ex_fat::persistent_layer::model::index_entry::file_directory::FileDirectoryCostume;
use crate::ex_fat::persistent_layer::model::index_entry::file_directory::{FileAttributes, FragmentFlag};
use crate::ex_fat::persistent_layer::model::index_entry::IndexEntryChecksum;
use crate::ex_fat::persistent_layer::model::index_entry::{EntryCostume, IndexEntry};
use crate::ex_fat::persistent_layer::model::unicode_str::UnicodeString;
use crate::ex_fat::persistent_layer::up_case_table::{FileNameHash, UpCaseTable};
use crate::ex_fat::persistent_layer::ClusterManager;
use crate::BlockDevice;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cmp::min;
use spin::{RwLock, RwLockReadGuard};

/// 文件目录
#[derive(Debug)]
pub struct FileMetaData {
    pub file_attributes: FileAttributes,
    pub create_time_unix_ms_stamp: u64,
    pub last_modified_time_unix_stamp: u64,
    pub last_accessed_time_unix_stamp: u64,
    pub is_fragment: bool,
    pub file_name: UnicodeString,
    pub file_name_hash: FileNameHash,
    pub file_size: u64,
    pub first_cluster: Option<ClusterId>,

    /// 目录项首项位置（用于协助定位）
    pub index_position: (ClusterId, u32, u32),
}

impl FileMetaData {
    pub fn empty() -> Self {
        FileMetaData {
            file_attributes: FileAttributes::from_bits(0).unwrap(),
            create_time_unix_ms_stamp: 0,
            last_modified_time_unix_stamp: 0,
            last_accessed_time_unix_stamp: 0,
            is_fragment: false,
            file_name: UnicodeString::new(),
            file_name_hash: FileNameHash(0),
            file_size: 0,
            first_cluster: None,
            index_position: (ClusterId::free(), 0, 0),
        }
    }

    pub fn is_directory(&self) -> bool {
        self.file_attributes.contains(FileAttributes::Directory)
    }

    pub fn is_read_only(&self) -> bool {
        self.file_attributes.contains(FileAttributes::ReadOnly)
    }

    pub fn is_hidden(&self) -> bool {
        self.file_attributes.contains(FileAttributes::Hidden)
    }

    pub fn is_system(&self) -> bool {
        self.file_attributes.contains(FileAttributes::System)
    }

    pub fn is_archive(&self) -> bool {
        self.file_attributes.contains(FileAttributes::Archive)
    }

    /// 从目录项位置列表构造文件
    pub fn from_entry_pos_list(cluster_manager: &RwLockReadGuard<ClusterManager>, entry_pos_list: Vec<(ClusterId, u32, u32)>) -> Option<Self> {
        let mut entries = Vec::new();

        // 读取目录项并计算校验和
        let mut checksum = IndexEntryChecksum(0);
        let mut target_checksum = IndexEntryChecksum(0);
        for (i, entry_pos) in entry_pos_list.iter().enumerate() {
            if let Some(block_cache) = cluster_manager.get_cluster_sector(&entry_pos.0, entry_pos.1) {
                block_cache.read().read((entry_pos.2 * 32) as usize, |index_entry_bytes: &[u8; 32]| {
                    let index_entry = IndexEntry::from_bytes(index_entry_bytes).unwrap();
                    if i == 0 {
                        let EntryCostume::FileDirectory(FileDirectoryCostume::Costume1(custom)) = &index_entry.custom_defined
                        else { unreachable!("Not a Costume1 entry") };
                        target_checksum = custom.set_check_sum.clone();
                    }
                    checksum.add_entry(index_entry_bytes, i == 0);
                    entries.push(index_entry.clone());
                })
            }
        }

        // 检查校验和
        assert_eq!(checksum.0, target_checksum.0);

        let mut file = FileMetaData::empty();

        let mut file_name_length = 0;

        file.index_position = (entry_pos_list[0].0.clone(), entry_pos_list[0].1, entry_pos_list[0].2);

        for (i, index_entry) in entries.iter().enumerate() {
            if i == 0 {
                // 属性项1
                if let EntryCostume::FileDirectory(FileDirectoryCostume::Costume1(custom)) = &index_entry.custom_defined {
                    file.file_attributes = custom.file_attributes.clone();
                    file.create_time_unix_ms_stamp = custom.create_time_stamp.to_unix_timestamp() * 1000 + custom.create_10ms_increment as u64 * 10;
                    file.last_modified_time_unix_stamp = custom.last_modified_time_stamp.to_unix_timestamp();
                    file.last_accessed_time_unix_stamp = custom.last_accessed_time_stamp.to_unix_timestamp();
                }
            } else if i == 1 {
                // 属性项2
                if let EntryCostume::FileDirectory(FileDirectoryCostume::Costume2(custom)) = &index_entry.custom_defined {
                    file.is_fragment = {
                        if custom.fragment_flag.contains(FragmentFlag::Continuous) {
                            false
                        } else if custom.fragment_flag.contains(FragmentFlag::Fragmented) {
                            true
                        } else {
                            unreachable!("Unknown fragment flag")
                        }
                    };
                    file_name_length = custom.file_name_length as usize;
                    file.file_name_hash = custom.file_name_hash.clone();
                    file.file_size = custom.file_size1;
                    if custom.file_size2 != 0 && custom.file_size1 != custom.file_size2 {
                        unreachable!("Mismatched file size")
                    }

                    if custom.start_cluster.0 != 0 {
                        file.first_cluster = Some(ClusterId::from(custom.start_cluster));
                    } else {
                        file.first_cluster = None;
                    }
                }
            } else if i < 19 {
                // 属性项3
                if let EntryCostume::FileDirectory(FileDirectoryCostume::Costume3(custom)) = &index_entry.custom_defined {
                    // 追加文件名（根据剩余长度决定追加长度，因为从custom直接取出的文件名长度为15）
                    let append_len = min(file_name_length - file.file_name.len(), 15);
                    file.file_name.append(&custom.file_name.slice(0, append_len));
                }
            } else {
                unreachable!("Too much entry!")
            }
        }

        Some(file)
    }

    pub fn to_entries(&self) -> Vec<IndexEntry> {
        let first_cluster = if self.first_cluster.is_some() {
            self.first_cluster.unwrap()
        } else {
            ClusterId::free()
        };


        IndexEntry::new_file_directory(
            self.file_attributes,
            self.create_time_unix_ms_stamp as usize,
            self.last_modified_time_unix_stamp as usize,
            self.last_accessed_time_unix_stamp as usize,
            self.is_fragment,
            self.file_name.clone(),
            self.file_name_hash.clone(),
            self.file_size,
            first_cluster,
        )
    }
}

pub struct IndexEntryManager {
    /// 大写字母表
    up_case_table: UpCaseTable,
    /// 簇管理器
    cluster_manager: Arc<RwLock<ClusterManager>>,
    /// 存储设备接口指针
    device: Arc<dyn BlockDevice>,
}

impl IndexEntryManager {
    pub fn new(up_case_table: UpCaseTable, cluster_manager: Arc<RwLock<ClusterManager>>, device: Arc<dyn BlockDevice>) -> Self {
        IndexEntryManager {
            up_case_table,
            cluster_manager,
            device,
        }
    }

    /// 在指定簇链中根据文件名查找第一个目录项的位置
    fn find_first_entry_pos_by_name(&self, cluster_id: &ClusterId, name: &UnicodeString) -> Option<(ClusterId, u32, u32)> {
        let cluster_manager = self.cluster_manager.read();
        let mut result: Option<(ClusterId, u32, u32)> = None;

        // 目标文件名的哈希值
        let target_hash = {
            let mut hash = FileNameHash(0);
            hash.add_chars(&self.up_case_table, name);
            hash
        };

        // 暂存当前文件名
        let mut now_name = UnicodeString::new();
        // 暂存当前簇号
        let mut now_cluster_id = cluster_id.clone();
        // 暂存当前簇内扇区偏移量
        let mut sector_offset_in_cluster = 0;
        // 暂存当前文件目录项起始位置
        let mut now_entry_first_pos: (ClusterId, u32, u32) = (ClusterId::free(), 0, 0);

        // 流程控制变量：是否直接查找下一个85H目录项
        let mut find_next_85h = true;
        // 流程控制变量：是否结束查找
        let mut end_procedure = false;

        // 遍历簇链中每个扇区
        loop {
            let block_cache = cluster_manager.get_cluster_sector(&now_cluster_id, sector_offset_in_cluster);
            if block_cache.is_none() {
                break;
            }
            let block_cache = block_cache.unwrap();

            // 遍历扇区中每个目录项
            block_cache.read().read(0, |index_entries: &[[u8; 32]; config::SECTOR_BYTES / 32]| {
                for (i, entry_bytes) in index_entries.iter().enumerate() {
                    let entry = IndexEntry::from_bytes(entry_bytes).unwrap();
                    // 分析其类型
                    match entry.entry_type.bits() {
                        0x00 => {
                            // 空的目录项，表示后面没有目录项了
                            end_procedure = true;
                            break;
                        }
                        0x85 => {
                            // 属性项1 目录项
                            // 重置流程控制变量
                            find_next_85h = false;
                            // 重置当前文件名
                            now_name.clear();
                            // 更新当前文件目录项起始位置
                            now_entry_first_pos = (now_cluster_id.clone(), sector_offset_in_cluster, i as u32);
                        }
                        0xC0 => {
                            // 属性项2 目录项
                            if find_next_85h {
                                continue;
                            }

                            // 比较文件名长度和哈希值
                            if let EntryCostume::FileDirectory(FileDirectoryCostume::Costume2(custom)) = entry.custom_defined.clone() {
                                if custom.file_name_length != name.len() as u8
                                    || custom.file_name_hash.0 != target_hash.0 {
                                    // 文件名长度或哈希值不匹配，跳过
                                    find_next_85h = true;
                                    continue;
                                }
                            } else {
                                unreachable!("Not a Costume2 entry");
                            }
                        }
                        0xC1 => {
                            // 属性项3 目录项
                            if find_next_85h {
                                continue;
                            }

                            // 取出文件名
                            if let EntryCostume::FileDirectory(FileDirectoryCostume::Costume3(custom)) = entry.custom_defined.clone() {
                                // 追加文件名（根据剩余长度决定追加长度，因为从custom直接取出的文件名长度为15）
                                let append_len = min(name.len() - now_name.len(), 15);
                                now_name.append(&custom.file_name.slice(0, append_len));
                            } else {
                                unreachable!("Not a Costume3 entry");
                            }

                            // append之后判断是否相等
                            if now_name == *name {
                                // 找到了目标文件
                                result = Some(now_entry_first_pos);
                                // 退出查找
                                end_procedure = true;
                                break;
                            }
                        }
                        _ => continue
                    }
                }
            });

            if end_procedure {
                break;
            }

            // 更新扇区偏移和簇号
            sector_offset_in_cluster += 1;
            if sector_offset_in_cluster >= cluster_manager.sectors_per_cluster {
                // 读取下一个簇
                let next_cluster_id = cluster_manager.get_next_cluster(&now_cluster_id);
                if next_cluster_id.is_none() {
                    break;
                }
                now_cluster_id = next_cluster_id.unwrap();
                sector_offset_in_cluster = 0;
            }
        }

        result
    }

    /// 创建新文件
    pub fn create_file(&mut self, file_meta_data: &mut FileMetaData, parent_cluster_id: &ClusterId) -> Option<(ClusterId, u32, u32)> {
        // 流程：
        // 1. 在指定簇链中查找文件名是否已存在，如果存在则直接返回
        // 2. 遍历目录项，找到空闲目录项
        // 3. 计算文件名哈希值
        // 4. 从空闲目录项开始写入文件信息

        // 1. 在指定粗链中查找文件名是否已存在
        if self.find_entry_by_name(parent_cluster_id, &file_meta_data.file_name).is_some() {
            // 文件名已存在
            return None;
        }

        let mut cluster_manager = self.cluster_manager.write();

        // 2. 在指定簇链中查找空闲目录项（可能需要新建簇）
        let mut result: Option<(ClusterId, u32, u32)> = None;

        // 暂存当前簇号
        let mut now_cluster_id = parent_cluster_id.clone();
        // 暂存当前簇内扇区偏移量
        let mut now_sector_offset_in_cluster = 0;
        // 流程控制变量：是否结束查找
        let mut end_procedure = false;

        // 遍历簇链中每个扇区，查找空闲目录项
        loop {
            let block_cache = cluster_manager.get_cluster_sector(&now_cluster_id, now_sector_offset_in_cluster);
            if block_cache.is_none() {
                break;
            }
            let block_cache = block_cache.unwrap();

            if result.is_none() {
                // 遍历扇区中每个目录项
                block_cache.read().read(0, |index_entries: &[[u8; 32]; config::SECTOR_BYTES / 32]| {
                    for (i, entry_bytes) in index_entries.iter().enumerate() {
                        let entry = IndexEntry::from_bytes(entry_bytes).unwrap();
                        // 分析其类型
                        match entry.entry_type.bits() {
                            0x00 => {
                                // 空的目录项，表示后面没有目录项了，可以使用
                                result = Some((now_cluster_id.clone(), now_sector_offset_in_cluster, i as u32));
                                end_procedure = true;
                                break;
                            }
                            _ => continue
                        }
                    }
                });
            }

            if end_procedure {
                break;
            }

            // 更新扇区偏移和簇号
            now_sector_offset_in_cluster += 1;
            if now_sector_offset_in_cluster >= cluster_manager.sectors_per_cluster {
                // 读取下一个簇
                let mut next_cluster_id = cluster_manager.get_next_cluster(&now_cluster_id);
                if next_cluster_id.is_none() {
                    // 需要新建簇
                    let new_cluster_id = cluster_manager.alloc_and_append_cluster(&now_cluster_id);
                    if new_cluster_id.is_none() {
                        return None;
                    } else {
                        next_cluster_id = new_cluster_id;
                    }
                }
                now_cluster_id = next_cluster_id.unwrap();
                now_sector_offset_in_cluster = 0;
            }
        }

        // 3. 计算并更新文件名哈希值
        let target_hash = {
            let mut hash = FileNameHash(0);
            hash.add_chars(&self.up_case_table, &file_meta_data.file_name);
            hash
        };
        file_meta_data.file_name_hash = target_hash;

        // 4. 从空闲目录项开始写入文件信息
        let result = result.unwrap();

        now_cluster_id = result.0;
        now_sector_offset_in_cluster = result.1;
        let mut now_entry_offset_in_sector = result.2;

        let entries = file_meta_data.to_entries();

        for entry in entries.iter() {
            cluster_manager.get_cluster_sector(&now_cluster_id, now_sector_offset_in_cluster).map(|block_cache| {
                block_cache.write().modify(now_entry_offset_in_sector as usize * 32, |index_entry: &mut [u8; 32]| {
                    index_entry.copy_from_slice(&entry.to_bytes());
                });
            });

            // 更新位置
            now_entry_offset_in_sector += 1;
            if now_entry_offset_in_sector >= (config::SECTOR_BYTES / 32) as u32 {
                now_entry_offset_in_sector = 0;
                now_sector_offset_in_cluster += 1;
                if now_sector_offset_in_cluster >= cluster_manager.sectors_per_cluster {
                    now_sector_offset_in_cluster = 0;
                    // 尝试获取下一个簇
                    if let Some(next_cluster_id) = cluster_manager.get_next_cluster(&now_cluster_id) {
                        now_cluster_id = next_cluster_id;
                    } else {
                        // 需要新建簇
                        let new_cluster_id = cluster_manager.alloc_and_append_cluster(&now_cluster_id);
                        if new_cluster_id.is_none() {
                            return None;
                        } else {
                            now_cluster_id = new_cluster_id.unwrap();
                        }
                    }
                }
            }
        }

        Some(result)
    }

    /// 根据名称在指定簇链中查找目录项
    pub fn find_entry_by_name(&self, cluster_id: &ClusterId, name: &UnicodeString) -> Option<FileMetaData> {
        if let Some(position) = self.find_first_entry_pos_by_name(cluster_id, name) {
            self.get_file_by_pos(&position.0, position.1, position.2)
        } else {
            None
        }
    }

    /// 根据目录项位置获取文件
    pub fn get_file_by_pos(&self, cluster_id: &ClusterId, sector_offset_in_cluster: u32, entry_offset_in_sector: u32) -> Option<FileMetaData> {
        let cluster_manager = self.cluster_manager.read();

        // 各目录项的位置
        let mut entry_positions = Vec::new();

        // 存储文件目录项数
        let mut entry_count = 0;

        // 获取目录项项数
        cluster_manager.get_cluster_sector(cluster_id, sector_offset_in_cluster).map(|block_cache| {
            block_cache.read().read(0, |index_entries: &[[u8; 32]; config::SECTOR_BYTES / 32]| {
                let entry = IndexEntry::from_bytes(&index_entries[entry_offset_in_sector as usize]).unwrap();
                // 分析其类型
                if entry.entry_type.bits() == 0x85 {
                    // 属性项1 目录项
                    if let EntryCostume::FileDirectory(FileDirectoryCostume::Costume1(custom)) = entry.custom_defined.clone() {
                        entry_count = custom.secondary_count + 1;
                    } else {
                        unreachable!("Not a Costume1 entry")
                    }
                } else {
                    unreachable!("Unknown entry type")
                }
            });
        });

        // 计算各目录项的位置
        let mut now_cluster_id = cluster_id.clone();
        let mut now_sector_offset_in_cluster = sector_offset_in_cluster;
        let mut now_entry_offset_in_sector = entry_offset_in_sector;

        // 属性项1
        entry_positions.push((now_cluster_id.clone(), now_sector_offset_in_cluster, now_entry_offset_in_sector));
        for _ in 1..entry_count {
            now_entry_offset_in_sector += 1;
            if now_entry_offset_in_sector >= (config::SECTOR_BYTES / 32) as u32 {
                now_entry_offset_in_sector = 0;
                now_sector_offset_in_cluster += 1;
                if now_sector_offset_in_cluster >= cluster_manager.sectors_per_cluster {
                    now_sector_offset_in_cluster = 0;
                    now_cluster_id = cluster_manager.get_next_cluster(&now_cluster_id).unwrap();
                }
            }

            entry_positions.push((now_cluster_id.clone(), now_sector_offset_in_cluster, now_entry_offset_in_sector));
        }

        FileMetaData::from_entry_pos_list(&cluster_manager, entry_positions)
    }

    /// 根据名称删除文件
    pub fn delete_file_by_name(&self, cluster_id: &ClusterId, name: &UnicodeString) {
        self.find_first_entry_pos_by_name(cluster_id, name).map(|entry_pos| {
            self.delete_file_by_pos(&entry_pos.0, entry_pos.1, entry_pos.2);
        });
    }

    /// 根据目录项位置删除文件
    pub fn delete_file_by_pos(&self, cluster_id: &ClusterId, sector_offset_in_cluster: u32, entry_offset_in_sector: u32) {
        let mut cluster_manager = self.cluster_manager.write();

        // 存储文件目录项数
        let mut entry_count = 0;

        // 获取目录项项数
        cluster_manager.get_cluster_sector(cluster_id, sector_offset_in_cluster).map(|block_cache| {
            block_cache.write().modify(0, |index_entries: &mut [[u8; 32]; config::SECTOR_BYTES / 32]| {
                let mut entry = IndexEntry::from_bytes(&index_entries[entry_offset_in_sector as usize]).unwrap();
                // 分析其类型
                if entry.entry_type.bits() == 0x85 {
                    // 属性项1 目录项
                    index_entries[entry_offset_in_sector as usize][0] = 0x05; // 标记为未使用
                    if let EntryCostume::FileDirectory(FileDirectoryCostume::Costume1(custom)) = &entry.custom_defined {
                        entry_count = custom.secondary_count + 1;
                    } else {
                        unreachable!("Not a Costume1 entry")
                    }
                } else {
                    unreachable!("Unknown entry type")
                }
            });
        });

        // 计算各目录项的位置
        let mut now_cluster_id = cluster_id.clone();
        let mut now_sector_offset_in_cluster = sector_offset_in_cluster;
        let mut now_entry_offset_in_sector = sector_offset_in_cluster;


        for _ in 1..entry_count {
            now_entry_offset_in_sector += 1;
            if now_entry_offset_in_sector >= (config::SECTOR_BYTES / 32) as u32 {
                now_entry_offset_in_sector = 0;
                now_sector_offset_in_cluster += 1;
                if now_sector_offset_in_cluster >= cluster_manager.sectors_per_cluster {
                    now_sector_offset_in_cluster = 0;
                    now_cluster_id = cluster_manager.get_next_cluster(&now_cluster_id).unwrap();
                }
            }

            cluster_manager.get_cluster_sector(&now_cluster_id, now_sector_offset_in_cluster).map(|block_cache| {
                block_cache.write().modify(0, |index_entries: &mut [[u8; 32]; config::SECTOR_BYTES / 32]| {
                    let entry = IndexEntry::from_bytes(&index_entries[now_entry_offset_in_sector as usize]).unwrap();
                    // 分析其类型
                    if entry.entry_type.bits() == 0xC0 || entry.entry_type.bits() == 0xC1 {
                        // 属性项2/3 目录项
                        index_entries[now_entry_offset_in_sector as usize][0] = entry.entry_type.bits() & 0x7F; // 标记为未使用
                    } else {
                        unreachable!("Unknown entry type")
                    }
                });
            });
        }
    }

    /// 修改文件数据（不包括文件名）
    pub fn modify_file(&mut self, file_meta_data: &FileMetaData) {
        // 过程：找到目标文件的目录项位置，覆写85H C0H目录项
        
        // 在指定簇链中查找文件是否已存在
        let entry_pos = self.get_file_by_pos(&file_meta_data.index_position.0, file_meta_data.index_position.1, file_meta_data.index_position.2);
        if entry_pos.is_none() {
            // 文件不存在
            return;
        }

        let cluster_manager = self.cluster_manager.read();

        let mut now_cluster_id = file_meta_data.index_position.0.clone();
        let mut now_sector_offset_in_cluster = file_meta_data.index_position.1;
        let mut now_entry_offset_in_sector = file_meta_data.index_position.2;

        let entries = file_meta_data.to_entries();

        // 覆写85H目录项
        cluster_manager.get_cluster_sector(&now_cluster_id, now_sector_offset_in_cluster).map(|block_cache| {
            block_cache.write().modify(now_entry_offset_in_sector as usize * 32, |index_entry: &mut [u8; 32]| {
                index_entry.copy_from_slice(&entries[0].to_bytes());
            });
        });
        // 移动指针
        now_entry_offset_in_sector += 1;
        if now_entry_offset_in_sector >= (config::SECTOR_BYTES / 32) as u32 {
            now_entry_offset_in_sector = 0;
            now_sector_offset_in_cluster += 1;
            if now_sector_offset_in_cluster >= cluster_manager.sectors_per_cluster {
                now_sector_offset_in_cluster = 0;
                now_cluster_id = cluster_manager.get_next_cluster(&now_cluster_id).unwrap();
            }
        }
        // 覆写C0H目录项
        cluster_manager.get_cluster_sector(&now_cluster_id, now_sector_offset_in_cluster).map(|block_cache| {
            block_cache.write().modify(now_entry_offset_in_sector as usize * 32, |index_entry: &mut [u8; 32]| {
                index_entry.copy_from_slice(&entries[1].to_bytes());
            });
        });
    }

    /// 列出簇链中所有目录项的文件
    pub fn list_files(&self, cluster_id: &ClusterId) -> Vec<FileMetaData> {
        let cluster_manager = self.cluster_manager.read();
        let mut result = Vec::new();

        // 暂存当前簇号
        let mut now_cluster_id = cluster_id.clone();
        // 暂存当前簇内扇区偏移量
        let mut now_sector_offset_in_cluster = 0;

        let mut end_procedure = false;

        // 遍历簇链中每个扇区
        loop {
            let block_cache = cluster_manager.get_cluster_sector(&now_cluster_id, now_sector_offset_in_cluster);
            if block_cache.is_none() {
                break;
            }
            let block_cache = block_cache.unwrap();

            // 遍历扇区中每个目录项
            block_cache.read().read(0, |index_entries: &[[u8; 32]; config::SECTOR_BYTES / 32]| {
                for (i, entry_bytes) in index_entries.iter().enumerate() {
                    let entry = IndexEntry::from_bytes(entry_bytes).unwrap();
                    // 分析其类型
                    match entry.entry_type.bits() {
                        0x00 => {
                            // 空的目录项，表示后面没有目录项了
                            end_procedure = true;
                            break;
                        }
                        0x85 => {
                            // 属性项1 目录项
                            let file = self.get_file_by_pos(&now_cluster_id, now_sector_offset_in_cluster, i as u32);
                            if file.is_some() {
                                result.push(file.unwrap());
                            }
                        }
                        _ => {
                            continue;
                        }
                    }
                }
            });
            
            if end_procedure {
                break;
            }

            // 更新扇区偏移和簇号
            now_sector_offset_in_cluster += 1;
            if now_sector_offset_in_cluster >= cluster_manager.sectors_per_cluster {
                // 读取下一个簇
                let next_cluster_id = cluster_manager.get_next_cluster(&now_cluster_id);
                if next_cluster_id.is_none() {
                    break;
                }
                now_cluster_id = next_cluster_id.unwrap();
                now_sector_offset_in_cluster = 0;
            }
        }

        result
    }

    /// 重新安排簇链中的目录项
    pub fn rearrange(&self, cluster_id: &ClusterId) {
        // TODO: 重新安排簇链中的目录项，删除空目录项，并移动合并目录项
    }
}