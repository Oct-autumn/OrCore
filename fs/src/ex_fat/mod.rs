use crate::block_device::block_cache::BlockCacheManager;
use crate::config;
use alloc::collections::VecDeque;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use model::up_case_table::UpCaseTable;
use cluster_chain::ClusterManager;
use core::usize;
use bitflags::Flags;
use model::cluster_id::ClusterId;
use model::index_entry::IndexEntry;
use spin::{Mutex, RwLock};
use core::cmp::min;
use std::ptr::{addr_of, read_unaligned};
use crate::block_device::BlockDevice;
use model::up_case_table::FileNameHash;
use crate::ex_fat::model::index_entry::{Attributes, BitmapCustom, EntryCustom, FileCustom, FragmentFlag, GenericCustom, IndexEntryChecksum, IndexEntryType, NameCustom, StreamCustom, TimeStamp, UpCaseCustom};
pub use model::unicode_str::UnicodeString;
use crate::ex_fat::boot_sector::{BootChecksum, BootSector};
use crate::ex_fat::file_manage::FileManager;
use crate::ex_fat::index_entry_manage::IndexEntryManager;

mod index_entry_manage;
mod file_manage;
mod cluster_chain;
mod r#const;
pub mod model;
pub mod boot_sector;

/// 文件元数据
#[derive(Debug, Clone)]
pub struct FileDirMetadata {
    pub attributes: Attributes,
    pub create_time_unix_ms_stamp: u64,
    pub last_modified_time_unix_ms_stamp: u64,
    pub last_accessed_time_unix_stamp: u64,
    pub is_fragment: bool,
    pub name: UnicodeString,
    pub name_hash: FileNameHash,
    pub first_cluster: ClusterId,
    pub size: usize,
}

impl FileDirMetadata {
    pub fn from_entry_set(entry_set: &[IndexEntry]) -> Option<Self> {
        assert!(entry_set.len() >= 2, "Entry set length must be at least 2");

        let mut name = UnicodeString::new();

        let record_checksum = unsafe { entry_set[0].custom_defined.file.check_sum };
        let mut index_entry_checksum = IndexEntryChecksum(0);

        // 处理第一个目录项
        assert_eq!(entry_set[0].entry_type, IndexEntryType::ExfatFile, "First entry must be File Entry");
        let file_custom_raw = unsafe { entry_set[0].custom_defined.file };

        let attributes = file_custom_raw.file_attributes;

        let create_time_unix_ms_stamp =
            unsafe { read_unaligned(addr_of!(file_custom_raw.create_time_stamp)) }.to_unix_timestamp() * 1000
                + file_custom_raw.create_10ms_increment as u64 * 10;
        let last_modified_time_unix_ms_stamp =
            unsafe { read_unaligned(addr_of!(file_custom_raw.last_modified_time_stamp)) }.to_unix_timestamp() * 1000
                + file_custom_raw.modify_10ms_increment as u64 * 10;
        let last_accessed_time_unix_stamp =
            unsafe { read_unaligned(addr_of!(file_custom_raw.last_accessed_time_stamp)) }.to_unix_timestamp();
        // 计算校验和
        index_entry_checksum.add_entry(entry_set[0].to_bytes().as_slice(), true);

        // 处理第二个目录项
        assert_eq!(entry_set[1].entry_type, IndexEntryType::ExfatStream, "Second entry must be Stream Entry");
        let stream_custom_raw = unsafe { entry_set[1].custom_defined.stream };

        let is_fragment = match stream_custom_raw.fragment_flag {
            FragmentFlag::Fragmented => true,
            FragmentFlag::Continuous => false,
            _ => panic!("Invalid FragmentFlag"),
        };

        let name_len = stream_custom_raw.file_name_length;
        let name_hash = stream_custom_raw.file_name_hash;
        let first_cluster = stream_custom_raw.start_cluster;
        let size = stream_custom_raw.file_size1 as usize;
        // 计算校验和
        index_entry_checksum.add_entry(entry_set[1].to_bytes().as_slice(), false);

        // 由剩余目录项获取文件名
        for i in 2..entry_set.len() {
            assert_eq!(entry_set[i].entry_type, IndexEntryType::ExfatName, "Entry must be Name Entry");
            let name_bytes = unsafe { entry_set[i].custom_defined.name.name };
            let len = min(name_bytes.len(), name_len as usize); // 读入的文件名长度
            for i in 0..len {
                name.push(name_bytes[i]);
            }
            // 计算校验和
            index_entry_checksum.add_entry(entry_set[i].to_bytes().as_slice(), false);
        }

        // 核对校验和
        if record_checksum != index_entry_checksum {
            panic!("Checksum mismatch");
        }

        Some(FileDirMetadata {
            attributes,
            create_time_unix_ms_stamp,
            last_modified_time_unix_ms_stamp,
            last_accessed_time_unix_stamp,
            is_fragment,
            name,
            name_hash,
            first_cluster,
            size,
        })
    }

