//! fs/src/ex_fat/persistent_layer/model/index_entry/volume_label.rs
//! 
//! 卷标目录项

use core::fmt::{Debug, Formatter};

use super::super::{
    index_entry::IndexEntryCostumeBytes,
    unicode_str::UnicodeString,
};

#[repr(C)]
#[derive(Clone)]
pub struct VolumeLabelCostume {
    pub volume_label_length: u8,
    pub volume_label: UnicodeString,
}

impl VolumeLabelCostume {
    pub fn new(volume_label: &UnicodeString) -> Self {
        Self {
            volume_label_length: volume_label.len() as u8,
            volume_label: volume_label.clone(),
        }
    }
}

impl IndexEntryCostumeBytes for VolumeLabelCostume {
    fn to_bytes(&self) -> [u8; 31] {
        let mut arr = [0; 31];
        arr[0] = self.volume_label_length;
        arr[1..(self.volume_label_length * 2 + 1) as usize].copy_from_slice(self.volume_label.to_le_bytes().as_slice());

        arr
    }

    fn from_bytes(arr: &[u8]) -> Self {
        assert_eq!(arr.len(), 31);
        let volume_label_length = arr[0];
        
        Self {
            volume_label_length: arr[0],
            volume_label: UnicodeString::from_le_bytes(arr[1..(volume_label_length * 2 + 1) as usize].as_ref()),
        }
    }
}

impl Debug for VolumeLabelCostume {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "VolumeLabelCostume {{ volume_label_length: {}, volume_label: {} }}", self.volume_label_length, self.volume_label.to_string())
    }
}