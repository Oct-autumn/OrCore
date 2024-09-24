//! user/src/bin/02power.rs
//! 实验：系统运算测试

#![no_std] //Delete std-lib, use rust-core-lib
#![no_main] //Remove main() func

#[macro_use]
extern crate user_lib;

const LEN: usize = 100;

static mut S: [u64; LEN] = [0u64; LEN];

#[no_mangle]
unsafe fn main() -> i32 {
    let p = 3u64;
    let m = 998244353u64;
    let iter: usize = 300000;
    let mut cur = 0usize;
    S[cur] = 1;
    for i in 1..=iter {
        let next = if cur + 1 == LEN { 0 } else { cur + 1 };
        S[next] = S[cur] * p % m;
        cur = next;
        if i % 10000 == 0 {
            println!("power_3 [{}/{}]", i, iter);
        }
    }
    println!("{}^{} = {}(MOD {})", p, iter, S[cur], m);
    println!("Test power_3 OK!");
    0
}
