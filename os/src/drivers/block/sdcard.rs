//! SD卡驱动
//!
//! TODO: 实现DMA支持

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(unused)]

use super::BlockDevice;
use crate::println;
use crate::sync::RwLock;
use core::convert::TryInto;
use k210_hal::prelude::*;
use k210_pac::{Peripherals, SPI0};
use k210_soc::{
    fpioa::{self, io},
    //dmac::{dma_channel, DMAC, DMACExt},
    gpio,
    gpiohs,
    sleep::usleep,
    spi::{aitm, frame_format, tmod, work_mode, SPIExt, SPIImpl, SPI},
    sysctl,
};
use lazy_static::*;
use log::{error, trace};
use crate::util::time::sleep;

/// 封装 SD卡 使用SPI通信
pub struct SDCard<SPI> {
    spi: SPI,
    spi_cs: u32,    // 片选信号
    cs_gpionum: u8, // 片选引脚
    //dmac: &'a DMAC,
    //channel: dma_channel,
}

/*
 * Start Data tokens:
 *         Tokens (necessary because at nop/idle (and CS active) only 0xff is
 *         on the data/command line)
 */

/// 数据帧起始，用于开始单块读取
pub const SD_START_DATA_SINGLE_BLOCK_READ: u8 = 0xFE;
/// 数据帧起始，用于开始多块读取
pub const SD_START_DATA_MULTIPLE_BLOCK_READ: u8 = 0xFE;
/// 数据帧起始，用于开始单块写入
pub const SD_START_DATA_SINGLE_BLOCK_WRITE: u8 = 0xFE;
/// 数据帧起始，用于开始多块写入
pub const SD_START_DATA_MULTIPLE_BLOCK_WRITE: u8 = 0xFC;

/// 块大小：512Byte
pub const SECTOR_SIZE: usize = 512;

/** SD卡控制命令 */
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
#[allow(unused)]
pub enum CMD {
    /// 软件重置
    SOFTWARE_REST = 0,
    /// 检查电压范围
    CHECK_OCR = 8,
    /// 读取CSD寄存器
    READ_CSD_REG = 9,
    /// 读取CID寄存器
    READ_CID_REG = 10,
    /// 停止读取数据
    STOP_READING = 12,
    /// 更改读写块大小
    CMD16 = 16,
    /// 读取块
    READ_SINGLE_BLOCK = 17,
    /// 读取多个块
    READ_MULTIPLE_BLOCK = 18,
    /// 擦除块数量
    PRE_EREASE_BLOCKS = 23,
    /// 写入块
    WRITE_SINGLE_BLOCK = 24,
    /// 写入多个块
    WRITE_MULTIPLE_BLOCK = 25,
    /// 启动初始化进程
    ACMD_INIT = 41,
    /// ACMD*的前导命令
    ACMD_FOREHEAD = 55,
    /// 读取OCR
    READ_OCR = 58,
    /// 启用/禁用CRC检查
    CMD59 = 59,
}

/// 初始化错误
#[allow(unused)]
#[derive(Debug, Copy, Clone)]
pub enum InitError {
    /// 命令执行失败
    CMDFailed(CMD, u8),
    /// 无法获取卡容量状态
    CardCapacityStatusNotSet([u8; 4]),
    /// 无法获取卡信息
    CannotGetCardInfo,
}


