use crate::block_cache::{get_block_cache, sync_all};
use crate::ex_fat::file_manage::FileManager;
use crate::ex_fat::index_entry_manage::IndexEntryManager;
use crate::ex_fat::persistent_layer::model::index_entry::EntryCostume;
use crate::{config, BlockDevice};
use alloc::collections::VecDeque;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use persistent_layer::boot::{BootChecksum, BootSector};
use persistent_layer::model::cluster_id::ClusterId;
use persistent_layer::model::index_entry::IndexEntry;
use persistent_layer::up_case_table::UpCaseTable;
use persistent_layer::ClusterManager;
use spin::RwLock;

use crate::ex_fat::persistent_layer::up_case_table::FileNameHash;
pub use index_entry_manage::FileMetaData;
pub use persistent_layer::model::index_entry::file_directory::FileAttributes;
pub use persistent_layer::model::unicode_str::UnicodeString;

mod index_entry_manage;
mod file_manage;
mod persistent_layer;

pub struct ExFAT {
    /// 卷标
    volume_label: UnicodeString,
    /// 引导扇区
    boot_sector: BootSector,
    /// 存储设备接口指针
    device: Arc<dyn BlockDevice>,
    /// 簇管理器
    cluster_manager: Arc<RwLock<ClusterManager>>,
    /// 目录管理器
    index_entry_manager: IndexEntryManager,
    /// 文件管理器
    file_manager: FileManager,
}


// TODO: 构造函数（from_device、create）加入对隐藏扇区的处理
impl ExFAT {
    /// 从引导扇区创建ExFAT文件系统
    pub fn from_device(device: Arc<dyn BlockDevice>) -> Option<ExFAT> {
        // 流程：
        // 1. 读取引导扇区
        // 2. 创建簇管理器
        // 3. 创建大写字母表
        // 4. 创建目录管理器、文件管理器
        // 5. 读取卷标

        // 1. 读取引导扇区
        // 从0号扇区开始，查找引导扇区并读取、计算引导扇区校验和
        
        let mut boot_sector_id = u64::MAX;
        for i in 0..device.num_blocks() {
            get_block_cache(0, &device).read().read(0, |data: &[u8; config::SECTOR_BYTES]| {
                if data[0..3] == config::EXFAT_BOOT_JUMP && data[3..11] == config::EXFAT_SIGNATURE {
                    boot_sector_id = i;
                }
            });   
        }
        
        
        let mut checksum = BootChecksum(0);

        let boot_sector = get_block_cache(0, &device).read().read(0, |data: &[u8; config::SECTOR_BYTES]| {
            checksum.add_sector(data, true);
            BootSector::from_bytes(data.clone())
        });

        if boot_sector.jump_boot != config::EXFAT_BOOT_JUMP
            || boot_sector.file_system_name != config::EXFAT_SIGNATURE {
            return None;
        }

        // 读取1～10号扇区并计算引导扇区校验和
        for i in 1..=10 {
            get_block_cache(i, &device).read().read(0, |data: &[u8; config::SECTOR_BYTES]| {
                checksum.add_sector(data, false);
            });
        }
        // 从11号扇区读取校验和，进行比较
        if !get_block_cache(11, &device).read().read(0, |data: &[u32; 128]| {
            data[0] == checksum.0
        }) {
            return None;
        }

        // 2. 创建簇管理器
        let cluster_manager = Arc::new(RwLock::new(ClusterManager::new(&boot_sector, &device)));
        // 因为是从设备中读取的引导扇区，所以不需要再次初始化

        // 3. 创建大写字母表
        let up_case_table = UpCaseTable::generate_up_case_table();
        // 因为是从设备中读取的引导扇区，所以不需要再次初始化

        // 4. 创建目录管理器、文件管理器
        let index_entry_manager = IndexEntryManager::new(up_case_table, cluster_manager.clone(), device.clone());
        let file_manager = FileManager::new(cluster_manager.clone(), device.clone());

        // 5. 读取卷标
        // 卷标应该保存在根目录的第一个目录项中
        let volume_label = cluster_manager.read().get_cluster_sector(&ClusterId(boot_sector.first_cluster_of_root_directory), 0).unwrap()
            .read().read(0, |index_entry_bytes: &[u8; 32]| {
            let index_entry = IndexEntry::from_bytes(index_entry_bytes).unwrap();
            match index_entry.entry_type.bits() {
                0x83 => {
                    // 有卷标
                    let EntryCostume::VolumeLabel(custom) = &index_entry.custom_defined else {
                        unreachable!("Invalid index_entry.");
                    };
                    let volume_label = custom.volume_label.clone();
                    if volume_label.len() != custom.volume_label_length as usize {
                        panic!("Volume label length is not equal to the length in the index entry.");
                    }
                    return Some(volume_label);
                }
                0x03 => {
                    // 无卷标
                    return None;
                }
                _ => {
                    // 异常
                    unreachable!("Invalid index_entry.");
                }
            }
        });

        Some(ExFAT {
            volume_label: volume_label.unwrap_or(UnicodeString::new()),
            boot_sector,
            device: device.clone(),
            cluster_manager,
            index_entry_manager,
            file_manager,
        })
    }

