//! Test stack overflow.

#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

#[allow(unconditional_recursion)]
fn f(depth: usize) {
    if depth % 10 == 0 {
        println!("depth = {}", depth);
    }
    f(depth + 1);
}

#[no_mangle]
pub fn main() -> i32 {
    println!("this program runs a function that will cause stack overflow.");
    println!("It should trigger segmentation fault!");
    f(0);
    0
}
