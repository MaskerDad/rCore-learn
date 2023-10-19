//! Implementation of TaskManager
mod context;
mod switch;
mod pid;
mod manager;
mod processor;
#[allow(clippy::module_inception)]
mod task;

use crate::sbi::shutdown;
use crate::loader::{get_num_app, get_app_data};
use crate::sync::UPSafeCell;
use crate::trap::TrapContext;
use lazy_static::*;
use task::{TaskControlBlock, TaskStatus};
use switch::__switch;
pub use context::TaskContext;

use alloc::vec::Vec;

pub use pid::{pid_alloc, KernelStack, PidAllocator, PidHandle};
pub use manager::{add_task, fetch_task, TaskManager};

pub fn suspend_current_and_run_next() {
    //TODO
}

pub fn exit_current_and_run_next() {
    //TODO
}