    pub fn to_entry_set(&self) -> Vec<IndexEntry> {
        let mut result = Vec::new();

        // 生成目录项
        { // 生成第一个目录项
            let secondary_count = (((self.name.len() + 14) / 15) + 1) as u8;
            let (create_time_stamp, create_10ms_increment) = TimeStamp::from_unix_ms_timestamp(self.create_time_unix_ms_stamp);
            let (last_modified_time_stamp, modify_10ms_increment) = TimeStamp::from_unix_ms_timestamp(self.last_modified_time_unix_ms_stamp);
            let last_accessed_time_stamp = TimeStamp::from_unix_timestamp(self.last_accessed_time_unix_stamp);

            result.push(IndexEntry {
                entry_type: IndexEntryType::ExfatFile,
                custom_defined: EntryCustom {
                    file: FileCustom::new(
                        secondary_count,
                        IndexEntryChecksum(0),  // 稍后更新
                        self.attributes,
                        create_time_stamp,
                        last_modified_time_stamp,
                        last_accessed_time_stamp,
                        create_10ms_increment,
                        modify_10ms_increment,
                        0,  // 不使用
                        0,  // 不使用
                        0,  // 不使用
                    )
                },
            });
        }

        { // 生成第二个目录项
            let fragment_flag = if self.is_fragment { FragmentFlag::Fragmented } else { FragmentFlag::Continuous };

            result.push(IndexEntry {
                entry_type: IndexEntryType::ExfatStream,
                custom_defined: EntryCustom {
                    stream: StreamCustom::new(
                        fragment_flag,
                        self.name.len() as u8,
                        self.name_hash,
                        self.size as u64,
                        self.first_cluster,
                        self.size as u64,
                    )
                },
            });
        }

        { // 生成文件名目录项
            let name_len = self.name.len();
            let mut index = 0;
            while index < name_len {
                let name_slice = &self.name.data[index..index + min(name_len - index, 15)];
                let mut name_arr = [0u16; 15];
                for (wbyte_src, wbyte_dst) in name_slice.iter().zip(name_arr.iter_mut()) {
                    *wbyte_dst = *wbyte_src;
                }

                let custom = EntryCustom { name: NameCustom::new(name_arr) };

                result.push(IndexEntry {
                    entry_type: IndexEntryType::ExfatName,
                    custom_defined: custom,
                });
                index += min(name_len - index, 15);
            }
        }

        { // 计算校验和
            let mut checksum = IndexEntryChecksum(0);
            for entry in result.iter() {
                checksum.add_entry(entry.to_bytes().as_slice(), entry.entry_type == IndexEntryType::ExfatFile);
            }
            unsafe { result[0].custom_defined.file.check_sum = checksum; }
        }

        result
    }

    pub fn is_directory(&self) -> bool {
        self.attributes.contains(Attributes::Directory)
    }

    pub fn is_read_only(&self) -> bool { self.attributes.contains(Attributes::ReadOnly) }
}

pub enum MetadataType {
    FileOrDir(FileDirMetadata),
    Root,
}

pub struct ExFAT {
    /// 卷标
    volume_label: UnicodeString,
    /// 引导扇区
    boot_sector: BootSector,
    /// 存储设备接口指针
    device: Arc<dyn BlockDevice>,
    /// 簇管理器
    cluster_manager: Arc<RwLock<ClusterManager>>,
    /// 大写字母表
    up_case_table: UpCaseTable,
    /// 块缓存管理器
    block_cache_manager: Arc<Mutex<BlockCacheManager>>,
    /// 目录项管理器
    index_entry_manager: IndexEntryManager,
    /// 文件访问管理器
    file_manager: FileManager,
}