/// Card Specific Data: CSD Register
/// CSD寄存器，存储了SD卡的特定数据
#[derive(Debug, Copy, Clone)]
pub struct SDCardCSD {
    /// CSD结构
    pub CSDStruct: u8,
    /// 系统规范版本
    pub SysSpecVersion: u8,
    /// 保留
    pub Reserved1: u8,
    /// 数据读取访问时间1
    pub TAAC: u8,
    /// 数据读取访问时间2
    pub NSAC: u8,
    /// 最大总线时钟频率
    pub MaxBusClkFrec: u8,
    /// 卡命令类
    pub CardComdClasses: u16,
    /// 读块长度
    pub RdBlockLen: u8,
    /// 允许部分块读取
    pub PartBlockRead: u8,
    /// 写块不对齐
    pub WrBlockMisalign: u8,
    /// 读块不对齐
    pub RdBlockMisalign: u8,
    /// DSR实现
    pub DSRImpl: u8,
    /// 保留
    pub Reserved2: u8,
    /// 设备大小
    pub DeviceSize: u32,
    //MaxRdCurrentVDDMin: u8,   /* Max. read current @ VDD min */
    //MaxRdCurrentVDDMax: u8,   /* Max. read current @ VDD max */
    //MaxWrCurrentVDDMin: u8,   /* Max. write current @ VDD min */
    //MaxWrCurrentVDDMax: u8,   /* Max. write current @ VDD max */
    //DeviceSizeMul: u8,        /* Device size multiplier */
    /// 擦除组大小
    pub EraseGrSize: u8,
    /// 擦除组大小乘数
    pub EraseGrMul: u8,
    /// 写保护组大小
    pub WrProtectGrSize: u8,
    /// 写保护组启用
    pub WrProtectGrEnable: u8,
    /// 制造商默认ECC
    pub ManDeflECC: u8,
    /// 写速度因数
    pub WrSpeedFact: u8,
    /// 最大写块长度
    pub MaxWrBlockLen: u8,
    /// 允许部分块写入
    pub WriteBlockPaPartial: u8,
    /// 保留
    pub Reserved3: u8,
    /// 内容保护应用
    pub ContentProtectAppli: u8,
    /// 文件格式组
    pub FileFormatGroup: u8,
    /// 复制标志
    pub CopyFlag: u8,
    /// 永久写保护
    pub PermWrProtect: u8,
    /// 临时写保护
    pub TempWrProtect: u8,
    /// 文件格式
    pub FileFormat: u8,
    /// ECC码
    pub ECC: u8,
    /// CSD CRC校验码
    pub CSD_CRC: u8,
    /// 保留
    pub Reserved4: u8,
}

/// Card Identification Data: CID Register
///
/// CID寄存器，存储了SD卡的识别数据
#[derive(Debug, Copy, Clone)]
pub struct SDCardCID {
    /// 制造商ID
    pub ManufacturerID: u8,
    /// OEM/Application ID
    pub OEM_AppliID: u16,
    /// 产品名称 part1
    pub ProdName1: u32,
    /// 产品名称 part2
    pub ProdName2: u8,
    /// 产品版本
    pub ProdRev: u8,
    /// 产品序列号
    pub ProdSN: u32,
    /// 保留
    pub Reserved1: u8,
    /// 制造日期
    pub ManufactDate: u16,
    /// CID CRC校验码
    pub CID_CRC: u8,
    /// 保留
    pub Reserved2: u8,
}

/// SD卡信息
#[derive(Debug, Copy, Clone)]
pub struct SDCardInfo {
    /// CSD寄存器
    pub csd: SDCardCSD,
    /// CID寄存器
    pub cid: SDCardCID,
    /// 卡容量（单位：字节）
    pub card_capacity: u64,
    /// 块大小（单位：字节）
    pub card_block_size: u64,
}

impl<X: SPI> SDCard<X> {
    pub fn new(
        spi: X,
        spi_cs: u32,
        cs_gpionum: u8,
        // 用于DMA
        //dmac: &'a DMAC,
        //channel: dma_channel
    ) -> Self {
        Self {
            spi,
            spi_cs,
            cs_gpionum,
            //dmac,
            //channel,
        }
    }

    /// 拉高片选信号（高电平无效）
    fn CS_HIGH(&self) {
        gpiohs::set_pin(self.cs_gpionum, true);
    }

    /// 拉低片选信号（低电平有效）
    fn CS_LOW(&self) {
        gpiohs::set_pin(self.cs_gpionum, false);
    }

    /// 启用高速SPI
    fn HIGH_SPEED_ENABLE(&self) {
        // 设置SPI时钟速率为10MHz
        self.spi.set_clk_rate(10000000);
    }

