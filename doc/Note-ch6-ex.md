## 6.ex exFAT文件系统

### 6.ex.1 exFAT文件系统简介

exFAT文件系统是微软公司为了解决FAT32文件系统在大容量存储设备上的不足而推出的一种新的文件系统。exFAT文件系统是FAT32文件系统的升级版，它支持更大的文件和更大的存储设备。exFAT文件系统的最大文件大小为16EB，最大存储设备大小为128PB。exFAT文件系统的簇大小可以达到32MB，这使得exFAT文件系统在大容量存储设备上的性能更好。

### 6.ex.2 块缓存机制

在文件系统中引入块缓存机制，将一部分磁盘中的盘块缓存到内存中，以提高文件系统的性能。块缓存机制可以减少磁盘I/O操作的次数，提高文件系统的读写性能。

我们使用LRU置换算法来管理块缓存。LRU算法是一种常用的性能优秀的页面置换算法，它根据页面的使用次数来决定哪个页面被置换出去。

不过我们从另一个角度实现该算法：我们用一个链表来存储所有缓存页面的指针，当一个页面被访问时，将该页面移到链表的头部；当需要置换页面时，将链表尾部的页面置换出去。

### 6.ex.3 exFAT文件系统的实现

整体上，我们以三个层次来构建exFAT文件系统：面向字节的底层数据结构，面向文件系统业务的业务逻辑，面向操作系统的文件操作接口。

#### 6.ex.3.1 面向字节的底层数据结构

##### 6.ex.3.1.1 **BootSector**

   BootSector是exFAT文件系统的引导扇区，它包含了文件系统的基本信息，如文件系统类型、簇大小、FAT表偏移量等。BootSector的结构如下：

   ```rust
   pub struct BootSector {
       /// **跳转字段** 0x000 3B
       pub jump_boot: [u8; 3],
       /// **文件系统名** 0x003 8B
       pub file_system_name: [u8; 8],
       /// **对齐** 0x00B 53B
       _must_be_zero: [u8; 53],
       /// **卷偏移量**（单位：扇区，为0时应忽略） 0x040 8B
       pub partition_offset: u64,
       /// **卷大小**（单位：扇区） 0x048 8B
       pub volume_length: u64,
       /// **FAT表偏移量**（单位：扇区） 0x050 4B
       pub fat_offset: u32,
       /// **FAT表长度**（单位：扇区） 0x054 4B
       pub fat_length: u32,
       /// **首簇偏移量**（单位：扇区） 0x058 4B
       pub cluster_heap_offset: u32,
       /// **卷内簇数量** 0x05C 4B
       pub cluster_count: u32,
       /// **根目录起始簇号** 0x060 4B
       pub first_cluster_of_root_directory: u32,
       /// **卷序列号（用于区分不同卷）** 0x064 4B
       pub volume_serial_number: u32,
       /// **文件系统版本号** 0x068 2B
       pub filesystem_revision: u16,
       /// **卷状态** 0x06A 2B
       pub volume_flags: VolumeFlags,
       /// **每扇区字节数描述**（`2^N`字节） 0x06C 1B
       pub bytes_per_sector_shift: u8,
       /// **每簇扇区数描述**（`2^N`扇区） 0x06D 1B
       pub sectors_per_cluster_shift: u8,
       /// **FAT表个数** 0x06E 1B
       pub number_of_fats: u8,
       /// **驱动标记** 0x06F 1B
       pub drive_select: u8,
       /// **分区使用百分比** 0x070 1B
       pub percent_in_use: u8,
       /// **保留区域** 0x071 7B
       _reserved: [u8; 7],
       /// **启动代码区** 0x078 390B
       pub boot_code: [u8; 390],
       /// **结束符**（固定为0x55AA） 0x1FE 2B
       pub boot_signature: u16,
   }
   ```

##### 6.ex.3.1.2 **FAT表**

   FAT表是管理簇链的数据结构。但我们不会在内存中构建一个完整的FAT表，那样占用的内存就太大了。我们会在内存中构建一个FAT表管理器，用于管理FAT表的读写操作。我们会在之后的小节中详细介绍。