impl ExFAT {
    /// 从设备创建ExFAT文件系统
    pub fn from_device(device: Arc<dyn BlockDevice>) -> Option<ExFAT> {
        // 流程：
        // 0. 创建块缓存管理器
        // 1. 读取引导扇区
        // 2. 创建簇管理器
        // 3. 创建大写字母表
        // 4. 创建目录管理器、文件管理器
        // 5. 读取卷标

        // 0. 创建块缓存管理器
        let block_cache_manager = Arc::new(Mutex::new(BlockCacheManager::new(device.clone())));

        // 1. 读取引导扇区
        // <---- BCM独占区 开始 ---->
        let mut bcm_guard = block_cache_manager.lock();

        // 1.1 读取引导扇区
        // 从0号扇区开始，查找引导扇区
        let mut boot_sector_id = <usize>::MAX;
        for i in 0..device.num_blocks() {
            if bcm_guard
                .get_block_cache(i)
                .read()
                .read(0, |data: &[u8; config::SECTOR_BYTES]| {
                    data[0..3] == r#const::EXFAT_BOOT_JUMP
                        && data[3..11] == r#const::EXFAT_SIGNATURE
                        && data[config::SECTOR_BYTES - 2..] == r#const::EXFAT_BOOT_SIGNATURE
                }) {
                boot_sector_id = i;
                break;
            }
        }
        if boot_sector_id == <usize>::MAX {
            panic!("No exFAT boot sector found.");
        }

        // 读取引导扇区
        let boot_sector = bcm_guard.get_block_cache(boot_sector_id).read().read(0, |data: &BootSector| {
            data.clone()
        });
        if let Err(e) = boot_sector.check_valid() {
            panic!("Invalid BootSector: {}", e)
        }

        // 1.2 引导区校验
        let mut checksum = BootChecksum(0);
        // 读取0～10号区并计算引导区校验和
        for i in 0..=10 {
            bcm_guard.get_block_cache(i).read().read(0, |data: &[u8; config::SECTOR_BYTES]| {
                if i != 0 && i <= 8 {
                    // 扩展引导扇区，检查引导标记
                    if data[config::SECTOR_BYTES - 4..] != r#const::EXFAT_EXBOOT_SIGNATURE {
                        panic!("Invalid ExBootSector signature.");
                    }
                }

                checksum.add_sector(data, i == 0);
            });
        }
        // 从11号扇区读取校验和，进行比较
        if !bcm_guard.get_block_cache(11).read().read(0, |data: &[u32; 128]| {
            for i in 0..128 {
                if data[i] != checksum.0 {
                    return false;
                }
            }
            true
        }) {
            panic!("Invalid BootChecksum.");
        }

        drop(bcm_guard);
        // <---- BCM独占区 结束 ---->

        // 2. 创建簇管理器
        let cluster_manager =
            Arc::new(RwLock::new(ClusterManager::new(&boot_sector, block_cache_manager.clone())));
        // 因为是从设备中读取的引导扇区，所以不需要再次初始化

        // 3. 创建大写字母表
        let up_case_table = UpCaseTable::create(&boot_sector, &cluster_manager);
        // 因为是从设备中读取的引导扇区，所以不需要再次初始化

        // 4. 读取卷标
        // 卷标应该保存在根目录的第一个目录项中
        let volume_label = cluster_manager.read().get_cluster_sector(&ClusterId(boot_sector.first_cluster_of_root_directory), 0).unwrap()
            .read().read(0, |entries: &[IndexEntry; config::SECTOR_BYTES >> 5]| {
            let volume_name_entry = &entries[0];

            return match volume_name_entry.entry_type {
                IndexEntryType::ExfatVolume => {
                    // 有卷标
                    let custom = unsafe { volume_name_entry.custom_defined.name };
                    let mut volume_label = UnicodeString::new();
                    for i in 0..custom.flags {
                        volume_label.push(custom.name[i as usize]);
                    }
                    Some(volume_label)
                }
                _ => {
                    // 无卷标
                    None
                }
            };
        });

        let index_entry_manage = IndexEntryManager::new(&boot_sector, up_case_table, cluster_manager.clone());
        let file_manager = FileManager::new(&boot_sector, cluster_manager.clone());