    /// 低级初始化
    fn lowlevel_init(&self) {
        // 设置片选GPIO引脚为输出模式
        gpiohs::set_direction(self.cs_gpionum, gpio::direction::OUTPUT);
        // 设置SPI时钟速率0.2MHz
        self.spi.set_clk_rate(200000);
    }

    /// 向SPI写数据
    fn write_data(&self, data: &[u8]) {
        self.spi.configure(
            work_mode::MODE0,
            frame_format::STANDARD,
            8, /* data bits */
            0, /* endian */
            0, /*instruction length*/
            0, /*address length*/
            0, /*wait cycles*/
            aitm::STANDARD,
            tmod::TRANS,
        );
        self.spi.send_data(self.spi_cs, data);
    }

    /*
    fn write_data_dma(&self, data: &[u32]) {
        self.spi.configure(
            work_mode::MODE0,
            frame_format::STANDARD,
            8, /* data bits */
            0, /* endian */
            0, /*instruction length*/
            0, /*address length*/
            0, /*wait cycles*/
            aitm::STANDARD,
            tmod::TRANS,
        );
        self.spi
            .send_data_dma(self.dmac, self.channel, self.spi_cs, data);
    }
     */

    /// 从SPI读数据
    fn read_data(&self, data: &mut [u8]) {
        self.spi.configure(
            work_mode::MODE0,
            frame_format::STANDARD,
            8, /* data bits */
            0, /* endian */
            0, /*instruction length*/
            0, /*address length*/
            0, /*wait cycles*/
            aitm::STANDARD,
            tmod::RECV,
        );
        self.spi.recv_data(self.spi_cs, data);
    }

    /*
    fn read_data_dma(&self, data: &mut [u32]) {
        self.spi.configure(
            work_mode::MODE0,
            frame_format::STANDARD,
            8, /* data bits */
            0, /* endian */
            0, /*instruction length*/
            0, /*address length*/
            0, /*wait cycles*/
            aitm::STANDARD,
            tmod::RECV,
        );
        self.spi
            .recv_data_dma(self.dmac, self.channel, self.spi_cs, data);
    }
     */

    /// 向SD卡发送命令
    /// # param
    ///  - cmd: 命令
    ///  - arg: 命令参数
    ///  - crc: CRC校验码
    /// # retval
    ///  None
    fn send_cmd(&self, cmd: CMD, arg: u32, crc: u8) {
        // 拉低片选信号
        self.CS_LOW();
        // 发送命令
        self.write_data(&[
            /* Construct byte 1 */
            ((cmd as u8) | 0x40),
            /* Construct byte 2 */
            (arg >> 24) as u8,
            /* Construct byte 3 */
            ((arg >> 16) & 0xff) as u8,
            /* Construct byte 4 */
            ((arg >> 8) & 0xff) as u8,
            /* Construct byte 5 */
            (arg & 0xff) as u8,
            /* Construct CRC: byte 6 */
            crc,
        ]);
    }

    /// 获取SD卡响应
    /// # param
    /// None
    /// # retval
    /// 来自SD卡的响应
    ///    - `0xFF`: 未收到响应
    ///    - `Other`: 命令执行成功
    fn get_response(&self) -> u8 {
        let result = &mut [0u8];
        let mut timeout = 0x0FFF;
        /* Check if response is got or a timeout is happen */
        while timeout != 0 {
            self.read_data(result);
            /* Right response got */
            if result[0] != 0xFF {
                return result[0];
            }
            timeout -= 1;
        }
        /* After time out */
        0xFF
    }

    /// 发送命令并获取响应
    fn send_cmd_and_get_response(&self, cmd: CMD, arg: u32, crc: u8) -> u8 {
        self.send_cmd(cmd, arg, crc);
        self.get_response()
    }

    /// 结束命令
    /// # param
    ///  None
    /// # retval
    ///  None
    fn end_cmd(&self) {
        // 拉高片选信号
        self.CS_HIGH();
        // 发送一个空字节
        self.write_data(&[0xff]);
    }

    /// 获取来自SD卡的数据响应
    /// # param
    /// None
    /// # retval
    /// SD卡状态: 读取数据响应 xxx0<status>1
    ///    - status 010: 数据接收成功
    ///    - status 101: 因为CRC错误而拒绝数据
    ///    - status 110: 因为写入错误而拒绝数据
    ///    - status 111: 因为未知原因而拒绝数据
    fn get_data_response(&self) -> u8 {
        let response = &mut [0u8];
        // 读取数据响应
        self.read_data(response);
        // 取低5位
        response[0] &= 0x1F;
        if response[0] != 0x05 {
            // 读取数据响应失败
            return 0xFF;
        }
        /* Wait null data */
        self.read_data(response);
        while response[0] == 0 {
            self.read_data(response);
        }
        /* Return response */
        response[0]
    }

    /// 读取CSD寄存器（此操作等同于一个读块事务）
    /// # param
    /// None
    /// # retval
    /// - `Err()`: 发生错误
    /// - `Ok(SDCardCSD)`: 读取成功
    ///
    /// TODO: CSD有两个版本，需要根据版本号解析，目前只支持了CSD1.0
    fn get_csd_register(&self) -> Result<SDCardCSD, ()> {
        let mut csd_tab = [0u8; 18];
        // 发送CMD9（读取CSD寄存器）        
        if self.send_cmd_and_get_response(CMD::READ_CSD_REG, 0, 0) != 0x00 {
            self.end_cmd();
            return Err(());
        }
        
        // 等待数据块读取开始
        if self.get_response() != SD_START_DATA_SINGLE_BLOCK_READ {
            self.end_cmd();
            return Err(());
        }
        // 将CSD寄存器值存储到csd_tab
        self.read_data(&mut csd_tab);
        // 结束命令
        self.end_cmd();

        Ok(SDCardCSD {
            /* Byte 0 */
            CSDStruct: (csd_tab[0] & 0xC0) >> 6,
            SysSpecVersion: (csd_tab[0] & 0x3C) >> 2,
            Reserved1: csd_tab[0] & 0x03,
            /* Byte 1 */
            TAAC: csd_tab[1],
            /* Byte 2 */
            NSAC: csd_tab[2],
            /* Byte 3 */
            MaxBusClkFrec: csd_tab[3],
            /* Byte 4, 5 */
            CardComdClasses: (u16::from(csd_tab[4]) << 4) | ((u16::from(csd_tab[5]) & 0xF0) >> 4),
            /* Byte 5 */
            RdBlockLen: csd_tab[5] & 0x0F,
            /* Byte 6 */
            PartBlockRead: (csd_tab[6] & 0x80) >> 7,
            WrBlockMisalign: (csd_tab[6] & 0x40) >> 6,
            RdBlockMisalign: (csd_tab[6] & 0x20) >> 5,
            DSRImpl: (csd_tab[6] & 0x10) >> 4,
            Reserved2: 0,
            // DeviceSize: (csd_tab[6] & 0x03) << 10,
            /* Byte 7, 8, 9 */
            DeviceSize: ((u32::from(csd_tab[7]) & 0x3F) << 16)
                | (u32::from(csd_tab[8]) << 8)
                | u32::from(csd_tab[9]),
            /* Byte 10 */
            EraseGrSize: (csd_tab[10] & 0x40) >> 6,
            /* Byte 10, 11 */
            EraseGrMul: ((csd_tab[10] & 0x3F) << 1) | ((csd_tab[11] & 0x80) >> 7),
            /* Byte 11 */
            WrProtectGrSize: (csd_tab[11] & 0x7F),
            /* Byte 12 */
            WrProtectGrEnable: (csd_tab[12] & 0x80) >> 7,
            ManDeflECC: (csd_tab[12] & 0x60) >> 5,
            WrSpeedFact: (csd_tab[12] & 0x1C) >> 2,
            /* Byte 12,13 */
            MaxWrBlockLen: ((csd_tab[12] & 0x03) << 2) | ((csd_tab[13] & 0xC0) >> 6),
            /* Byte 13 */
            WriteBlockPaPartial: (csd_tab[13] & 0x20) >> 5,
            Reserved3: 0,
            ContentProtectAppli: (csd_tab[13] & 0x01),
            /* Byte 14 */
            FileFormatGroup: (csd_tab[14] & 0x80) >> 7,
            CopyFlag: (csd_tab[14] & 0x40) >> 6,
            PermWrProtect: (csd_tab[14] & 0x20) >> 5,
            TempWrProtect: (csd_tab[14] & 0x10) >> 4,
            FileFormat: (csd_tab[14] & 0x0C) >> 2,
            ECC: (csd_tab[14] & 0x03),
            /* Byte 15 */
            CSD_CRC: (csd_tab[15] & 0xFE) >> 1,
            Reserved4: 1,
            /* Return the reponse */
        })
    }


    /// 读取CID寄存器（此操作等同于一个读块事务）
    /// # param
    /// None
    /// # retval
    /// - `Err()`: 发生错误
    /// - `Ok(SDCardCID)`: 读取成功
    pub fn get_cid_register(&self) -> Result<SDCardCID, ()> {
        let mut cid_tab = [0u8; 18];
        // 发送CMD10（读取CID寄存器）        
        if self.send_cmd_and_get_response(CMD::READ_CID_REG, 0, 0) != 0x00 {
            self.end_cmd();
            return Err(());
        }
        
        // 等待数据块读取开始
        if self.get_response() != SD_START_DATA_SINGLE_BLOCK_READ {
            self.end_cmd();
            return Err(());
        }
        // 将CID寄存器值存储到cid_tab
        self.read_data(&mut cid_tab);
        // 结束命令
        self.end_cmd();

        Ok(SDCardCID {
            /* Byte 0 */
            ManufacturerID: cid_tab[0],
            /* Byte 1, 2 */
            OEM_AppliID: (u16::from(cid_tab[1]) << 8) | u16::from(cid_tab[2]),
            /* Byte 3, 4, 5, 6 */
            ProdName1: (u32::from(cid_tab[3]) << 24)
                | (u32::from(cid_tab[4]) << 16)
                | (u32::from(cid_tab[5]) << 8)
                | u32::from(cid_tab[6]),
            /* Byte 7 */
            ProdName2: cid_tab[7],
            /* Byte 8 */
            ProdRev: cid_tab[8],
            /* Byte 9, 10, 11, 12 */
            ProdSN: (u32::from(cid_tab[9]) << 24)
                | (u32::from(cid_tab[10]) << 16)
                | (u32::from(cid_tab[11]) << 8)
                | u32::from(cid_tab[12]),
            /* Byte 13, 14 */
            Reserved1: (cid_tab[13] & 0xF0) >> 4,
            ManufactDate: ((u16::from(cid_tab[13]) & 0x0F) << 8) | u16::from(cid_tab[14]),
            /* Byte 15 */
            CID_CRC: (cid_tab[15] & 0xFE) >> 1,
            Reserved2: 1,
        })
    }

    /// 获取SD卡信息
    ///
    /// # param
    /// None
    /// # retval
    /// - `Err(())`: 发生错误
    /// - `Ok(SDCardInfo)`: 读取成功
    fn get_cardinfo(&self) -> Result<SDCardInfo, ()> {
        let mut info = SDCardInfo {
            csd: self.get_csd_register()?,
            cid: self.get_cid_register()?,
            card_capacity: 0,
            card_block_size: 0,
        };
        info.card_block_size = 1 << u64::from(info.csd.RdBlockLen);
        info.card_capacity = (u64::from(info.csd.DeviceSize) + 1) * 1024 * info.card_block_size;

        Ok(info)
    }

    /// 初始化SD卡通信&SD卡
    /// # param
    /// None
    /// # retval
    /// - `Err(InitError)`: 初始化失败
    /// - `Ok(SDCardInfo)`: 初始化成功
    pub fn init(&self) -> Result<SDCardInfo, InitError> {
        trace!("Using low level SPI to init SD Card...");
        // 配置片选引脚与低速SPI
        self.lowlevel_init();

        trace!("Let SD Card enter SPI mode...");
        // 要让SD卡进入SPI模式，需要发送至少74个时钟周期的 CS&MOSI 的高电平
        // 拉高片选信号
        self.CS_HIGH();
        // 为了确保SD卡进入SPI模式，发送10个0xFF
        self.write_data(&[0xff; 10]);

        trace!("Sending Software Reset Command...");
        // 发送CMD0（软件重置）
        // 这并不总是有效，如果SD访问在操作中断开，将会失败
        let result = self.send_cmd_and_get_response(CMD::SOFTWARE_REST, 0, 0x95);
        self.end_cmd();
        if result != 0x01 {
            return Err(InitError::CMDFailed(CMD::SOFTWARE_REST, result));
        }

        trace!("Checking OCR...");
        // 发送CMD8（检查操作电压范围）
        let result = self.send_cmd_and_get_response(CMD::CHECK_OCR, 0x01AA, 0x87);
        /* 0x01 or 0x05 */
        let mut frame = [0u8; 4];
        self.read_data(&mut frame);
        self.end_cmd();
        if result != 0x01 {
            error!("CMD8(CHECK_OCR) failed: {:x?}, this may be an unsupported SD Card", result);
            return Err(InitError::CMDFailed(CMD::CHECK_OCR, result));
        }

        trace!("Start ACMD init...");
        // 启动ACMD初始化进程
        let mut index = 255;
        while index != 0 {
            // 发送CMD55（ACMD前导命令）
            let result = self.send_cmd_and_get_response(CMD::ACMD_FOREHEAD, 0, 0);
            self.end_cmd();
            if result != 0x01 {
                return Err(InitError::CMDFailed(CMD::ACMD_FOREHEAD, result));
            }

            // 发送ACMD41（启动初始化进程）
            let result = self.send_cmd_and_get_response(CMD::ACMD_INIT, 0x40000000, 0);
            self.end_cmd();
            if result == 0x00 {
                break;
            }
            index -= 1;
            trace!("Waiting for ACMD init...{}", index);
            sleep(1000);
        }
        if index == 0 {
            return Err(InitError::CMDFailed(CMD::ACMD_INIT, result));
        }

        trace!("Further check of OCR...");
        // 进一步检查OCR
        index = 255;
        let mut frame = [0u8; 4];
        while index != 0 {
            // 发送CMD58（读取OCR）
            let result = self.send_cmd_and_get_response(CMD::READ_OCR, 0, 1);
            self.read_data(&mut frame);
            self.end_cmd();
            if result == 0 {
                break;
            }
            index -= 1;
            trace!("Waiting for READ_OCR resp...{}", index);
            sleep(1000);
        }
        if index == 0 {
            return Err(InitError::CMDFailed(CMD::READ_OCR, result));
        }
        if (frame[0] & 0x40) == 0 {
            return Err(InitError::CardCapacityStatusNotSet(frame));
        }
        trace!("Enabling High Speed SPI...");
        // 启用高速SPI
        self.HIGH_SPEED_ENABLE();

        trace!("Getting Card Info...");
        self.get_cardinfo()
            .map_err(|_| InitError::CannotGetCardInfo)
    }

    /// 从SD卡读取一个块
    /// # param
    /// - `data_buf`: 用于存储读取数据的缓冲区
    /// - `sector`: 读取的扇区地址
    /// # retval
    /// - `Err(())`: Sequence failed
    /// - `Ok(())`: Sequence succeed
    pub fn read_sector(&self, data_buf: &mut [u8], sector: u32) -> Result<(), ()> {
        // 保证data_buf的大小是SECTOR_SIZE的正整数倍
        assert!(data_buf.len() >= SECTOR_SIZE && (data_buf.len() % SECTOR_SIZE) == 0);

        // 根据data_buf的大小选择读取方式
        let multi_block_reading = if data_buf.len() == SECTOR_SIZE {
            // 读取单块
            self.send_cmd(CMD::READ_SINGLE_BLOCK, sector, 0);
            false
        } else {
            // 读取多块
            self.send_cmd(CMD::READ_MULTIPLE_BLOCK, sector, 0);
            true
        };

        // 检查SD卡响应，0x00表示无错误
        if self.get_response() != 0x00 {
            self.end_cmd();
            return Err(());
        }

        // 开始读取数据
        // 流程控制标志：读取时发生错误
        let mut error = false;
        // DMA缓冲区
        // let mut dma_chunk = [0u32; SEC_LEN];
        // 临时缓冲区
        let mut tmp_chunk = [0u8; SECTOR_SIZE];
        // 按扇区大小读取数据
        for chunk in data_buf.chunks_mut(SECTOR_SIZE) {
            // 检查数据块读取开始
            if self.get_response() != SD_START_DATA_SINGLE_BLOCK_READ {
                error = true;
                break;
            }
            //// 从DMA缓冲区读取数据
            //self.read_data_dma(&mut dma_chunk);
            // 读取数据到临时缓冲区
            self.read_data(&mut tmp_chunk);
            // 将数据写入data_buf
            for (a, b) in chunk.iter_mut().zip(/*dma_chunk*/ tmp_chunk.iter()) {
                //// 将u32转换为u8
                //*a = (b & 0xff) as u8;
                *a = *b;
            }
            // 获取CRC校验码（忽略）
            let mut frame = [0u8; 2];
            self.read_data(&mut frame);
        }
        self.end_cmd();

        // 若读取多块，发送停止读取命令
        if multi_block_reading {
            self.send_cmd(CMD::STOP_READING, 0, 0);
            self.get_response();
            self.end_cmd();
            self.end_cmd();
        }

        if error {
            // 读取过程中发生错误，读取失败
            Err(())
        } else {
            // 读取成功
            Ok(())
        }
    }

    /// 向SD卡写入一个块
    /// # param
    /// - `data_buf`: 待写入数据的缓冲区
    /// - `sector`: 写入的扇区地址
    /// # retval
    /// - `Err(())`: Sequence failed
    /// - `Ok(())`: Sequence succeed
    pub fn write_sector(&self, data_buf: &[u8], sector: u32) -> Result<(), ()> {
        // 保证data_buf的大小是SECTOR_SIZE的正整数倍
        assert!(data_buf.len() >= SECTOR_SIZE && (data_buf.len() % SECTOR_SIZE) == 0);

        // 数据帧起始
        let mut frame = [0xff, 0x00];
        // 根据data_buf的大小选择写入方式
        if data_buf.len() == SECTOR_SIZE {
            // 写入单块
            frame[1] = SD_START_DATA_SINGLE_BLOCK_WRITE;
            self.send_cmd(CMD::WRITE_SINGLE_BLOCK, sector, 0);
        } else {
            // 写入多块
            frame[1] = SD_START_DATA_MULTIPLE_BLOCK_WRITE;
            // 预擦除指令
            self.send_cmd(CMD::ACMD_FOREHEAD, 0, 0);
            self.get_response();
            self.send_cmd(
                CMD::PRE_EREASE_BLOCKS,
                (data_buf.len() / SECTOR_SIZE) as u32,
                0,
            );
            self.get_response();
            self.end_cmd();
            self.send_cmd(CMD::WRITE_MULTIPLE_BLOCK, sector, 0);
        }

        // 检查SD卡响应，0x00表示无错误
        if self.get_response() != 0x00 {
            self.end_cmd();
            return Err(());
        }
        // DMA缓冲区
        //let mut dma_chunk = [0u32; SEC_LEN];
        // 临时缓冲区
        let mut tmp_chunk = [0u8; SECTOR_SIZE];
        // 按扇区大小写入数据
        for chunk in data_buf.chunks(SECTOR_SIZE) {
            // 发送数据帧前导，数据帧起始
            self.write_data(&frame);
            // 将数据写入临时缓冲区
            for (a, &b) in /*dma_chunk*/ tmp_chunk.iter_mut().zip(chunk.iter()) {
                //*a = b.into();
                *a = b;
            }
            // 发送DMA缓冲区数据
            //self.write_data_dma(&mut dma_chunk);
            // 发送临时缓冲区数据
            self.write_data(&tmp_chunk);
            // 发送CRC校验码（忽略）
            self.write_data(&[0xff, 0xff]);
            // 获取数据响应
            if self.get_data_response() != 0x00 {
                self.end_cmd();
                return Err(());
            }
        }
        self.end_cmd();
        self.end_cmd();
        Ok(())
    }
}

