//! fs/src/ex_fat/persistent_layer/up_case_table
//!
//! 大写字符映射表、文件名哈希

use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use core::cmp::min;
use spin::RwLock;
use crate::config;
use crate::ex_fat::persistent_layer::ClusterManager;
use crate::ex_fat::persistent_layer::model::unicode_str::UnicodeString;

pub struct UpCaseTable(pub [u16; 2918]);

impl UpCaseTable {
    fn lookup(&self, c: u16) -> u16 {
        if (c as usize) < self.0.len() {
            self.0[c as usize]
        } else {
            c
        }
    }
    
    /// 将Unicode字符串转成大写字符串
    pub fn to_upper(&self, uni_str: &UnicodeString) -> Vec<u8> {
        let mut up_str: Vec<u8> = Vec::new();
        for c in uni_str.data.iter() {
            let up_c = self.lookup(*c);
            // 小端模式：低位在前，高位在后
            up_str.push((up_c & 0x00FF) as u8);
            up_str.push((up_c & 0xFF00) as u8);
        }
        up_str
    }
    
    /// 将大写字符表写入到磁盘中
    pub fn save(&self, cluster_manager: &Arc<RwLock<ClusterManager>>) {
        let mut cluster_manager = cluster_manager.write();
        
        // 申请一个簇（簇号应为3），并将大写字符表写入到这个簇中
        let cluster_id = cluster_manager.alloc_new_cluster().unwrap();
        assert_eq!(cluster_id.0, 3);   // 大写字符表的簇号应为3
        
        // 将大写字符表写入到簇中        
        let sector_len = (2918 * 2 + config::SECTOR_BYTES - 1) / config::SECTOR_BYTES;
        for i in 0..sector_len {
            let start = (i * config::SECTOR_BYTES) >> 1;
            let end = min(((i + 1) * config::SECTOR_BYTES) >> 1, self.0.len());
            cluster_manager.get_cluster_sector(&cluster_id, i as u32).unwrap().write().modify_and_sync(0, |data: &mut [u16; config::SECTOR_BYTES / 2]| {
                data[0..(end - start)].copy_from_slice(&self.0[start..end]);
            });
        }
    }
    
