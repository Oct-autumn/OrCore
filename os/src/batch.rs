//! os/src/batch.rs <br>
//! The batch system

use core::arch::asm;

use lazy_static::lazy_static;
use log::*;

use crate::sync::UPSafeCell;
use crate::trap::TrapContext;

// 设置用户栈大小、内核栈大小、最大App数量、App基地址、App大小限制
const USER_STACK_SIZE: usize = 4096 * 2;
const KERNEL_STACK_SIZE: usize = 4096 * 2;
const MAX_APP_NUM: usize = 16;
const APP_BASE_ADDRESS: usize = 0x80400000;
const APP_SIZE_LIMIT: usize = 0x20000;

// 定义并初始化内核栈和用户栈
#[repr(align(4096))]
struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE],
}

impl KernelStack {
    // 获取内核栈的栈顶指针
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + KERNEL_STACK_SIZE
    }

    // 将TrapContext压入内核栈
    pub fn push_context(&self, cx: TrapContext) -> &'static mut TrapContext {
        let cx_ptr = (self.get_sp() - core::mem::size_of::<TrapContext>()) as *mut TrapContext;
        unsafe {
            *cx_ptr = cx;
        }
        unsafe { cx_ptr.as_mut().unwrap() }
    }
}

#[repr(align(4096))]
struct UserStack {
    data: [u8; USER_STACK_SIZE],
}

impl UserStack {
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + USER_STACK_SIZE
    }
}

#[link_section = ".bss.kernel_stack"] // 将内核栈放在.bss.kernel_stack段
static KERNEL_STACK: KernelStack = KernelStack {
    data: [0; KERNEL_STACK_SIZE],
};
#[link_section = ".bss.user_stack"] // 将用户栈放在.bss.user_stack段
static USER_STACK: UserStack = UserStack {
    data: [0; USER_STACK_SIZE],
};

struct AppManager {
    num_app: usize,
    // App数量
    current_app: usize,
    // 当前运行的App
    app_start: [usize; MAX_APP_NUM + 1], // 各App代码的起始点
}

// 运行时初始化
lazy_static! {
    static ref APP_MANAGER: UPSafeCell<AppManager> = unsafe {
        trace!("Initializing APP_MANAGER...");
        UPSafeCell::new({
            // 获取App数量
            extern "C" {
                fn _num_app();
            }
            let num_app_ptr = _num_app as usize as *const usize;
            let num_app = num_app_ptr.read_volatile();

            // 设置程序开始标志
            let mut app_start: [usize; MAX_APP_NUM + 1] = [0; MAX_APP_NUM + 1];
            let app_start_raw: &[usize] = core::slice::from_raw_parts(
                num_app_ptr.add(1),
                num_app + 1,
            );

            // 将各App代码的起始位置拷贝至数组app_start中
            app_start[..=num_app].copy_from_slice(app_start_raw);

            AppManager {
                num_app,
                current_app: 0,
                app_start,
            }
        })
    };
}

impl AppManager {
    pub fn print_app_info(&self) {
        debug!("num_app = {}", self.num_app);
        for i in 0..self.num_app {
            debug!(
                "address app_{} [{:#x}, {:#x})",
                i,
                self.app_start[i],
                self.app_start[i + 1]
            );
        }
    }

    unsafe fn load_app(&self, app_id: usize) {
        // 检查执行序列是否已完成
        if app_id >= self.num_app {
            panic!("[AppManager] All applications completed!");
        }
        info!("Loading app_{}", app_id);
        // 清空App可能会使用到的内存空间
        trace!("Cleaning App memory...");
        core::slice::from_raw_parts_mut(APP_BASE_ADDRESS as *mut u8, APP_SIZE_LIMIT).fill(0);
        // 将对应的App代码拷贝至APP_BASE_ADDRESS处
        trace!("Copying app code...");
        let app_src = core::slice::from_raw_parts(
            self.app_start[app_id] as *const u8,
            self.app_start[app_id + 1] - self.app_start[app_id],
        );
        if app_src.len() > APP_SIZE_LIMIT {
            // App大小超过限制，报错并退出执行
            error!("App size exceeds limit!");
            return;
        }
        let app_dst = core::slice::from_raw_parts_mut(APP_BASE_ADDRESS as *mut u8, app_src.len());
        app_dst.copy_from_slice(app_src);
        // 刷新指令缓存
        asm!("fence.i");
    }

    pub fn get_current_app_id(&self) -> usize {
        self.current_app
    }

    pub fn move_to_next_app(&mut self) {
        self.current_app += 1
    }
}

/// 初始化batch系统
pub fn init() {
    print_app_info();
}

/// 输出App信息
pub fn print_app_info() {
    // 由于使用了UPSafeCell封装，因此不需要使用unsafe
    APP_MANAGER.exclusive_access().print_app_info();
}

/// 运行下一个App
pub fn run_next_app() -> ! {
    trace!("Going to run next app...");
    // 此时APP_MANAGER应当已经初始化完成
    let mut app_manager = APP_MANAGER.exclusive_access();
    let current_app = app_manager.get_current_app_id();
    trace!("current_app = {}", current_app);
    unsafe {
        app_manager.load_app(current_app);
    }
    app_manager.move_to_next_app();
    drop(app_manager);  // 释放mut引用
    extern "C" {
        fn __restore(cx_addr: usize);
    }
    trace!("Jumping to app_{}...", current_app);
    unsafe {
        __restore(KERNEL_STACK.push_context(TrapContext::app_init_context(
            APP_BASE_ADDRESS,
            USER_STACK.get_sp(),
        )) as *const _ as usize);
    }
    panic!("Unreachable in batch::run_current_app!");
}