##### 6.ex.3.1.3 **簇分配位图**

   簇分配位图是管理簇使用情况的数据结构。同样的，完整的簇分配位图也会占用大量内存，我们会在内存中构建一个簇分配位图管理器，用于管理簇分配位图的读写操作。我们会在之后的小节中详细介绍。

##### 6.ex.3.1.4 **目录项**

   目录项是文件系统中的一个重要数据结构，它用于描述文件和目录的属性。exFAT文件系统中的目录项都是由一个固定长度的结构体描述的。目录项的结构如下：

   ```rust
   pub struct IndexEntry {
       /// 目录项类型
       pub entry_type: IndexEntryType,
       /// 本条目由派生的目录项定义
       pub custom_defined: EntryCostume,
   }
   ```

   其它目录项均由该结构体“派生”而来，不同类型的目录项有不同的custom_defined字段。我们会为它们实现to_bytes和from_bytes方法，用于将目录项转换为32字节的数组和从32字节数组中解析出目录项。

   1. **卷标目录项** 该目录项的类型值为`83H`(二进制形式为`10000011B`)

      其自定义字段如下：
      ```rust
      pub struct VolumeLabelCostume {
          /// 卷标长度
          pub volume_label_length: u8,
          /// 卷标（UTF-16编码）
          pub volume_label: UnicodeString,
      }
      ```

   2. **簇分配位图目录项、大写字母表目录项**

      在exFAT文件系统中，簇分配位图和大写字母表均是作为一个文件，分别存储在2号簇和3号簇中。它们的目录项格式完全一致，我将其称为“文件系统文件专用目录项”。

       - `簇分配位图目录项`的类型值为`81H`(二进制形式为`10000001B`)
       - `大写字母表目录项`的类型值为`82H`(二进制形式为`10000010B`)

      其自定义字段如下：
      ```rust
      pub struct FsFileCostume {
          /// 起始簇号
          first_cluster: ClusterId,
          /// 文件大小（单位：字节）
          data_length: u64,
      }
      ```

   3. **文件/文件夹目录项**
       1. **属性项1** 该目录项的类型值为`85H`(二进制形式为`10000101B`)
           ```rust
           pub struct FileDirectoryCostume1 {
               pub secondary_count: u8,
               pub set_check_sum: IndexEntryChecksum,
               pub file_attributes: FileAttributes,
               pub create_time_stamp: TimeStamp,
               pub last_modified_time_stamp: TimeStamp,
               pub last_accessed_time_stamp: TimeStamp,
               pub create_10ms_increment: u8,
           }
           ```
       2. **属性项2** 该目录项的类型值为`C0H`(二进制形式为`11000000B`)
           ```rust
           pub struct FileDirectoryCostume2 {
               pub fragment_flag: FragmentFlag,
               pub file_name_length: u8,
               pub file_name_hash: FileNameHash,
               pub file_size1: u64,
               pub start_cluster: ClusterId,
               pub file_size2: u64,
           }
           ```
       3. **属性项3** 该目录项的类型值为`C1H`(二进制形式为`11000001B`)
           ```rust
           pub struct FileDirectoryCostume3 {
               pub file_name: UnicodeString,
           }
           ```

#### 6.ex.3.2 面向文件系统业务的业务逻辑

首先，我们要构建两个对接底层数据结构的管理器：FAT表管理器和簇分配位图管理器。