    /// **功能：** 创建一个新的ExFAT文件系统
    ///
    /// **参数：**
    ///   - `bytes_per_sector`：每个扇区的字节数
    ///   - `sector_per_cluster`：每个簇的扇区数
    ///   - `volume_label`：卷标
    ///   - `device`：块设备
    pub fn create(bytes_per_sector: u32, sector_per_cluster: u32, volume_label: UnicodeString, device: Arc<dyn BlockDevice>) -> Self {
        // 流程：
        // 1. 计算各个区域的大小和位置
        // 2. 创建引导扇区
        // 3. 创建簇管理器
        // 4. 创建大写字母表
        // 5. 创建目录管理器、文件管理器
        // 6. 创建并写入根目录

        // MBR&BBR 24扇区，向上对齐到一个簇
        // FAT表 向上对齐到簇
        // 簇堆 从FAT表后开始

        // 卷扇区数
        let volume_length = device.num_blocks() as u32;

        // 卷按照指定的簇大小划分，向下取整，得到簇数
        let mut cluster_count = volume_length / sector_per_cluster;
        // MBR区和BBR区，向上对齐到一簇
        let boot_reserve = (12 * 2 + sector_per_cluster - 1) / sector_per_cluster;

        // FAT表的偏移量，即FAT表的起始扇区号，直接设置为簇扇区数
        let fat_offset = boot_reserve * sector_per_cluster;

        // FAT表长度，单位：簇，
        let mut fat_length = 1;

        // 每簇FAT表项数 = 每扇区字节数 * 每簇扇区数 / 4
        // 找到合适的FAT表长度，使得FAT表最小，又能容纳所有簇
        loop {
            let fat_table_size = fat_length * sector_per_cluster * bytes_per_sector / 4;
            if fat_table_size < (cluster_count - boot_reserve - fat_length) {
                fat_length += 1;
            } else {
                break;
            }
        }

        // 计算簇分配位图的长度，防止超过一簇
        let cluster_bitmap_bytes = ((cluster_count + 7) >> 3);
        if cluster_bitmap_bytes > sector_per_cluster * bytes_per_sector {
            panic!("Cluster bitmap is too large. Please enlarge the number of sector per cluster.");
        }

        // 簇堆中簇的数量
        cluster_count = cluster_count - boot_reserve - fat_length;
        // 首簇偏移量，即簇堆的起始扇区号，FAT表起始位置为簇的整数倍，所以我们将FAT表长度向上对齐到簇的整数倍，加上FAT表起始位置，得到首簇偏移量
        let cluster_heap_offset = fat_offset + fat_length;
        // FAT表的长度，每个簇对应一个FAT表项，每个FAT表项占用4个字节，故FAT表长度为：（簇数量*4字节）/ 每个扇区的字节数
        // （计算结果向上取整）
        let fat_length = (cluster_count * 4 + bytes_per_sector - 1) / bytes_per_sector;

        // 根目录的首簇号
        let first_cluster_of_root_directory = 4;

        // 卷序列号
        let volume_serial_number = 0x705A_5236;

        // 2. 创建引导扇区
        let boot_sector = BootSector::create(
            volume_length as u64,
            fat_offset,
            fat_length,
            cluster_heap_offset,
            cluster_count,
            first_cluster_of_root_directory,
            volume_serial_number,
            bytes_per_sector,
            sector_per_cluster,
        );
        let boot_sector_bytes = boot_sector.to_bytes();

        // 将MBR写入0号扇区
        get_block_cache(0, &device).write().modify_and_sync(0, |data: &mut [u8; config::SECTOR_BYTES]| {
            data.copy_from_slice(&boot_sector_bytes);
        });
        // 将1～10号扇区清零，并在每扇区最后两个字节写入boot_signature
        for i in 1..=10 {
            get_block_cache(i, &device).write().modify_and_sync(0, |data: &mut [u8; config::SECTOR_BYTES]| {
                data.fill(0);
                data[510] = config::EXFAT_BOOT_END[0];
                data[511] = config::EXFAT_BOOT_END[1];
            });
        }

        // 计算引导扇区校验和
        let mut checksum = BootChecksum(0);
        checksum.add_sector(boot_sector_bytes.as_slice(), true);
        for i in 1..=10 {
            let mut block = [0; config::SECTOR_BYTES];
            block[510] = config::EXFAT_BOOT_END[0];
            block[511] = config::EXFAT_BOOT_END[1];

            checksum.add_sector(&block, false);
        }
        // 将校验和写入11号扇区
        get_block_cache(11, &device).write().modify_and_sync(0, |data: &mut [u32; 128]| {
            // 校验和将在扇区中重复，直至扇区结束
            for i in 0..128 {
                data[i] = checksum.0;
            }
        });

        // 3. 创建簇管理器
        let cluster_manager = Arc::new(RwLock::new(ClusterManager::new(&boot_sector, &device)));
        cluster_manager.write().init();

        // 4. 创建并写入大写字母表
        let up_case_table = UpCaseTable::generate_up_case_table();
        up_case_table.save(&cluster_manager);
        let up_case_table_bytes = up_case_table.0.len();

        // 5. 创建目录管理器、文件管理器
        let index_entry_manager = IndexEntryManager::new(up_case_table, cluster_manager.clone(), device.clone());
        let file_manager = FileManager::new(cluster_manager.clone(), device.clone());

        // 6. 创建并写入根目录
        // 申请一个簇（簇号应为4）
        let cluster_id = cluster_manager.write().alloc_new_cluster().unwrap();
        assert_eq!(cluster_id.0, first_cluster_of_root_directory);   // 根目录的首簇号应为4
        // 在簇中创建卷标目录项、簇分配位图目录项、大写字母表目录项（应当可以在一个扇区中完成）
        cluster_manager.read().get_cluster_sector(&cluster_id, 0).unwrap().write().modify_and_sync(0, |entries: &mut [[u8; 32]; config::SECTOR_BYTES / 32]| {
            // 创建并写入卷标目录项
            let volume_label_entry = IndexEntry::new_volume_label(&volume_label);
            entries[0] = volume_label_entry.to_bytes();
            // 创建并写入簇分配位图目录项
            let cluster_bitmap_entry = IndexEntry::new_allocation_bitmap(ClusterId(2), cluster_bitmap_bytes as u64);
            entries[1] = cluster_bitmap_entry.to_bytes();
            // 创建并写入大写字母表目录项
            let up_case_table_entry = IndexEntry::new_up_case_table(ClusterId(3), up_case_table_bytes as u64);
            entries[2] = up_case_table_entry.to_bytes();
            // 写入一个空目录项，表示根目录结束
            entries[3] = IndexEntry::new_empty().to_bytes();
        });

        ExFAT {
            volume_label,
            boot_sector,
            device: device.clone(),
            cluster_manager,
            index_entry_manager,
            file_manager,
        }
    }

