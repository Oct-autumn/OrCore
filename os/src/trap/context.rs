use log::trace;
use riscv::register::sstatus::{self, Sstatus, SPP};

/// 中断上下文，用于保存寄存器和CSR
#[repr(C)]
pub struct TrapContext {
    /// 通用寄存器x0-x31
    pub x: [usize; 32],
    /// CSR sstatus
    pub sstatus: Sstatus,
    /// CSR sepc
    pub sepc: usize,
}

impl TrapContext {
    /// 设置栈指针至`x[2]`处（因为`x0`、`tp(x4)`这两个寄存器不需要保存 ，所以不使用他们的存储空间）
    pub fn set_sp(&mut self, sp: usize) {
        self.x[2] = sp;
    }

    /// 构造函数，初始化应用程序的中断上下文
    pub fn app_init_context(entry: usize, sp: usize) -> Self {
        trace!("app_init_context: entry = {:#x}, sp = {:#x}", entry, sp);
        let sstatus = sstatus::read(); // CSR sstatus
        unsafe {
            sstatus::set_spp(SPP::User); // 设置sstatus的SPP位为1，表示当前运行在用户态
        }
        // 若使用`https://github.com/rcore-os/riscv`作为riscv依赖库，则需使用以下代码：
        //let mut sstatus = sstatus::read(); // CSR sstatus
        //sstatus.set_spp(SPP::User); // 设置sstatus的SPP位为1，表示当前运行在用户态
        let mut cx = Self {
            x: [0; 32],
            sstatus,
            sepc: entry, // 应用程序的入口地址
        };
        cx.set_sp(sp); // 设置应用程序的栈指针
        cx // 返回初始化的TrapContext
    }
}
