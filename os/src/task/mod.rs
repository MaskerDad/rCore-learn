//! Implementation of TaskManager

mod context;
mod switch;

#[allow(clippy::module_inception)]
mod task;

use crate::config::MAX_APP_NUM;
use crate::loader::{get_num_app, get_app_data};
use crate::sync::UPSafeCell;
use crate::trap::TrapContext;
use lazy_static::*;
use task::{TaskControlBlock, TaskStatus};
use switch::__switch;
pub use context::TaskContext;

use alloc::vec::Vec;

/// TaskManager, where all the tasks are managed.
///
/// Why use UPSafeCell to wrap TaskManagerInner?
/// Because we want to impl `Sync` trait for it.
pub struct TaskManager {
    num_app: usize,
    inner: UPSafeCell<TaskManagerInner>,
}

pub struct TaskManagerInner {
    tasks: Vec<TaskControlBlock>,
    current_task: usize,
}

lazy_static! {
    pub static ref TASK_MANAGER: TaskManager = {
        println!("init TASK_MANAGER!");
        let num_app = get_num_app();
        println!("current num_app = {}", num_app);
        
        let mut tasks: Vec<TaskControlBlock> = Vec::new();
        for i in 0..num_app {
            tasks.push(TaskControlBlock::new(
                    get_app_data(i),
                    i,
            ));
        }

        TaskManager {
            num_app,
            inner: unsafe {
                UPSafeCell::new(TaskManagerInner {
                    tasks,
                    current_task: 0,
                })
            },
        }
    };
}

impl TaskManager {
    fn mark_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Ready;
    }
    
    fn mark_current_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Exited;
    }

    /// Simple task scheduling policy
    /// 
    /// Find next task to run and return task id,
    /// We only return the first `Ready` task in list.
    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        (current + 1..current + self.num_app + 1)
            .map(|id| id % self.num_app)
            .find(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
    }

    /// Run first task in task list.
    fn  run_first_task(&self) -> ! {
        // Mark task0 from UnInit as Running
        let mut inner = self.inner.exclusive_access();
        let task0 = &mut inner.tasks[0];
        task0.task_status = TaskStatus::Running;
       
        // {_Unused/Task0} context switch
        let next_task_cx_ptr = &task0.task_cx as *const TaskContext;
        let mut _unused = TaskContext::zero_init();
        drop(inner);
        unsafe {
            __switch(&mut _unused as *mut TaskContext, next_task_cx_ptr);
        }
        panic!("unreachable in run_first task!");
    }

    /// Core function of task switch
    fn run_next_task(&self) {
        //println!("Enter run_next_task...");
        if let Some(next) = self.find_next_task() {
           
           // Update tasks status of TaskManager
           let mut inner = self.inner.exclusive_access();
           let current = inner.current_task;
           inner.tasks[next].task_status = TaskStatus::Running;
           inner.current_task = next;
           
           // Implement task switch
           let current_task_cx_ptr =
               &mut inner.tasks[current].task_cx as *mut TaskContext;
           let next_task_cx_ptr =
               &inner.tasks[next].task_cx as *const TaskContext;
           drop(inner);
           unsafe {
               __switch(current_task_cx_ptr, next_task_cx_ptr);
           }
           // bbbbbback to user mode!
        } else { // All tasks completed
            println!("All applications completed!");
            
            #[cfg(feature = "board_qemu")]
            use crate::board::QEMUExit;
            #[cfg(feature = "board_qemu")]
            crate::board::QEMU_EXIT_HANDLE.exit_success();
        }
    }

    /// Get the current task's token with the "Running" flag
    fn get_current_token(&self) -> usize {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_user_token()
    }
 
    /// Get the current task's token with the "Running" flag
    fn get_current_trap_cx(&self) -> &'static mut TrapContext {
       let inner = self.inner.exclusive_access();
       inner.tasks[inner.current_task].get_trap_cx()
    }
}

pub fn current_user_token() -> usize {
    TASK_MANAGER.get_current_token()
}

pub fn current_trap_cx() -> &'static mut TrapContext {
    TASK_MANAGER.get_current_trap_cx()
}

pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

fn run_next_task() {
    TASK_MANAGER.run_next_task();
}

fn mark_current_suspended() {
    TASK_MANAGER.mark_current_suspended();
}

fn mark_current_exited() {
    TASK_MANAGER.mark_current_exited();
}

pub fn suspend_current_and_run_next() {
    mark_current_suspended();
    run_next_task();
}

pub fn exit_current_and_run_next() {
    mark_current_exited();
    run_next_task();
}