##### 6.ex.3.2.1 **FAT表管理器**

   FAT表管理器用于管理FAT表的读写操作。FAT表管理器的结构如下：

   ```rust
   pub struct FileAllocationTable {
       /// 扇区大小描述
       bytes_per_sector_shift: u8,
       /// FAT起始扇区
       start_sector: u32,
       /// FAT长度（单位：扇区）
       length: u32,
       /// 存储设备接口指针
       device: Arc<dyn BlockDevice>,
   }
   ```

   FAT表管理器提供了读取和写入FAT表的方法：

   ```rust
   impl FileAllocationTable {
       /// 读取FAT表项
       fn read_fat_entry(&self, cluster: u32) -> Option<u32> {/*略*/}
       /// 写入FAT表项
       fn write_fat_entry(&self, cluster: u32, value: u32) -> Option<()> {/*略*/}
   }
   ```
   
   但我们不直接向外提供这两个方法，而是提供两个更高层次封装的方法：`get_next_cluster`、`set_next_cluster`。`get_next_cluster`方法用于获取下一个簇号，如果没有下一个簇号则返回None。`set_next_cluster`方法则用于设置下一个簇号。

   ```rust
   impl FileAllocationTable {
       /// 获取文件的下一个簇号
       pub fn get_next_cluster(&self, cluster_id: &ClusterId) -> Option<ClusterId> {
           if cluster_id.is_free() || cluster_id.is_bad_cluster() || cluster_id.is_end_of_file() {
               // 未分配簇或坏簇或最后一个簇
               None
           } else {
               // 查找下一个簇号
               self.get_entry(&cluster_id).map(|entry| ClusterId(entry))
           }
       }
   
       /// 设置文件的下一个簇号
       pub fn set_next_cluster(
           &self,
           cluster_id: &ClusterId,
           next_cluster_id: &ClusterId,
       ) -> Option<()> {
           if cluster_id.is_free() || cluster_id.is_bad_cluster() || cluster_id.is_end_of_file() {
               // 未分配簇或坏簇或最后一个簇
               None
           } else {
               // 设置下一个簇号
               self.set_entry(&cluster_id, &next_cluster_id)
           }
       }
   }
   ```
   
   另外，我们还提供了一个`init_fat_on_device`方法，用于初始化磁盘上的FAT表，一个`check_validate`方法，用于检查FAT表的有效性。

   它们的实现都比较简单，就不在这里展示了。

##### 6.ex.3.2.2 **簇分配位图管理器**

   簇分配位图管理器用于管理簇分配位图的读写操作。簇分配位图管理器的结构如下：

   ```rust
   pub struct ClusterAllocBitmap {
       /// 位图
       pub cluster_bitmap: Bitmap,
       /// 存储设备接口指针
       pub device: Arc<dyn BlockDevice>,
   }
   ```
   
   我们在其中实现了`alloc`、`free`、`is_using`三个方法，分别用于分配簇、释放簇、判断簇是否被使用。

   ```rust
   impl ClusterAllocBitmap {
       /// 分配一个簇
       pub fn alloc(&mut self) -> Option<ClusterId> {
           self.cluster_bitmap
               .alloc(&self.device)
               .map(|id| ClusterId::from(id as u32))
       }
    
       /// 释放一个簇
       pub fn free(&mut self, cluster: &ClusterId) {
           self.cluster_bitmap.free(&self.device, cluster.0 as usize);
       }
    
       /// 判断簇是否被使用
       pub fn is_using(&self, cluster: &ClusterId) -> bool {
           self.cluster_bitmap
               .is_used(&self.device, cluster.0 as usize)
       }
   }
   ```

   （`Bitmap`的具体实现见源码）

