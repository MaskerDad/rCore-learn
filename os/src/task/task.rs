//! Implementation of TaskControlBlock
use super::TaskContext;
use crate::config::{TRAP_CONTEXT, kernel_stack_position};
use crate::mm::{
    MemorySet, PhysPageNum, KERNEL_SPACE,
    MapPermission, VirtAddr,
};
use crate::trap::{trap_handler, TrapContext};

use super::{pid_alloc, KernelStack, PidHandle};

use crate::sync::UPSafeCell;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use core::cell::RefMut;

#[derive(Copy, Clone, PartialEq)]
/// task status: Ready/Running/Zombie
pub enum TaskStatus {
    Ready,
    Running,
    Zombie,
}

/// task control core structure
pub struct TaskControlBlock {
    // immutable
    pub pid: PidHandle,
    pub kernel_stack: KernelStack,
    // mutable
    inner: UPSafeCell<TaskControlBlockInner>,
}

pub struct TaskControlBlockInner {
    pub task_status: TaskStatus,
    pub task_cx: TaskContext,
    pub trap_cx_ppn: PhysPageNum,
    pub memory_set: MemorySet,
    pub base_size: usize,
    pub parent: Option<Weak<TaskControlBlock>>,
    pub children: Vec<Arc<TaskControlBlock>>,
    pub exit_code: i32,
}

impl TaskControlBlockInner {
    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.get_mut()
    }

    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }

    fn get_status(&self) -> TaskStatus {
        self.task_status
    }

    pub fn is_zombie(&self) -> bool {
        self.get_status() == TaskStatus::Zombie
    }
}

impl TaskControlBlock {
    /// get inner of TaskControlBlock
    pub fn inner_exclusive_access(&self) 
        -> RefMut<'_, TaskControlBlockInner>
    {
        self.inner.exclusive_access()
    }

    pub fn getpid(&self) -> usize {
        self.pid.0
    }

    /// create a new task control block
    pub fn new(elf_data: &[u8]) -> Self {
        // memory_set with elf program headers/trampoline/trap_context/user_stack
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();

        // alloc a pid and a kernel stack in kernel space
        let pid_handle = pid_alloc();
        let kernel_stack = KernelStack::new(&pid_handle);
        let kernel_stack_top = kernel_stack.get_top();
        
        // create task_control_block
        let task_control_block = Self {
            pid: pid_handle,
            kernel_stack,
            inner: unsafe {
              UPSafeCell::new(TaskControlBlockInner {
                  task_status: TaskStatus::Ready,
                  task_cx: TaskContext::goto_trap_return(kernel_stack_top),
                  trap_cx_ppn,
                  memory_set,
                  base_size: user_sp,
                  parent: None,
                  children: Vec::new(),
                  exit_code: 0,
              })  
            },
        };

        // prepare TrapContext in user space
        let trap_cx = task_control_block.inner_exclusive_access().get_trap_cx();
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_top,
            trap_handler as usize,
        );
        task_control_block
    }

    pub fn fork(&self) -> Arc<TaskControlBlock> {
        //TODO
    }

    pub fn exec(&self, elf_data: &[u8]) {
        //TODO
    }
}
