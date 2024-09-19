//! os/src/batch.rs <br>
//! The batch system

use lazy_static::lazy_static;
use log::*;

use crate::config::{
    APP_BASE_ADDRESS, APP_SIZE_LIMIT, KERNEL_STACK_SIZE, MAX_APP_NUM, USER_STACK_SIZE,
};
use crate::loader::load_apps;
use crate::sync::UPSafeCell;
use crate::trap::TrapContext;

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
    // App数量
    num_app: usize,
    // 当前运行的App
    current_app: usize,
    // 各App代码在内存中的起始点位置
    app_start: [usize; MAX_APP_NUM + 1],
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

            // 初始化App代码的起始位置
            let mut app_start: [usize; MAX_APP_NUM + 1] = [0; MAX_APP_NUM + 1];

            app_start[0] = APP_BASE_ADDRESS;
            for i in 1..=num_app {
                app_start[i] = app_start[i - 1] + APP_SIZE_LIMIT;
            }

            load_apps();

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

    pub fn get_current_app_id(&self) -> usize {
        self.current_app
    }

    pub fn get_current_app_start(&self) -> usize {
        self.app_start[self.current_app]
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
    let current_app_start = app_manager.get_current_app_start();
    if current_app >= app_manager.num_app {
        panic!("No more apps to run!");
    }
    trace!("current_app = {}", current_app);
    // 所有应用程序已被加载进内存中，因此这里不再需要load_app
    app_manager.move_to_next_app();
    drop(app_manager); // 释放mut引用
    extern "C" {
        fn __restore(cx_addr: usize);
    }
    trace!("Jumping to app_{}...", current_app);

    unsafe {
        __restore(KERNEL_STACK.push_context(TrapContext::app_init_context(
            current_app_start,
            USER_STACK.get_sp(),
        )) as *const _ as usize);
        // 该操作将恢复上下文，跳转到用户态，执行用户程序，不会返回
    }
    panic!("Unreachable in batch::run_current_app!");
}