##### 6.ex.3.2.3 **簇管理器**

   簇管理器是FAT表管理器和簇分配位图管理器的封装，它提供了更高层次的接口，用于管理簇的分配、释放。并提供了簇内按扇区访问功能。簇管理器的结构如下：

   ```rust
   pub struct ClusterManager {
       /// 首簇起始位置
       cluster_heap_offset: u32,
       /// 每簇扇区数
       sectors_per_cluster: u32,
       /// 存储设备接口指针
       device: Arc<dyn BlockDevice>,
       /// FAT表
       file_allocation_table: FileAllocationTable,
       /// 簇分配位图
       cluster_alloc_bitmap: ClusterAllocBitmap,
   }
   ```
   
   簇管理器提供了`alloc_new_cluster`、`free_next_cluster`两个方法，用于分配和释放簇。它也提供高级封装：`alloc_and_append_cluster`，用于分配一个新簇并将其追加到文件的末尾。同时，它还提供了`get_next_cluster`方法，用于获取下一个簇号。

   ```rust
   impl ClusterManager {
       /// 释放当前簇的下一簇
       fn free_next_cluster(&mut self, cluster_id: &ClusterId) -> Option<()> {
           // 检查下一簇是否存在
           if let Some(next_cluster_id) = self.file_allocation_table.get_next_cluster(cluster_id) {
               // 检查下一簇是否被分配
               // 簇能被释放的条件：
               //   是最后一簇 -> FAT表项为EOF
               if self.cluster_alloc_bitmap.is_using(&next_cluster_id)
                   && self.is_last_cluster(&next_cluster_id)
               {
                   // 设置下一簇的FAT表项为FREE状态
                   self.file_allocation_table
                       .set_next_cluster(&next_cluster_id, &ClusterId::free());
                   // 释放下一簇
                   self.cluster_alloc_bitmap.free(&next_cluster_id);
                   // 设置当前簇的FAT表项为EOF
                   self.file_allocation_table
                       .set_next_cluster(cluster_id, &ClusterId::eof());
                   return Some(());
               }
           }
           None
       }
   
       /// 申请新的簇
       pub fn alloc_new_cluster(&mut self) -> Option<ClusterId> {
           if let Some(cluster_id) = self.cluster_alloc_bitmap.alloc() {
               self.file_allocation_table
                   .set_next_cluster(&cluster_id, &ClusterId::eof());
               Some(cluster_id)
           } else {
               None
           }
       }
   
       /// 为簇链申请并附加新簇
       ///
       /// 只支持在簇链的最后一簇上调用，非最后一簇会拒绝附加并返回None
       pub fn alloc_and_append_cluster(&mut self, cluster_id: &ClusterId) -> Option<ClusterId> {
           if self.is_last_cluster(cluster_id) {
               let new_cluster_id = self.alloc_new_cluster()?;
               self.file_allocation_table
                   .set_next_cluster(cluster_id, &new_cluster_id);
               self.file_allocation_table
                   .set_next_cluster(&new_cluster_id, &ClusterId::eof());
               Some(new_cluster_id)
           } else {
               None
           }
       }
   
       /// 获取下一簇的簇号
       pub fn get_next_cluster(&self, cluster_id: &ClusterId) -> Option<ClusterId> {
           self.file_allocation_table.get_next_cluster(cluster_id)
       }
   }
   ```
   
   另外，簇管理器也提供了根据簇号和簇内偏移量进行扇区访问的方法：`get_cluster_sector`

   ```rust
   /// 获取簇号的首个扇区号
   impl ClusterManager {
       /// 获取簇号的首个扇区号
       fn get_cluster_first_sector(&self, cluster_id: &ClusterId) -> u32 {
           self.cluster_heap_offset + (cluster_id.0 - 2) * self.sectors_per_cluster
       }

       /// 获取指定簇的指定扇区偏移的缓存
       ///
       /// 当簇偏移超出当前簇的扇区范围时，会自动查找下一簇
       pub fn get_cluster_sector(&self, cluster_id: &ClusterId, cluster_offset: u32) -> Arc<RwLock<BlockCache>> {
           if cluster_offset >= self.sectors_per_cluster {
               // 超出该簇的扇区范围，尝试查找目标簇
               let mut cluster_id = cluster_id.clone();
               let mut cluster_offset = cluster_offset;
               while let Some(next_cluster_id) = self.file_allocation_table.get_next_cluster(&cluster_id) {
                   cluster_id = next_cluster_id;
                   cluster_offset -= self.sectors_per_cluster;
                   if cluster_offset < self.sectors_per_cluster {
                       return get_block_cache((self.get_cluster_first_sector(&cluster_id) + cluster_offset) as usize, &self.device);
                   }
               }
               panic!("ClusterManager: cluster not found");
           } else {
               // 未超出该簇的扇区范围，直接获取
               get_block_cache((self.get_cluster_first_sector(cluster_id) + cluster_offset) as usize, &self.device)
           }
       }
   }
   ```

   簇管理器将被目录管理器和文件管理器使用。因为会被两个及以上的对象共同拥有可变引用，所以我们需要使用`RwLock`来保证线程安全。