    /// 列出指定路径下的文件
    pub fn list(&self, path: &String) -> Option<Vec<FileMetaData>> {
        // 流程：
        // 1. 查找目标文件元数据
        // 2. 检查是否为文件夹，如果不是，返回空
        // 3. 调用目录管理器列出目录项

        // 1. 查找目标文件元数据
        let Some(target) = self.find(path) else {
            return None;
        };

        // 2. 检查是否为文件夹
        if !target.is_directory() {
            return None;
        }

        // 3. 检查是否已分配簇
        if target.first_cluster.is_none() {
            return None;
        }

        // 3. 调用目录管理器列出目录项
        Some(self.index_entry_manager.list_files(&target.first_cluster.unwrap()))
    }

    /// **功能：** 创建文件
    ///
    /// **参数：**
    ///         - `path`：文件路径
    ///         - `file_meta_data`：文件元数据
    pub fn touch(&mut self, path: String, file_attributes: FileAttributes, unix_ms_timestamp: u64) -> Option<FileMetaData> {
        // 流程：
        // 1. 创建文件元数据
        // 2. 查找上层目录
        // 4. 在上层目录中创建文件
        // 5. 返回文件元数据

        // 1. 查找上层目录
        // 将path分割为目录和文件名
        let mut path = Self::path_to_dir_list(&path);
        let file_name = path.pop().unwrap();

        // 1. 创建文件元数据

        let mut file_meta_data = FileMetaData {
            file_attributes,
            create_time_unix_ms_stamp: unix_ms_timestamp,
            last_modified_time_unix_stamp: unix_ms_timestamp / 1000,
            last_accessed_time_unix_stamp: unix_ms_timestamp / 1000,
            is_fragment: false,
            file_name,
            file_name_hash: FileNameHash(0),
            file_size: 0,
            first_cluster: None,           // 未分配簇
            index_position: (ClusterId::free(), 0, 0),  // 未分配目录项
        };

        // 查找上层目录
        if let Some(mut dir_meta_data) = self.find_by_dir_list(&path) {
            // 2. 在上层目录中创建文件
            if let Some(dir_cluster_id) = dir_meta_data.first_cluster {
                // 上层目录已分配簇，可以创建文件
                file_meta_data.index_position = self.index_entry_manager.create_file(&mut file_meta_data, &dir_cluster_id)?;
                Some(file_meta_data)
            } else {
                // 上层目录未分配簇，先分配簇再创建文件
                let new_cluster_id = self.cluster_manager.write().alloc_new_cluster().unwrap();
                dir_meta_data.first_cluster = Some(new_cluster_id);
                self.index_entry_manager.modify_file(&dir_meta_data);
                file_meta_data.index_position = self.index_entry_manager.create_file(&mut file_meta_data, &new_cluster_id)?;
                Some(file_meta_data)
            }
        } else {
            // 如果上层目录不存在，返回None
            None
        }
    }