    /// 大写字符映射表
    pub fn generate_up_case_table() -> Self {
        let mut table: Vec<u16> = Vec::new();
        let mut tmp: Vec<u16>;

        // 生成映射表
        for c in 0x0000..=0x0060 {
            table.push(c);
        }
        for c in 0x0041..=0x005A {
            table.push(c);
        }
        for c in 0x007B..=0x00DF {
            table.push(c);
        }
        for c in 0x00C0..=0x00D6 {
            table.push(c);
        }
        tmp = vec![0x00F7];
        table.append(&mut tmp);
        for c in 0x00D8..=0x00DE {
            table.push(c);
        }
        tmp = vec![
            0x0178, 0x0100, 0x0100, 0x0102, 0x0102, 0x0104, 0x0104, 0x0106, 0x0106, 0x0108, 0x0108,
            0x010A, 0x010A, 0x010C, 0x010C, 0x010E, 0x010E, 0x0110, 0x0110, 0x0112, 0x0112, 0x0114,
            0x0114, 0x0116, 0x0116, 0x0118, 0x0118, 0x011A, 0x011A, 0x011C, 0x011C, 0x011E, 0x011E,
            0x0120, 0x0120, 0x0122, 0x0122, 0x0124, 0x0124, 0x0126, 0x0126, 0x0128, 0x0128, 0x012A,
            0x012A, 0x012C, 0x012C, 0x012E, 0x012E,
        ];
        table.append(&mut tmp);
        for c in 0x0130..=0x0132 {
            table.push(c);
        }
        tmp = vec![0x0132, 0x0134, 0x0134, 0x0136, 0x0136];
        table.append(&mut tmp);
        for c in 0x0138..=0x0139 {
            table.push(c);
        }
        tmp = vec![
            0x0139, 0x013B, 0x013B, 0x013D, 0x013D, 0x013F, 0x013F, 0x0141, 0x0141, 0x0143, 0x0143,
            0x0145, 0x0145, 0x0147, 0x0147,
        ];
        table.append(&mut tmp);
        for c in 0x0149..=0x014A {
            table.push(c);
        }
        tmp = vec![
            0x014A, 0x014C, 0x014C, 0x014E, 0x014E, 0x0150, 0x0150, 0x0152, 0x0152, 0x0154, 0x0154,
            0x0156, 0x0156, 0x0158, 0x0158, 0x015A, 0x015A, 0x015C, 0x015C, 0x015E, 0x015E, 0x0160,
            0x0160, 0x0162, 0x0162, 0x0164, 0x0164, 0x0166, 0x0166, 0x0168, 0x0168, 0x016A, 0x016A,
            0x016C, 0x016C, 0x016E, 0x016E, 0x0170, 0x0170, 0x0172, 0x0172, 0x0174, 0x0174, 0x0176,
            0x0176,
        ];
        table.append(&mut tmp);
        for c in 0x0178..=0x0179 {
            table.push(c);
        }
        tmp = vec![0x0179, 0x017B, 0x017B, 0x017D, 0x017D, 0x017F, 0x0243];
        table.append(&mut tmp);
        for c in 0x0181..=0x0182 {
            table.push(c);
        }
        tmp = vec![0x0182, 0x0184, 0x0184];
        table.append(&mut tmp);
        for c in 0x0186..=0x0187 {
            table.push(c);
        }
        tmp = vec![0x0187];
        table.append(&mut tmp);
        for c in 0x0189..=0x018B {
            table.push(c);
        }
        tmp = vec![0x018B];
        table.append(&mut tmp);
        for c in 0x018D..=0x0191 {
            table.push(c);
        }
        tmp = vec![0x0191];
        table.append(&mut tmp);
        for c in 0x0193..=0x0194 {
            table.push(c);
        }
        tmp = vec![0x01F6];
        table.append(&mut tmp);
        for c in 0x0196..=0x0198 {
            table.push(c);
        }
        tmp = vec![0x0198, 0x023D];
        table.append(&mut tmp);
        for c in 0x019B..=0x019D {
            table.push(c);
        }
        tmp = vec![0x0220];
        table.append(&mut tmp);
        for c in 0x019F..=0x01A0 {
            table.push(c);
        }
        tmp = vec![0x01A0, 0x01A2, 0x01A2, 0x01A4, 0x01A4];
        table.append(&mut tmp);
        for c in 0x01A6..=0x01A7 {
            table.push(c);
        }
        tmp = vec![0x01A7];
        table.append(&mut tmp);
        for c in 0x01A9..=0x01AC {
            table.push(c);
        }
        tmp = vec![0x01AC];
        table.append(&mut tmp);
        for c in 0x01AE..=0x01AF {
            table.push(c);
        }
        tmp = vec![0x01AF];
        table.append(&mut tmp);
        for c in 0x01B1..=0x01B3 {
            table.push(c);
        }
        tmp = vec![0x01B3, 0x01B5, 0x01B5];
        table.append(&mut tmp);
        for c in 0x01B7..=0x01B8 {
            table.push(c);
        }
        tmp = vec![0x01B8];
        table.append(&mut tmp);
        for c in 0x01BA..=0x01BC {
            table.push(c);
        }
        tmp = vec![0x01BC, 0x01BE, 0x01F7];
        table.append(&mut tmp);
        for c in 0x01C0..=0x01C5 {
            table.push(c);
        }
        tmp = vec![0x01C4];
        table.append(&mut tmp);
        for c in 0x01C7..=0x01C8 {
            table.push(c);
        }
        tmp = vec![0x01C7];
        table.append(&mut tmp);
        for c in 0x01CA..=0x01CB {
            table.push(c);
        }
        tmp = vec![
            0x01CA, 0x01CD, 0x01CD, 0x01CF, 0x01CF, 0x01D1, 0x01D1, 0x01D3, 0x01D3, 0x01D5, 0x01D5,
            0x01D7, 0x01D7, 0x01D9, 0x01D9, 0x01DB, 0x01DB, 0x018E, 0x01DE, 0x01DE, 0x01E0, 0x01E0,
            0x01E2, 0x01E2, 0x01E4, 0x01E4, 0x01E6, 0x01E6, 0x01E8, 0x01E8, 0x01EA, 0x01EA, 0x01EC,
            0x01EC, 0x01EE, 0x01EE,
        ];
        table.append(&mut tmp);
        for c in 0x01F0..=0x01F2 {
            table.push(c);
        }
        tmp = vec![0x01F1, 0x01F4, 0x01F4];
        table.append(&mut tmp);
        for c in 0x01F6..=0x01F8 {
            table.push(c);
        }
        tmp = vec![
            0x01F8, 0x01FA, 0x01FA, 0x01FC, 0x01FC, 0x01FE, 0x01FE, 0x0200, 0x0200, 0x0202, 0x0202,
            0x0204, 0x0204, 0x0206, 0x0206, 0x0208, 0x0208, 0x020A, 0x020A, 0x020C, 0x020C, 0x020E,
            0x020E, 0x0210, 0x0210, 0x0212, 0x0212, 0x0214, 0x0214, 0x0216, 0x0216, 0x0218, 0x0218,
            0x021A, 0x021A, 0x021C, 0x021C, 0x021E, 0x021E,
        ];
        table.append(&mut tmp);
        for c in 0x0220..=0x0222 {
            table.push(c);
        }
        tmp = vec![
            0x0222, 0x0224, 0x0224, 0x0226, 0x0226, 0x0228, 0x0228, 0x022A, 0x022A, 0x022C, 0x022C,
            0x022E, 0x022E, 0x0230, 0x0230, 0x0232, 0x0232,
        ];
        table.append(&mut tmp);
        for c in 0x0234..=0x0239 {
            table.push(c);
        }
        tmp = vec![0x2C65, 0x023B, 0x023B, 0x023D, 0x2C66];
        table.append(&mut tmp);
        for c in 0x023F..=0x0241 {
            table.push(c);
        }
        tmp = vec![0x0241];
        table.append(&mut tmp);
        for c in 0x0243..=0x0246 {
            table.push(c);
        }
        tmp = vec![
            0x0246, 0x0248, 0x0248, 0x024A, 0x024A, 0x024C, 0x024C, 0x024E, 0x024E,
        ];
        table.append(&mut tmp);
        for c in 0x0250..=0x0252 {
            table.push(c);
        }
        tmp = vec![0x0181, 0x0186, 0x0255];
        table.append(&mut tmp);
        for c in 0x0189..=0x018A {
            table.push(c);
        }
        tmp = vec![0x0258, 0x018F, 0x025A, 0x0190];
        table.append(&mut tmp);
        for c in 0x025C..=0x025F {
            table.push(c);
        }
        tmp = vec![0x0193];
        table.append(&mut tmp);
        for c in 0x0261..=0x0262 {
            table.push(c);
        }
        tmp = vec![0x0194];
        table.append(&mut tmp);
        for c in 0x0264..=0x0267 {
            table.push(c);
        }
        tmp = vec![0x0197, 0x0196, 0x026A, 0x2C62];
        table.append(&mut tmp);
        for c in 0x026C..=0x026E {
            table.push(c);
        }
        tmp = vec![0x019C];
        table.append(&mut tmp);
        for c in 0x0270..=0x0271 {
            table.push(c);
        }
        tmp = vec![0x019D];
        table.append(&mut tmp);
        for c in 0x0273..=0x0274 {
            table.push(c);
        }
        tmp = vec![0x019F];
        table.append(&mut tmp);
        for c in 0x0276..=0x027C {
            table.push(c);
        }
        tmp = vec![0x2C64];
        table.append(&mut tmp);
        for c in 0x027E..=0x027F {
            table.push(c);
        }
        tmp = vec![0x01A6];
        table.append(&mut tmp);
        for c in 0x0281..=0x0282 {
            table.push(c);
        }
        tmp = vec![0x01A9];
        table.append(&mut tmp);
        for c in 0x0284..=0x0287 {
            table.push(c);
        }
        tmp = vec![0x01AE, 0x0244];
        table.append(&mut tmp);
        for c in 0x01B1..=0x01B2 {
            table.push(c);
        }
        tmp = vec![0x0245];
        table.append(&mut tmp);
        for c in 0x028D..=0x0291 {
            table.push(c);
        }
        tmp = vec![0x01B7];
        table.append(&mut tmp);
        for c in 0x0293..=0x037A {
            table.push(c);
        }
        for c in 0x03FD..=0x03FF {
            table.push(c);
        }
        for c in 0x037E..=0x03AB {
            table.push(c);
        }
        tmp = vec![0x0386];
        table.append(&mut tmp);
        for c in 0x0388..=0x038A {
            table.push(c);
        }
        tmp = vec![0x03B0];
        table.append(&mut tmp);
        for c in 0x0391..=0x03A1 {
            table.push(c);
        }
        tmp = vec![0x03A3];
        table.append(&mut tmp);
        for c in 0x03A3..=0x03AB {
            table.push(c);
        }
        tmp = vec![0x038C];
        table.append(&mut tmp);
        for c in 0x038E..=0x038F {
            table.push(c);
        }
        for c in 0x03CF..=0x03D8 {
            table.push(c);
        }
        tmp = vec![
            0x03D8, 0x03DA, 0x03DA, 0x03DC, 0x03DC, 0x03DE, 0x03DE, 0x03E0, 0x03E0, 0x03E2, 0x03E2,
            0x03E4, 0x03E4, 0x03E6, 0x03E6, 0x03E8, 0x03E8, 0x03EA, 0x03EA, 0x03EC, 0x03EC, 0x03EE,
            0x03EE,
        ];
        table.append(&mut tmp);
        for c in 0x03F0..=0x03F1 {
            table.push(c);
        }
        tmp = vec![0x03F9];
        table.append(&mut tmp);
        for c in 0x03F3..=0x03F7 {
            table.push(c);
        }
        tmp = vec![0x03F7];
        table.append(&mut tmp);
        for c in 0x03F9..=0x03FA {
            table.push(c);
        }
        tmp = vec![0x03FA];
        table.append(&mut tmp);
        for c in 0x03FC..=0x042F {
            table.push(c);
        }
        for c in 0x0410..=0x042F {
            table.push(c);
        }
        for c in 0x0400..=0x040F {
            table.push(c);
        }
        tmp = vec![
            0x0460, 0x0460, 0x0462, 0x0462, 0x0464, 0x0464, 0x0466, 0x0466, 0x0468, 0x0468, 0x046A,
            0x046A, 0x046C, 0x046C, 0x046E, 0x046E, 0x0470, 0x0470, 0x0472, 0x0472, 0x0474, 0x0474,
            0x0476, 0x0476, 0x0478, 0x0478, 0x047A, 0x047A, 0x047C, 0x047C, 0x047E, 0x047E, 0x0480,
            0x0480,
        ];
        table.append(&mut tmp);
        for c in 0x0482..=0x048A {
            table.push(c);
        }
        tmp = vec![
            0x048A, 0x048C, 0x048C, 0x048E, 0x048E, 0x0490, 0x0490, 0x0492, 0x0492, 0x0494, 0x0494,
            0x0496, 0x0496, 0x0498, 0x0498, 0x049A, 0x049A, 0x049C, 0x049C, 0x049E, 0x049E, 0x04A0,
            0x04A0, 0x04A2, 0x04A2, 0x04A4, 0x04A4, 0x04A6, 0x04A6, 0x04A8, 0x04A8, 0x04AA, 0x04AA,
            0x04AC, 0x04AC, 0x04AE, 0x04AE, 0x04B0, 0x04B0, 0x04B2, 0x04B2, 0x04B4, 0x04B4, 0x04B6,
            0x04B6, 0x04B8, 0x04B8, 0x04BA, 0x04BA, 0x04BC, 0x04BC, 0x04BE, 0x04BE,
        ];
        table.append(&mut tmp);
        for c in 0x04C0..=0x04C1 {
            table.push(c);
        }
        tmp = vec![
            0x04C1, 0x04C3, 0x04C3, 0x04C5, 0x04C5, 0x04C7, 0x04C7, 0x04C9, 0x04C9, 0x04CB, 0x04CB,
            0x04CD, 0x04CD, 0x04C0, 0x04D0, 0x04D0, 0x04D2, 0x04D2, 0x04D4, 0x04D4, 0x04D6, 0x04D6,
            0x04D8, 0x04D8, 0x04DA, 0x04DA, 0x04DC, 0x04DC, 0x04DE, 0x04DE, 0x04E0, 0x04E0, 0x04E2,
            0x04E2, 0x04E4, 0x04E4, 0x04E6, 0x04E6, 0x04E8, 0x04E8, 0x04EA, 0x04EA, 0x04EC, 0x04EC,
            0x04EE, 0x04EE, 0x04F0, 0x04F0, 0x04F2, 0x04F2, 0x04F4, 0x04F4, 0x04F6, 0x04F6, 0x04F8,
            0x04F8, 0x04FA, 0x04FA, 0x04FC, 0x04FC, 0x04FE, 0x04FE, 0x0500, 0x0500, 0x0502, 0x0502,
            0x0504, 0x0504, 0x0506, 0x0506, 0x0508, 0x0508, 0x050A, 0x050A, 0x050C, 0x050C, 0x050E,
            0x050E, 0x0510, 0x0510, 0x0512, 0x0512,
        ];
        table.append(&mut tmp);
        for c in 0x0514..=0x0560 {
            table.push(c);
        }
        for c in 0x0531..=0x0556 {
            table.push(c);
        }
        tmp = vec![0xFFFF, 0x17F6, 0x2C63];
        table.append(&mut tmp);
        for c in 0x1D7E..=0x1E00 {
            table.push(c);
        }
        tmp = vec![
            0x1E00, 0x1E02, 0x1E02, 0x1E04, 0x1E04, 0x1E06, 0x1E06, 0x1E08, 0x1E08, 0x1E0A, 0x1E0A,
            0x1E0C, 0x1E0C, 0x1E0E, 0x1E0E, 0x1E10, 0x1E10, 0x1E12, 0x1E12, 0x1E14, 0x1E14, 0x1E16,
            0x1E16, 0x1E18, 0x1E18, 0x1E1A, 0x1E1A, 0x1E1C, 0x1E1C, 0x1E1E, 0x1E1E, 0x1E20, 0x1E20,
            0x1E22, 0x1E22, 0x1E24, 0x1E24, 0x1E26, 0x1E26, 0x1E28, 0x1E28, 0x1E2A, 0x1E2A, 0x1E2C,
            0x1E2C, 0x1E2E, 0x1E2E, 0x1E30, 0x1E30, 0x1E32, 0x1E32, 0x1E34, 0x1E34, 0x1E36, 0x1E36,
            0x1E38, 0x1E38, 0x1E3A, 0x1E3A, 0x1E3C, 0x1E3C, 0x1E3E, 0x1E3E, 0x1E40, 0x1E40, 0x1E42,
            0x1E42, 0x1E44, 0x1E44, 0x1E46, 0x1E46, 0x1E48, 0x1E48, 0x1E4A, 0x1E4A, 0x1E4C, 0x1E4C,
            0x1E4E, 0x1E4E, 0x1E50, 0x1E50, 0x1E52, 0x1E52, 0x1E54, 0x1E54, 0x1E56, 0x1E56, 0x1E58,
            0x1E58, 0x1E5A, 0x1E5A, 0x1E5C, 0x1E5C, 0x1E5E, 0x1E5E, 0x1E60, 0x1E60, 0x1E62, 0x1E62,
            0x1E64, 0x1E64, 0x1E66, 0x1E66, 0x1E68, 0x1E68, 0x1E6A, 0x1E6A, 0x1E6C, 0x1E6C, 0x1E6E,
            0x1E6E, 0x1E70, 0x1E70, 0x1E72, 0x1E72, 0x1E74, 0x1E74, 0x1E76, 0x1E76, 0x1E78, 0x1E78,
            0x1E7A, 0x1E7A, 0x1E7C, 0x1E7C, 0x1E7E, 0x1E7E, 0x1E80, 0x1E80, 0x1E82, 0x1E82, 0x1E84,
            0x1E84, 0x1E86, 0x1E86, 0x1E88, 0x1E88, 0x1E8A, 0x1E8A, 0x1E8C, 0x1E8C, 0x1E8E, 0x1E8E,
            0x1E90, 0x1E90, 0x1E92, 0x1E92, 0x1E94, 0x1E94,
        ];
        table.append(&mut tmp);
        for c in 0x1E96..=0x1EA0 {
            table.push(c);
        }
        tmp = vec![
            0x1EA0, 0x1EA2, 0x1EA2, 0x1EA4, 0x1EA4, 0x1EA6, 0x1EA6, 0x1EA8, 0x1EA8, 0x1EAA, 0x1EAA,
            0x1EAC, 0x1EAC, 0x1EAE, 0x1EAE, 0x1EB0, 0x1EB0, 0x1EB2, 0x1EB2, 0x1EB4, 0x1EB4, 0x1EB6,
            0x1EB6, 0x1EB8, 0x1EB8, 0x1EBA, 0x1EBA, 0x1EBC, 0x1EBC, 0x1EBE, 0x1EBE, 0x1EC0, 0x1EC0,
            0x1EC2, 0x1EC2, 0x1EC4, 0x1EC4, 0x1EC6, 0x1EC6, 0x1EC8, 0x1EC8, 0x1ECA, 0x1ECA, 0x1ECC,
            0x1ECC, 0x1ECE, 0x1ECE, 0x1ED0, 0x1ED0, 0x1ED2, 0x1ED2, 0x1ED4, 0x1ED4, 0x1ED6, 0x1ED6,
            0x1ED8, 0x1ED8, 0x1EDA, 0x1EDA, 0x1EDC, 0x1EDC, 0x1EDE, 0x1EDE, 0x1EE0, 0x1EE0, 0x1EE2,
            0x1EE2, 0x1EE4, 0x1EE4, 0x1EE6, 0x1EE6, 0x1EE8, 0x1EE8, 0x1EEA, 0x1EEA, 0x1EEC, 0x1EEC,
            0x1EEE, 0x1EEE, 0x1EF0, 0x1EF0, 0x1EF2, 0x1EF2, 0x1EF4, 0x1EF4, 0x1EF6, 0x1EF6, 0x1EF8,
            0x1EF8,
        ];
        table.append(&mut tmp);
        for c in 0x1EFA..=0x1EFF {
            table.push(c);
        }
        for c in 0x1F08..=0x1F0F {
            table.push(c);
        }
        for c in 0x1F08..=0x1F0F {
            table.push(c);
        }
        for c in 0x1F18..=0x1F1D {
            table.push(c);
        }
        for c in 0x1F16..=0x1F1F {
            table.push(c);
        }
        for c in 0x1F28..=0x1F2F {
            table.push(c);
        }
        for c in 0x1F28..=0x1F2F {
            table.push(c);
        }
        for c in 0x1F38..=0x1F3F {
            table.push(c);
        }
        for c in 0x1F38..=0x1F3F {
            table.push(c);
        }
        for c in 0x1F48..=0x1F4D {
            table.push(c);
        }
        for c in 0x1F46..=0x1F50 {
            table.push(c);
        }
        tmp = vec![0x1F59, 0x1F52, 0x1F5B, 0x1F54, 0x1F5D, 0x1F56, 0x1F5F];
        table.append(&mut tmp);
        for c in 0x1F58..=0x1F5F {
            table.push(c);
        }
        for c in 0x1F68..=0x1F6F {
            table.push(c);
        }
        for c in 0x1F68..=0x1F6F {
            table.push(c);
        }
        for c in 0x1FBA..=0x1FBB {
            table.push(c);
        }
        for c in 0x1FC8..=0x1FCB {
            table.push(c);
        }
        for c in 0x1FDA..=0x1FDB {
            table.push(c);
        }
        for c in 0x1FF8..=0x1FF9 {
            table.push(c);
        }
        for c in 0x1FEA..=0x1FEB {
            table.push(c);
        }
        for c in 0x1FFA..=0x1FFB {
            table.push(c);
        }
        for c in 0x1F7E..=0x1F7F {
            table.push(c);
        }
        for c in 0x1F88..=0x1F8F {
            table.push(c);
        }
        for c in 0x1F88..=0x1F8F {
            table.push(c);
        }
        for c in 0x1F98..=0x1F9F {
            table.push(c);
        }
        for c in 0x1F98..=0x1F9F {
            table.push(c);
        }
        for c in 0x1FA8..=0x1FAF {
            table.push(c);
        }
        for c in 0x1FA8..=0x1FAF {
            table.push(c);
        }
        for c in 0x1FB8..=0x1FB9 {
            table.push(c);
        }
        tmp = vec![0x1FB2, 0x1FBC];
        table.append(&mut tmp);
        for c in 0x1FB4..=0x1FCB {
            table.push(c);
        }
        tmp = vec![0x1FC3];
        table.append(&mut tmp);
        for c in 0x1FCD..=0x1FCF {
            table.push(c);
        }
        for c in 0x1FD8..=0x1FD9 {
            table.push(c);
        }
        for c in 0x1FD2..=0x1FDF {
            table.push(c);
        }
        for c in 0x1FE8..=0x1FE9 {
            table.push(c);
        }
        for c in 0x1FE2..=0x1FE4 {
            table.push(c);
        }
        tmp = vec![0x1FEC];
        table.append(&mut tmp);
        for c in 0x1FE6..=0x1FFB {
            table.push(c);
        }
        tmp = vec![0x1FF3];
        table.append(&mut tmp);
        for c in 0x1FFD..=0x214D {
            table.push(c);
        }
        tmp = vec![0x2132];
        table.append(&mut tmp);
        for c in 0x214F..=0x216F {
            table.push(c);
        }
        for c in 0x2160..=0x216F {
            table.push(c);
        }
        for c in 0x2180..=0x2183 {
            table.push(c);
        }
        tmp = vec![0x2183, 0xFFFF, 0x034B];
        table.append(&mut tmp);
        for c in 0x24B6..=0x24CF {
            table.push(c);
        }
        tmp = vec![0xFFFF, 0x0746];
        table.append(&mut tmp);
        for c in 0x2C00..=0x2C2E {
            table.push(c);
        }
        for c in 0x2C5F..=0x2C60 {
            table.push(c);
        }
        tmp = vec![0x2C60];
        table.append(&mut tmp);
        for c in 0x2C62..=0x2C67 {
            table.push(c);
        }
        tmp = vec![0x2C67, 0x2C69, 0x2C69, 0x2C6B, 0x2C6B];
        table.append(&mut tmp);
        for c in 0x2C6D..=0x2C75 {
            table.push(c);
        }
        tmp = vec![0x2C75];
        table.append(&mut tmp);
        for c in 0x2C77..=0x2C80 {
            table.push(c);
        }
        tmp = vec![
            0x2C80, 0x2C82, 0x2C82, 0x2C84, 0x2C84, 0x2C86, 0x2C86, 0x2C88, 0x2C88, 0x2C8A, 0x2C8A,
            0x2C8C, 0x2C8C, 0x2C8E, 0x2C8E, 0x2C90, 0x2C90, 0x2C92, 0x2C92, 0x2C94, 0x2C94, 0x2C96,
            0x2C96, 0x2C98, 0x2C98, 0x2C9A, 0x2C9A, 0x2C9C, 0x2C9C, 0x2C9E, 0x2C9E, 0x2CA0, 0x2CA0,
            0x2CA2, 0x2CA2, 0x2CA4, 0x2CA4, 0x2CA6, 0x2CA6, 0x2CA8, 0x2CA8, 0x2CAA, 0x2CAA, 0x2CAC,
            0x2CAC, 0x2CAE, 0x2CAE, 0x2CB0, 0x2CB0, 0x2CB2, 0x2CB2, 0x2CB4, 0x2CB4, 0x2CB6, 0x2CB6,
            0x2CB8, 0x2CB8, 0x2CBA, 0x2CBA, 0x2CBC, 0x2CBC, 0x2CBE, 0x2CBE, 0x2CC0, 0x2CC0, 0x2CC2,
            0x2CC2, 0x2CC4, 0x2CC4, 0x2CC6, 0x2CC6, 0x2CC8, 0x2CC8, 0x2CCA, 0x2CCA, 0x2CCC, 0x2CCC,
            0x2CCE, 0x2CCE, 0x2CD0, 0x2CD0, 0x2CD2, 0x2CD2, 0x2CD4, 0x2CD4, 0x2CD6, 0x2CD6, 0x2CD8,
            0x2CD8, 0x2CDA, 0x2CDA, 0x2CDC, 0x2CDC, 0x2CDE, 0x2CDE, 0x2CE0, 0x2CE0, 0x2CE2, 0x2CE2,
        ];
        table.append(&mut tmp);
        for c in 0x2CE4..=0x2CFF {
            table.push(c);
        }
        for c in 0x10A0..=0x10C5 {
            table.push(c);
        }
        tmp = vec![0xFFFF, 0xD21B];
        table.append(&mut tmp);
        for c in 0xFF21..=0xFF3A {
            table.push(c);
        }
        for c in 0xFF5B..=0xFFFF {
            table.push(c);
        }

        assert_eq!(table.len(), 2918);
        let mut array = [0; 2918];
        for (i, c) in table.iter().enumerate() {
            array[i] = *c;
        }
        Self(array)
    }
}

/// 计算文件名的哈希值
#[derive(Default, Debug, Clone)]
pub struct FileNameHash(pub u16);

impl FileNameHash {
    pub fn add_chars(&mut self, up_case_table: &UpCaseTable, unicode_string: &UnicodeString) {
        let up_case_bytes = up_case_table.to_upper(unicode_string);

        for i in 0..up_case_bytes.len() {
            self.0 = ((self.0 & 1) << 15) + (self.0 >> 1) + up_case_bytes[i] as u16;
        }
    }
}