##### 6.ex.3.2.4 **目录管理器**
   
   目录管理器用于管理目录项（主要是文件和文件夹）的读写操作。目录管理器的结构如下：

   ```rust
   pub struct IndexEntryManager {
       /// 大写字母表
       upcase_table: Arc<UpCaseTable>,
       /// 簇管理器
       cluster_manager: Arc<RwLock<ClusterManager>>,
       /// 存储设备接口指针
       device: Arc<dyn BlockDevice>,
   }
   ```
   
   目录管理器提供了`create_file`、`find_entry_by_name`、`get_file_by_pos`、`delete_file_by_name`、`delete_file_by_pos`、`modify_file`等方法，用于创建、查找、读取、删除、修改文件目录项。

   ```rust
   impl DirectoryManager {
       /// 创建文件
       pub fn create_file(&mut self, file_meta_data: &FileMetaData, parent_cluster_id: &ClusterId) -> Option<(ClusterId, u32, u32)> {/* 略 */}
       /// 根据文件名查找文件
       pub fn find_entry_by_name(&self, cluster_id: &ClusterId, name: &UnicodeString) -> Option<FileMetaData> {/* 略 */}
       /// 根据位置查找文件
       pub fn get_file_by_pos(&self, cluster_id: &ClusterId, sector_offset_in_cluster: u32, entry_offset_in_sector: u32) -> Option<FileMetaData> {/* 略 */}
       /// 根据文件名删除文件
       pub fn delete_file_by_name(&self, cluster_id: &ClusterId, name: &UnicodeString) {/* 略 */}
       /// 根据位置删除文件
       pub fn delete_file_by_pos(&self, cluster_id: &ClusterId, sector_offset_in_cluster: u32, entry_offset_in_sector: u32) {/* 略 */}
       /// 修改文件
       pub fn modify_file(&mut self, file_meta_data: &FileMetaData) {/* 略 */}
       /// 列出目录下的文件
       pub fn list_files(&self, cluster_id: &ClusterId) -> Vec<FileMetaData> {/* 略 */}
   }
   ```

##### 6.ex.3.2.5 **文件管理器**

   文件管理器用于管理文件的读写操作。文件管理器的结构如下：

   ```rust
   pub struct FileManager {
       /// 簇管理器
       cluster_manager: Arc<RwLock<ClusterManager>>,
       /// 存储设备接口指针
       device: Arc<dyn BlockDevice>,
   }
   ```

   文件管理器提供了`read_file`、`write_file`、`create_file`、`delete_file`等方法，用于读取、写入、创建、删除文件。
    
   ```rust
   ```

#### 6.ex.3.3 面向操作系统的文件操作接口

   整个文件系统的操作接口都是基于上述的业务逻辑实现的。我们提供了`ExFAT`结构体，用于对外提供文件系统的操作接口。

   ```rust
   pub struct ExFAT {
       device: Arc<dyn BlockDevice>,
       boot_sector: BootSector,
       directory_manager: DirectoryManager,
       file_manager: FileManager,
   }
   ```

   可以看到，我们在其中封装了目录管理器和文件管理器，接下来我们要实现一些文件系统的操作接口，如`create`、`open`、`read`、`write`、`delete`等。

   ```rust
   ```

### 6.ex.4 exFAT文件系统

### 6.ex.5 命令行操作
   文件系统指令：`ls`列出、`touch`创建、`mv`移动、`cp`拷贝、`cat`查看、`rm`删除、`find`查找、`mkdir`创建目录
   
   ls命令：列出目标目录下的文件和目录。
   touch命令：创建一个新文件。先在目标目录下查找是否有同名文件，如果有则返回错误；否则创建一个新文件。
   mv命令：移动文件或目录。先在源目录下查找源文件或目录，若找不到则返回错误；然后在目标目录下查找是否有同名文件或目录，若有则返回错误；若检查都通过，则移动文件或目录。（移动文件本质上是将文件的目录项从源目录中删除，然后在目标目录中创建一个新的目录项。）
   cp命令：拷贝文件或目录。先在源目录下查找源文件或目录，若找不到则返回错误；然后在目标目录下查找是否有同名文件或目录，若有则返回错误；若检查都通过，则拷贝文件或目录。（拷贝文件本质上是创建一个新文件，然后将源文件的内容复制到新文件中。）
   cat命令：查看文件内容。先在目标目录下查找文件，若找不到则返回错误；然后读取文件内容并输出。
   rm命令：删除文件或目录。先在目标目录下查找文件或目录，若找不到则返回错误；然后删除文件或目录。
   