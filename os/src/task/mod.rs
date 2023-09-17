//! Implementation of TaskManager

mod context;

#[allow(clippy::module_inception)]
mod task;

use crate::config::MAX_APP_NUM;
use crate::loader::{get_num_app, init_app_cx};
use crate::sync::UPSafeCell;
use lazy_static::*;
use task::{TaskControlBlock, TaskStatus};
pub use context::TaskContext;

/// TaskManager, where all the tasks are managed.
///
/// Why use UPSafeCell to wrap TaskManagerInner?
/// Because we want to impl `Sync` trait for it.
pub struct TaskManager {
    num_app: usize,
    inner: UPSafeCell<TaskManagerInner>,
}

pub struct TaskManagerInner {
    tasks: [TaskControlBlock; MAX_APP_NUM],
    current_task: usize,
}

lazy_static! {
    pub static ref TASK_MANAGER: TaskManager = {
        let num_app = get_num_app();
        let mut tasks = [TaskControlBlock {
            task_cx: TaskContext::zero_init(),
            task_status: TaskStatus::UnInit,
        }; MAX_APP_NUM];
        for (i, task) in tasks.iter_mut().enumerate() {
            task.task_cx = TaskContext::goto_restore(init_app_cx(i));
            task.task_status = TaskStatus::Ready;
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
    
    fn mark_current_exited(&self) -> {
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
            .find(|id| inner.tasks[id].task_status == TaskStatus::Ready)
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
        if let Some(next) = self.find_next_task() {
           
           // Update tasks status of TaskManager
           let mut inner = self.inner.exclusive_access();
           let current = inner.current_task;
           inner.tasks[current].task_status = TaskStatus::Running;
           inner.current_task = next;
           
           // Implement task switch
           let current_task_cx_ptr =
               &mut inner.tasks[current].task_cx as *mut TaskContext;
           let next_task_cx_ptr =
               &inner.tasks[current].task_cx as *const TaskContext;
           drop(inner);
           unsafe {
               __switch(current_task_cx_ptr, next_task_cx_ptr);
           }
        } else { // All tasks completed
            println!("All applications completed!");
            
            #[cfg(feature = "board_qemu")]
            use crate::board::QEMUExit;
            #[cfg(feature = "board_qemu")]
            crate::board::QEMU_EXIT_HANDLE.exit_success();
        }
    }
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