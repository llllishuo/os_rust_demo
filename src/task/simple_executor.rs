use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

use alloc::collections::vec_deque::VecDeque;

use super::Task;

pub struct SimpleExecutor {
    task_queue: VecDeque<Task>,
}

impl SimpleExecutor {
    pub fn new() -> SimpleExecutor {
        SimpleExecutor {
            task_queue: VecDeque::new(),
        }
    }
    pub fn spawn(&mut self, task: Task) {
        self.task_queue.push_back(task);
    }
    pub fn run(&mut self) {
        while let Some(mut task) = self.task_queue.pop_front() {
            let waker = dumm_waker();
            let mut context = Context::from_waker(&waker);
            match task.poll(&mut context) {
                Poll::Ready(()) => {}
                Poll::Pending => self.task_queue.push_back(task),
            }
        }
    }
}

fn dumm_raw_waker() -> RawWaker {
    fn no_op(_: *const ()) {}
    fn clone(_: *const ()) -> RawWaker {
        dumm_raw_waker()
    }

    let vtable = &RawWakerVTable::new(clone, no_op, no_op, no_op);
    RawWaker::new(0 as *const (), vtable)
}
fn dumm_waker() -> Waker {
    unsafe { unsafe { Waker::from_raw(dumm_raw_waker()) } }
}