    /// 查找目标
    pub fn find(&self, path: &String) -> Option<FileMetaData> {
        // 将Path分割为目录层级
        let path = Self::path_to_dir_list(path);

        self.find_by_dir_list(&path)
    }

    /// 清空文件
    pub fn clear(&mut self, path: &String) -> Option<()> {
        // 流程：
        // 1. 查找文件
        // 2. 释放文件占用的簇
        // 3. 将文件大小置为0
        // 4. 更新文件元数据

        // 1. 查找文件
        let Some(mut file_meta_data) = self.find(path) else {
            return None;
        };

        // 2. 释放文件占用的簇（file meta data中的first cluster和file size会被重置）
        self.file_manager.clear_file(&file_meta_data);


        // 3. 将文件大小置为0，first cluster置为None
        file_meta_data.file_size = 0;
        file_meta_data.first_cluster = None;

        // 4. 更新文件元数据
        self.index_entry_manager.modify_file(&file_meta_data);

        Some(())
    }

    /// 删除文件
    pub fn delete(&mut self, path: &String) -> Option<()> {
        // 流程：
        // 1. 查找文件上层目录
        // 2. 查找文件
        // 3. 释放文件占用的簇
        // 4. 在上层目录中删除文件目录项

        // 1. 查找文件上层目录
        let mut path = Self::path_to_dir_list(path);
        let file_name = path.pop().unwrap();

        if let Some(dir_cluster_id) = self.find_by_dir_list(&path).map(|dir| {
            dir.first_cluster
        }) {
            // 2. 在上层目录中查找文件
            if let Some(dir_cluster_id) = dir_cluster_id {
                if let Some(now_file_meta_data) = self.index_entry_manager.find_entry_by_name(&dir_cluster_id, &file_name) {
                    // 3. 释放文件占用的簇
                    self.file_manager.clear_file(&now_file_meta_data);

                    // 4. 在上层目录中删除文件目录项
                    self.index_entry_manager.delete_file_by_name(&dir_cluster_id, &file_name);

                    return Some(());
                }
            }
        }

        None
    }

