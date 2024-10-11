use fs::ex_fat::{FileAttributes, UnicodeString};
use fs::{BlockDevice, ExFAT};
use std::fs::{File, OpenOptions};
use std::io;
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::{Arc, Mutex};

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

    fn num_blocks(&self) -> u64 {
        let file = self.0.lock().unwrap();
        file.metadata().unwrap().len() / BLOCK_SZ as u64
    }
}

fn main() {
    let block_file = Arc::new(BlockFile(Mutex::new({
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open("target/fs.img").unwrap();
        file.set_len(512 * 256 * 256).unwrap(); // 256 cluster, 256 sectors per cluster, 512 Bytes per sector, 32MB
        file
    })));

    print!("Establishing ExFAT file system...");
    io::stdout().flush().expect("Flush failed!");
    let mut ex_fat = ExFAT::create(
        512,
        256,
        UnicodeString::from_str("TestExFAT"),
        block_file.clone(),
    );
    println!("Success.");

    print!("Creating a directory...");
    io::stdout().flush().expect("Flush failed!");
    let file_meta_data = ex_fat.touch(String::from("/dir1"),
                                      FileAttributes::empty().directory(true), 339202800000);
    assert!(file_meta_data.is_some());
    println!("Success.");

    print!("Creating a file...");
    io::stdout().flush().expect("Flush failed!");
    let file_meta_data = ex_fat.touch(String::from("/dir1/file1"),
                                      FileAttributes::empty().archive(true), 339202800000);
    assert!(file_meta_data.is_some());
    println!("Success.");

    print!("Finding the file...");
    io::stdout().flush().expect("Flush failed!");
    let res = ex_fat.find(&String::from("/dir1/file1"));
    assert!(res.is_some());
    assert_eq!(res.unwrap().file_name.to_string().as_str(), "file1");
    println!("Success.");

    println!("Sequential read and write test:");
    
    print!("- Writing to the file...");
    io::stdout().flush().expect("Flush failed!");
    // 随机生成128kB的数据
    let buf = (0..131072).map(|_| rand::random::<u8>()).collect::<Vec<u8>>();
    let write_bytes = ex_fat.write(&String::from("/dir1/file1"), 0, buf.as_slice()).unwrap();
    assert_eq!(write_bytes, buf.len());
    println!("Success.");

    print!("- Reading from the file...");
    io::stdout().flush().expect("Flush failed!");
    let mut read_buf = vec![0u8; buf.len()];
    let read_bytes = ex_fat.read(&String::from("/dir1/file1"), 0, &mut read_buf).unwrap();
    assert_eq!(read_bytes, buf.len());
    assert_eq!(buf.as_slice(), &read_buf[..]);
    println!("Success.");

    println!("Random read and write test:");

    print!("- Writing to the file...");
    io::stdout().flush().expect("Flush failed!");
    // 随机生成4kB的数据
    let buf = (0..4096).map(|_| rand::random::<u8>()).collect::<Vec<u8>>();
    let write_bytes = ex_fat.write(&String::from("/dir1/file1"), 20000, buf.as_slice()).unwrap();
    assert_eq!(write_bytes, buf.len());
    println!("Success.");
    
    print!("- Reading from the file...");
    io::stdout().flush().expect("Flush failed!");
    let mut read_buf = vec![0u8; buf.len()];
    let read_bytes = ex_fat.read(&String::from("/dir1/file1"), 20000, &mut read_buf).unwrap();
    assert_eq!(read_bytes, buf.len());
    assert_eq!(buf.as_slice(), &read_buf[..]);
    println!("Success.");

    print!("Listing files in the root directory...");
    io::stdout().flush().expect("Flush failed!");
    let files = ex_fat.list(&String::from("/")).unwrap();
    for file in files {
        println!("/{}", file.file_name.to_string());
    }
    println!("Success.");

    print!("Listing files in the 'dir1' directory...");
    io::stdout().flush().expect("Flush failed!");
    let files = ex_fat.list(&String::from("/dir1")).unwrap();
    for file in files {
        println!("/dir1/{}", file.file_name.to_string());
    }
    println!("Success.");
    
    println!("Unmounting ExFAT file system...");
    drop(ex_fat);
    
    println!("Mounting ExFAT file system...");
    let mut ex_fat = ExFAT::from_device(block_file.clone()).unwrap();

    print!("Deleting the file...");
    io::stdout().flush().expect("Flush failed!");
    ex_fat.delete(&String::from("/dir1/file1")).unwrap();
    let res = ex_fat.find(&String::from("/dir1/file1"));
    assert!(res.is_none());
    println!("Success.");
}