        Some(ExFAT {
            volume_label: volume_label.unwrap_or(UnicodeString::new()),
            boot_sector,
            device: device.clone(),
            cluster_manager,
            up_case_table,
            block_cache_manager,
            index_entry_manager: index_entry_manage,
            file_manager,
        })
    }

    /// 内部函数：根据目录序列查找目标
    ///
    /// 返回值：(上层目录，目标)
    fn find_by_dir_list(&self, path: &Vec<UnicodeString>) -> Option<(Option<MetadataType>, MetadataType)> {
        let mut path: VecDeque<UnicodeString> = path.clone().into();

        if path.is_empty() {
            return Some((None, MetadataType::Root));
        }

        // 从根目录开始查找
        let mut current_target_name = path.pop_front().unwrap();
        let mut now_metadata = MetadataType::Root;

        loop {
            // 从当前目录中查找目标
            let res = self.index_entry_manager.find_metadata_by_name(&now_metadata, &current_target_name);

            if let Some(metadata) = res {
                // 如果找到了目标，查看path是否为空
                if path.is_empty() {
                    // 如果path为空，说明找到了目标
                    return Some((Some(now_metadata), metadata));
                } else {
                    // 如果path不为空，说明还需要继续查找
                    // 更新current_cluster_id和current_target_name
                    current_target_name = path.pop_front().unwrap();
                    now_metadata = metadata;
                    continue;
                }
            } else {
                // 在某次查找中丢失目标，查找失败
                return None;
            }
        }
    }

    /// 内部函数：将路径字符串转换为目录列表
    fn path_to_dir_list(path: &String) -> Vec<UnicodeString> {
        let path_list: Vec<&str> = path.split('/').collect();
        let mut ret = Vec::new();

        for p in path_list {
            if p.is_empty() {
                continue;
            }
            ret.push(UnicodeString::from_str(p));
        }

        ret
    }

    /// 查找目标
    ///
    /// 返回值：(上层目录，目标)
    pub fn find(&self, path: &String) -> Option<(Option<MetadataType>, MetadataType)> {
        // 将Path分割为目录层级
        let path = Self::path_to_dir_list(path);

        self.find_by_dir_list(&path)
    }

    /// 列出指定路径下的文件
    pub fn list(&self, path: &String) -> Option<Vec<FileDirMetadata>> {
        // 流程：
        // 1. 查找目标文件元数据
        // 2. 检查是否为文件夹，如果不是，返回空
        // 3. 调用目录管理器列出目录项

        // 1. 查找目标文件元数据
        let Some((_, target)) = self.find(path) else {
            panic!("No such file or directory");
        };

        match target {
            MetadataType::FileOrDir(ref metadata) => {
                // 2. 检查是否为文件夹，如果不是，返回空
                if !metadata.attributes.contains(Attributes::Directory) {
                    // 如果是文件，则返回空
                    return None;
                }
            }
            MetadataType::Root => { /* do nothing*/ }
        }
        // 3. 调用目录管理器列出目录项
        Some(self.index_entry_manager.list_metadata(&target))
    }

    /// **功能：** 创建文件
    ///
    /// **参数：**
    ///         - `path`：文件路径
    ///         - `file_meta_data`：文件元数据
    pub fn touch(&mut self, path: String, file_attributes: Attributes, unix_ms_timestamp: u64) -> Option<(MetadataType, FileDirMetadata)> {
        // 流程：
        // 1. 创建文件元数据
        // 2. 查找上层目录
        // 4. 在上层目录中创建文件
        // 5. 返回文件元数据

        // 1. 查找上层目录
        // 将path分割为目录和文件名
        let mut path = Self::path_to_dir_list(&path);
        let name = path.pop().unwrap();

        // 1. 创建文件元数据
        if unix_ms_timestamp < r#const::EXFAT_MIN_TIMESTAMP_MSECS || unix_ms_timestamp > r#const::EXFAT_MAX_TIMESTAMP_MSECS {
            panic!("Invalid timestamp");
        }

        let mut wrapped_file_metadata = MetadataType::FileOrDir(FileDirMetadata {
            attributes: file_attributes,
            create_time_unix_ms_stamp: unix_ms_timestamp,
            last_modified_time_unix_ms_stamp: unix_ms_timestamp,
            last_accessed_time_unix_stamp: unix_ms_timestamp / 1000,
            is_fragment: false,
            name,
            name_hash: FileNameHash(0),
            size: 0,
            first_cluster: ClusterId::eof(),           // 未分配簇
        });

        // 2. 查找上层目录
        if path.is_empty() {
            // 在根目录下创建文件
            let mut root_dir_metadata = MetadataType::Root;
            self.index_entry_manager.create_entries(&mut root_dir_metadata, &mut wrapped_file_metadata)?;
            let MetadataType::FileOrDir(file_meta_data) = wrapped_file_metadata else { unreachable!() };
            Some((MetadataType::Root, file_meta_data))
        } else {
            // 在非根目录下创建文件

            // 查找上层目录和上上层目录
            let Some((Some(wrapped_suparent_dir_metadata), mut wrapped_parent_dir_metadata)) = self.find_by_dir_list(&path) else {
                panic!("No such file or directory")
            };

            // 在上层目录中创建文件
            self.index_entry_manager.create_entries(&mut wrapped_parent_dir_metadata, &mut wrapped_file_metadata)?;

            // 更新时间戳
            let MetadataType::FileOrDir(mut parent_dir_metadata) = wrapped_parent_dir_metadata else { unreachable!() };
            parent_dir_metadata.last_modified_time_unix_ms_stamp = unix_ms_timestamp;
            parent_dir_metadata.last_accessed_time_unix_stamp = unix_ms_timestamp / 1000;
            let wrapped_parent_dir_metadata = MetadataType::FileOrDir(parent_dir_metadata);

            // 更新上层目录的目录项
            self.index_entry_manager.modify_entries(&wrapped_suparent_dir_metadata, &wrapped_parent_dir_metadata)?;

            let MetadataType::FileOrDir(file_meta_data) = wrapped_file_metadata else { unreachable!() };
            Some((wrapped_parent_dir_metadata, file_meta_data))
        }
    }

    /// 删除文件/文件夹
    pub fn delete(&mut self, path: &String) -> Option<()> {
        // 流程：
        // 1. 查找文件与上层目录
        // 2. 释放文件占用的簇
        // 3. 在上层目录中删除文件目录项

        // 1. 查找文件
        let Some((Some(wrapped_parent_dir_metadata), wrapped_file_metadata)) = self.find(path) else {
            panic!("No such file or directory")
        };
        let MetadataType::FileOrDir(mut file_metadata) = wrapped_file_metadata else {
            unreachable!()
        };

        // 2. 释放文件占用的簇
        if file_metadata.attributes.contains(Attributes::Directory) {
            // 如果是文件夹，检查是否为空
            // TODO: 考虑调用rearrange方法
            if file_metadata.size != 0 {
                panic!("Directory is not empty");
            }
        } else {
            // 如果是文件，释放文件占用的簇
            self.file_manager.clear_file(&mut file_metadata);
        }

        // 3. 在上层目录中删除文件目录项
        self.index_entry_manager.delete_entries(&wrapped_parent_dir_metadata, &file_metadata.name);

        Some(())
    }

    /// 移动文件/文件夹（也用于重命名）
    pub fn move_to(&mut self, old_path: &String, new_path: &String, unix_ms_timestamp: u64) -> Option<(MetadataType, FileDirMetadata)> {
        let Some((Some(wrapped_parent_dir_metadata), wrapped_file_metadata)) = self.find(old_path) else {
            panic!("No such file or directory")
        };
        let MetadataType::FileOrDir(mut file_metadata) = wrapped_file_metadata else {
            unreachable!()
        };
        // 从原目录中删除
        self.index_entry_manager.delete_entries(&wrapped_parent_dir_metadata, &file_metadata.name);

        let mut path = Self::path_to_dir_list(new_path);
        let name = path.pop().unwrap();

        // 更新文件名
        file_metadata.name = name;
        // 更新时间戳
        if unix_ms_timestamp < r#const::EXFAT_MIN_TIMESTAMP_MSECS || unix_ms_timestamp > r#const::EXFAT_MAX_TIMESTAMP_MSECS {
            panic!("Invalid timestamp");
        }
        file_metadata.last_modified_time_unix_ms_stamp = unix_ms_timestamp;
        file_metadata.last_accessed_time_unix_stamp = unix_ms_timestamp / 1000;

        // 重新套壳
        let mut wrapped_file_metadata = MetadataType::FileOrDir(file_metadata);

        if path.is_empty() {
            // 在根目录下创建文件
            let mut root_dir_metadata = MetadataType::Root;
            self.index_entry_manager.create_entries(&mut root_dir_metadata, &mut wrapped_file_metadata)?;
            let MetadataType::FileOrDir(file_meta_data) = wrapped_file_metadata else { unreachable!() };
            Some((MetadataType::Root, file_meta_data))
        } else {
            // 在非根目录下创建文件

            // 查找上层目录和上上层目录
            let Some((Some(wrapped_suparent_dir_metadata), mut wrapped_parent_dir_metadata)) = self.find_by_dir_list(&path) else {
                panic!("No such file or directory")
            };

            // 在上层目录中创建文件
            self.index_entry_manager.create_entries(&mut wrapped_parent_dir_metadata, &mut wrapped_file_metadata)?;

            // 更新上层目录的目录项
            self.index_entry_manager.modify_entries(&wrapped_suparent_dir_metadata, &wrapped_parent_dir_metadata)?;

            let MetadataType::FileOrDir(file_meta_data) = wrapped_file_metadata else { unreachable!() };
            Some((wrapped_parent_dir_metadata, file_meta_data))
        }
    }

    /// 清空文件
    pub fn clear(&mut self, path: &String, unix_ms_timestamp: u64) -> Option<()> {
        // 流程：
        // 1. 查找文件
        // 2. 释放文件占用的簇
        // 3. 将文件大小置为0
        // 4. 更新文件元数据

        // 1. 查找文件
        let Some((Some(wrapped_parent_dir_metadata), wrapped_file_metadata)) = self.find(path) else {
            panic!("No such file or directory")
        };
        let MetadataType::FileOrDir(mut file_metadata) = wrapped_file_metadata else {
            unreachable!()
        };

        // 2. 释放文件占用的簇（file meta data中的first cluster和file size会被重置）
        if file_metadata.attributes.contains(Attributes::Directory) {
            panic!("Not a file");
        }
        self.file_manager.clear_file(&mut file_metadata);

        // 更新时间戳
        if unix_ms_timestamp < r#const::EXFAT_MIN_TIMESTAMP_MSECS || unix_ms_timestamp > r#const::EXFAT_MAX_TIMESTAMP_MSECS {
            panic!("Invalid timestamp");
        }
        file_metadata.last_modified_time_unix_ms_stamp = unix_ms_timestamp;
        file_metadata.last_accessed_time_unix_stamp = unix_ms_timestamp / 1000;

        // 4. 更新文件元数据
        self.index_entry_manager.modify_entries(&wrapped_parent_dir_metadata, &MetadataType::FileOrDir(file_metadata));

        Some(())
    }

    /// 读取文件
    pub fn read(&self, file_metadata: &FileDirMetadata, offset: usize, buf: &mut [u8]) -> Option<usize> {
        if file_metadata.attributes.contains(Attributes::Directory) {
            panic!("Not a file");
        }

        Some(self.file_manager.read_at(&file_metadata, offset, buf))
    }

    /// 写入文件
    pub fn write(&mut self, mut file_metadata: &mut FileDirMetadata, offset: usize, buf: &[u8]) -> Option<usize> {
        if file_metadata.attributes.contains(Attributes::Directory) {
            panic!("Not a file");
        }

        Some(self.file_manager.write_at(&mut file_metadata, offset, buf))
    }

    /// 当文件元数据发生变化时，更新文件元数据
    pub fn update_file_metadata(&mut self, wrapped_parent_dir_metadata: &MetadataType, file_metadata: FileDirMetadata) -> Option<()> {
        if file_metadata.attributes.contains(Attributes::Directory) {
            panic!("Not a file");
        }

        self.index_entry_manager.modify_entries(wrapped_parent_dir_metadata, &MetadataType::FileOrDir(file_metadata))
    }
}

impl Drop for ExFAT {
    fn drop(&mut self) {
        // 退出时将所有数据写回设备
        self.block_cache_manager.lock().sync_all();
    }
}