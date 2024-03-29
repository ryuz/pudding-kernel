#![allow(stable_features)]
#![no_std]

pub type Priority = u32;
pub type RelativeTime = u32;
pub type SystemTime = u64;
pub type ActivateCount = u32;
pub type SemaphoreCount = u32;
pub type FlagPattern = u32;

#[derive(Clone, Copy)]
pub enum Order {
    Priority,
    Fifo,
}

#[derive(Clone, Copy)]
pub enum Error {
    Timeout,
}

pub mod cpu;
pub use cpu::*;

pub mod context;
pub use context::*;

pub mod system;
pub use system::*;

pub mod interrupt;
pub mod irc;

mod priority_queue;
mod timeout_queue;

pub mod task;
pub use task::*;

pub mod semaphore;
pub use semaphore::*;

pub mod eventflag;
pub use eventflag::*;

pub mod time;
pub use time::*;

pub unsafe fn initialize() {
    cpu::cpu_initialize();
    context::context_initialize();
}

// 以下、デバッグ用の暫定

static mut DEBUG_PRINT: Option<fn(str: &str)> = None;

pub fn set_debug_print(fnc: Option<fn(str: &str)>) {
    unsafe {
        DEBUG_PRINT = fnc;
    }
}

pub fn debug_print(str: &str) {
    unsafe {
        match DEBUG_PRINT {
            Some(print) => {
                print(str);
            }
            None => (),
        }
    }
}
