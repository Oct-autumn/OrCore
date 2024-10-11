//! fs/src/ex_fat/persistent_layer/model/unicode_str.rs
//! 
//! Unicode字符串

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Debug;

#[derive(Eq, PartialEq, Clone)]
pub struct UnicodeString {
    pub data: Vec<u16>,
}

impl UnicodeString {
    pub fn new() -> Self {
        UnicodeString { data: Vec::new() }
    }
    
    pub fn to_le_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for &c in &self.data {
            bytes.push((c & 0x00FF) as u8);
            bytes.push((c & 0xFF00) as u8);
        }
        bytes
    }
    
    pub fn from_le_bytes(bytes: &[u8]) -> Self {
        assert_eq!(bytes.len() % 2, 0); // bytes长度必须是偶数
        let mut data = Vec::new();
        for i in 0..bytes.len() / 2 {
            data.push(u16::from_le_bytes([bytes[i * 2], bytes[i * 2 + 1]]));
        }
        UnicodeString { data }
    }

    pub fn from_str(s: &str) -> Self {
        UnicodeString {
            data: s.encode_utf16().collect(),
        }
    }
    
    pub fn from_string(s: &String) -> Self {
        UnicodeString {
            data: s.encode_utf16().collect(),
        }
    }

    pub fn to_string(&self) -> String {
        String::from_utf16_lossy(&self.data)
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn get_char(&self, index: usize) -> u16 {
        self.data[index]
    }

    pub fn get_le_char(&self, index: usize) -> u16 {
        self.data[index].to_le()
    }

    pub fn set_char(&mut self, index: usize, value: u16) {
        self.data[index] = value;
    }

    pub fn set_le_char(&mut self, index: usize, value: u16) {
        self.data[index] = value.to_le();
    }

    pub fn push(&mut self, value: u16) {
        self.data.push(value);
    }
    
    pub fn clear(&mut self) {
        self.data.clear();
    }
    
    pub fn append(&mut self, other: &Self) {
        self.data.extend_from_slice(&other.data);
    }

    pub fn slice(&self, start: usize, end: usize) -> Self {
        UnicodeString {
            data: self.data[start..end].to_vec(),
        }
    }
}

impl Debug for UnicodeString {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{:?}", self.to_string())
    }
}
