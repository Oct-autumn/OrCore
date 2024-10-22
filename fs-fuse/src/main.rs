use fs::ExFAT;
use std::fs::{File, OpenOptions};
use std::io;
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::{Arc, Mutex};
use fs::block_device::BlockDevice;
use fs::ex_fat::MetadataType;
use fs::ex_fat::model::index_entry::Attributes;

const BLOCK_SZ: usize = 512;

struct BlockFile(Mutex<File>);

impl BlockDevice for BlockFile {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        let mut file = self.0.lock().unwrap();
        file.seek(SeekFrom::Start((block_id * BLOCK_SZ) as u64))
            .expect("Error when seeking!");
        assert_eq!(file.read(buf).unwrap(), BLOCK_SZ, "Not a complete block!");
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) {
        let mut file = self.0.lock().unwrap();
        file.seek(SeekFrom::Start((block_id * BLOCK_SZ) as u64))
            .expect("Error when seeking!");
        assert_eq!(file.write(buf).unwrap(), BLOCK_SZ, "Not a complete block!");
    }

    fn num_blocks(&self) -> usize {
        let file = self.0.lock().unwrap();
        file.metadata().unwrap().len() as usize / BLOCK_SZ
    }
}

fn main() {
    // 在这里制作系统镜像，在OS运行前，写入到SD卡中
    // 系统镜像包含必要的文件（如页面文件）
}

#[allow(unused)]
fn test() {
    let device_size = 16 * 1024 * 1024; // 16MB
    
    print!("Simulating a 16MB block device...");
    io::stdout().flush().expect("Flush failed!");
    let block_file = Arc::new(BlockFile(Mutex::new({
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open("target/fs-back.img").unwrap();
        file.set_len(device_size).unwrap();
        file
    })));
    println!("Success.");

    println!("Mounting ExFAT file system...");
    let mut ex_fat = ExFAT::from_device(block_file.clone()).unwrap();
    println!("Success.");

    if let Some(res) = ex_fat.find(&String::from("/dir1")) {
        println!("The directory already exists.");
        let (Some(_), wrapped_dir_metadata) = res else { unreachable!() };
        let MetadataType::FileOrDir(dir_metadata) = wrapped_dir_metadata else { unreachable!() };
        println!("{:?}", dir_metadata);
    } else {
        print!("Creating a directory...");
        io::stdout().flush().expect("Flush failed!");
        let file_meta_data = ex_fat.touch(String::from("/dir1"),
                                          Attributes::empty().directory(true), 315532800000);
        assert!(file_meta_data.is_some());
        println!("Success.");
    }

    if let Some(res) = ex_fat.find(&String::from("/dir1/file1")) {
        println!("The file already exists.");
        let (Some(_), wrapped_dir_metadata) = res else { unreachable!() };
        let MetadataType::FileOrDir(file_metadata) = wrapped_dir_metadata else { unreachable!() };
        println!("{:?}", file_metadata);
    } else {
        print!("Creating a file...");
        io::stdout().flush().expect("Flush failed!");
        let file_meta_data = ex_fat.touch(String::from("/dir1/file1"),
                                          Attributes::empty().archive(true), 315532800000);
        assert!(file_meta_data.is_some());
        println!("Success.");
    }

    print!("Finding the dir...");
    io::stdout().flush().expect("Flush failed!");
    let res = ex_fat.find(&String::from("/dir1"));
    assert!(res.is_some());
    let (Some(_), wrapped_dir_metadata) = res.unwrap() else { unreachable!() };
    let MetadataType::FileOrDir(dir_metadata) = wrapped_dir_metadata else { unreachable!() };
    assert_eq!(dir_metadata.name.to_string().as_str(), "dir1");
    println!("Success.");
    println!("{:?}", dir_metadata);

    print!("Finding the file...");
    io::stdout().flush().expect("Flush failed!");
    let res = ex_fat.find(&String::from("/dir1/file1"));
    assert!(res.is_some());
    let (Some(wrapped_parent_dir_metadata), wrapped_dir_metadata) = res.unwrap() else { unreachable!() };
    let MetadataType::FileOrDir(mut file_metadata) = wrapped_dir_metadata else { unreachable!() };
    assert_eq!(file_metadata.name.to_string().as_str(), "file1");
    println!("Success.");
    println!("{:?}", file_metadata);

    println!("Sequential read and write test:");

    print!("- Writing to the file...");
    io::stdout().flush().expect("Flush failed!");
    // 随机生成256kB的数据
    let buf = (0..262144).map(|_| rand::random::<u8>()).collect::<Vec<u8>>();
    let write_bytes = ex_fat.write(&mut file_metadata, 0, buf.as_slice()).unwrap();
    ex_fat.update_file_metadata(&wrapped_parent_dir_metadata, file_metadata.clone());
    assert_eq!(write_bytes, buf.len());
    println!("Success.");

    print!("- Reading from the file...");
    io::stdout().flush().expect("Flush failed!");
    let mut read_buf = vec![0u8; buf.len()];
    let read_bytes = ex_fat.read(&file_metadata, 0, &mut read_buf).unwrap();
    assert_eq!(read_bytes, buf.len());
    assert_eq!(buf.as_slice(), &read_buf[..]);
    println!("Success.");

    println!("Random read and write test:");

    print!("- Writing to the file...");
    io::stdout().flush().expect("Flush failed!");
    // 随机生成4kB的数据
    let buf = (0..4096).map(|_| rand::random::<u8>()).collect::<Vec<u8>>();
    // 随机生成写入偏移量（0~256kB）
    let offset = (rand::random::<u32>() % 262144) as usize;
    let write_bytes = ex_fat.write(&mut file_metadata, offset, buf.as_slice()).unwrap();
    ex_fat.update_file_metadata(&wrapped_parent_dir_metadata, file_metadata.clone());
    assert_eq!(write_bytes, buf.len());
    println!("Success.");

    print!("- Reading from the file...");
    io::stdout().flush().expect("Flush failed!");
    let mut read_buf = vec![0u8; buf.len()];
    let read_bytes = ex_fat.read(&file_metadata, offset, &mut read_buf).unwrap();
    assert_eq!(read_bytes, buf.len());
    assert_eq!(buf.as_slice(), &read_buf[..]);
    println!("Success.");

    print!("Listing files in the root directory...");
    io::stdout().flush().expect("Flush failed!");
    let files = ex_fat.list(&String::from("/")).unwrap();
    for file in files {
        println!("/{}", file.name.to_string());
    }
    println!("Success.");

    print!("Listing files in the 'dir1' directory...");
    io::stdout().flush().expect("Flush failed!");
    let files = ex_fat.list(&String::from("/dir1")).unwrap();
    for file in files {
        println!("/dir1/{}", file.name.to_string());
    }
    println!("Success.");

    //print!("Deleting the file...");
    //io::stdout().flush().expect("Flush failed!");
    //ex_fat.delete(&String::from("/dir1/file1")).unwrap();
    //let res = ex_fat.find(&String::from("/dir1/file1"));
    //assert!(res.is_none());
    //println!("Success.");
    
    // Clean up
    print!("Unmounting ExFAT file system...");
    drop(ex_fat);
    println!("Success.");
    println!("Close the block device...");
    drop(block_file);
    println!("Success.");
    
    println!("All tests passed!");
    
}