/// SD卡片选引脚编号（与k210开发板对应）
const SD_CS_GPIONUM: u8 = 7;
/// 给SPI控制器传递的CS值，这是一个虚拟值，因为SPI0_CS3没有映射到FPIOA中的任何内容
const SD_CS: u32 = 3;

/// 映射IO引脚与SPI0
fn io_init() {
    fpioa::set_function(io::SPI0_SCLK, fpioa::function::SPI0_SCLK);
    fpioa::set_function(io::SPI0_MOSI, fpioa::function::SPI0_D0);
    fpioa::set_function(io::SPI0_MISO, fpioa::function::SPI0_D1);
    fpioa::set_function(io::SPI0_CS0, fpioa::function::gpiohs(SD_CS_GPIONUM));
    fpioa::set_io_pull(io::SPI0_CS0, fpioa::pull::DOWN); // GPIO output=pull down
}

lazy_static! {
    /// 全局变量，用于存储外设
    static ref PERIPHERALS: RwLock<Peripherals> =
        RwLock::new(Peripherals::take().unwrap());
}

fn init_sdcard() -> SDCard<SPIImpl<SPI0>> {
    trace!("Sleeping for a while...");
    // 睡眠一段时间，等待外设初始化完成
    sleep(1000);
    // TODO: 修改使用全局变量PERIPHERALS（将Peripherals的实例使用Mutex包装，实现互斥使用）
    //   当前的实现是直接使用全局变量，在有多个外设时，可能会出现冲突
    trace!("Setting PLL...");
    let peripherals = unsafe { Peripherals::steal() };
    sysctl::pll_set_freq(sysctl::pll::PLL0, 800_000_000).unwrap();
    sysctl::pll_set_freq(sysctl::pll::PLL1, 300_000_000).unwrap();
    sysctl::pll_set_freq(sysctl::pll::PLL2, 45_158_400).unwrap();
    let clocks = k210_hal::clock::Clocks::new();
    // 因为前面设置了PLL0时钟，所以这里需要重新配置UART，否则会出现波特率错误
    peripherals.UARTHS.configure(115_200.bps(), &clocks);

    // 初始化IO引脚映射
    trace!("Initializing IO Pin...");
    io_init();

    let spi = peripherals.SPI0.constrain();
    let sd = SDCard::new(spi, SD_CS, SD_CS_GPIONUM);
    trace!("Initializing SD Card...");
    let info = sd.init().unwrap();
    let num_sectors = info.card_capacity / 512;
    assert!(num_sectors > 0);

    trace!("SD Card Initialized: Capacity={}MB, BlockSize={}B, NumBlocks={}", info.card_capacity >> 20, info.card_block_size, num_sectors);
    sd
}

pub struct SDCardWrapper(RwLock<SDCard<SPIImpl<SPI0>>>);

impl SDCardWrapper {
    pub fn new() -> Self {
        Self(RwLock::new(init_sdcard()))
    }
}

impl BlockDevice for SDCardWrapper {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        self.0
            .read()
            .read_sector(buf, block_id as u32)
            .unwrap();
    }
    fn write_block(&self, block_id: usize, buf: &[u8]) {
        self.0
            .write()
            .write_sector(buf, block_id as u32)
            .unwrap();
    }

    fn num_blocks(&self) -> u64 {
        self.0.read().get_cardinfo().unwrap().card_capacity / 512
    }
}