    /// 读取文件
    pub fn read(&self, path: &String, offset: usize, buf: &mut [u8]) -> Option<usize> {
        // 流程：
        // 1. 查找文件
        // 2. 读取数据

        // 1. 查找文件
        let Some(file_meta_data) = self.find(path) else {
            return None;
        };

        // 2. 读取数据
        Some(self.file_manager.read_at(&file_meta_data, offset, buf))
    }

    /// 写入文件
    pub fn write(&mut self, path: &String, offset: usize, buf: &[u8]) -> Option<usize> {
        // 流程：
        // 1. 查找文件
        // 2. 写入数据
        // 3. 更新文件元数据

        // 1. 查找文件
        let Some(mut file_meta_data) = self.find(path) else {
            return None;
        };

        // 2. 写入数据
        let write_bytes = Some(self.file_manager.write_at(&mut file_meta_data, offset, buf));
        
        // 3. 更新文件元数据
        self.index_entry_manager.modify_file(&file_meta_data);

        write_bytes
    }

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

    fn find_by_dir_list(&self, path: &Vec<UnicodeString>) -> Option<FileMetaData> {
        let mut path: VecDeque<UnicodeString> = path.clone().into();

        if path.is_empty() {
            // 如果path为空，直接返回根目录
            let mut ret = FileMetaData::empty();
            ret.file_attributes= FileAttributes::empty().directory(true);
            ret.first_cluster = Some(ClusterId(self.boot_sector.first_cluster_of_root_directory));
            return Some(ret);
        }

        // 从根目录开始查找
        let mut current_target_name = path.pop_front().unwrap();
        let mut current_cluster_id = ClusterId(self.boot_sector.first_cluster_of_root_directory);

        loop {
            // 从当前目录中查找目标
            let now_file_meta_data = self.index_entry_manager.find_entry_by_name(&current_cluster_id, &current_target_name);

            if let Some(file_meta_data) = now_file_meta_data {
                // 如果找到了目标，查看path是否为空
                if path.is_empty() {
                    // 如果path为空，说明找到了目标
                    return Some(file_meta_data);
                } else {
                    // 如果path不为空，说明还需要继续查找
                    // 更新current_cluster_id和current_target_name
                    if let Some(dir_cluster_id) = file_meta_data.first_cluster {
                        current_cluster_id = dir_cluster_id;
                        current_target_name = path.pop_front().unwrap();
                        continue;
                    }
                }
            }
            // 在某次查找中丢失目标，查找失败
            return None;
        }
    }
}

impl Drop for ExFAT {
    fn drop(&mut self) {
        // 退出时将所有数据写回设备
        sync_all();
    }
}
