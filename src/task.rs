
use crate::cpu::*;
use crate::context::*;
use core::ptr;


pub type Priority = i8;

// Task control block
// static初期化の為に泣く泣くすべてpubにする
pub struct Task {
    pub context: Context,
    pub queue: *mut TaskQueue,
    pub next: *mut Task,
    pub priority: Priority,
    pub task: Option<fn(isize)>,
    pub exinf: isize,
}

// static初期化時に中身知らなくてよいようにマクロで補助
#[macro_export]
macro_rules! task_default {
    () => {
        Task {
            context: context_default!(),
            queue: core::ptr::null_mut(),
            next: core::ptr::null_mut(),
            priority: 0,
            task: None,
            exinf: 0,
        }
    };
}

// タスクキュー
pub struct TaskQueue {
    tail: *mut Task,
}

static mut CURRENT_TASK: *mut Task = ptr::null_mut();
static mut READY_QUEUE: TaskQueue = TaskQueue {
    tail: ptr::null_mut(),
};

/*
fn current_task() -> & mut Task {
    unsafe { &mut *CURRENT_TASK }
}

fn task_eq(task0: &Task, task1: &Task) -> bool {
    let ptr0 = task0 as *const Task;
    let ptr1 = task1 as *const Task;
    ptr0 == ptr1
}
*/

pub (in crate) unsafe fn task_switch() {
    let head = READY_QUEUE.front();
    match head {
        None => {
            CURRENT_TASK = ptr::null_mut();
            context_switch_to_system();
        }
        Some(task) => {
            task.switch();
        }
    };
}

impl Task {
    /// タスク生成
    pub fn create(
        &mut self,
        exinf: isize,
        task: fn(isize),
        priority: Priority,
        stack: &mut [isize],
    ) {
        extern "C" fn task_entry(exinf: isize) {
            let task_ptr = exinf as *mut Task;
            let task = unsafe { &mut *task_ptr };
            (task.task.unwrap())(task.exinf);
            task.remove_queue();
            unsafe { task_switch() };
        }

        self.exinf = exinf;
        self.task = Some(task);
        self.priority = priority;

        let task_ptr = self as *mut Task;
        self.context.create(stack, task_entry, task_ptr as isize);
    }

    /*
    fn is_current(&self) -> bool {
        self.context.is_current()
    }

    fn is_eq(&self, task: &Task) -> bool {
        let ptr0 = self as *const Task;
        let ptr1 = task as *const Task;
        ptr0 == ptr1
    }

    fn get_next(&mut self) -> &mut Task {
        unsafe { &mut *self.next }
    }

    fn set_next(&mut self, task: &mut Task ) {
        self.next = task as *mut Task;
    }
    */

    /// タスクスイッチ
    unsafe fn switch(&mut self) {
        let task_ptr = self as *mut Task;
        CURRENT_TASK = task_ptr;
        //      CURRENT_TASK = self as *mut Task;
        self.context.switch();
    }

    pub fn get_priority(&self) -> Priority {
        self.priority
    }

    pub fn activate(&mut self) {
        unsafe {
            cpu_lock();
            READY_QUEUE.insert_priority_order(self);
            task_switch();
            cpu_unlock();
        }
    }

    fn remove_queue(&mut self) {
        if self.queue != ptr::null_mut() {
            let que = unsafe { &mut *self.queue };
            que.remove(self);
        }
    }
}

impl TaskQueue {
    pub fn new() -> Self {
        TaskQueue {
            tail: ptr::null_mut(),
        }
    }

    /// 優先度順で追加
    pub fn insert_priority_order(&mut self, task: &mut Task) {
        // タスクに所属キューを設定
        task.queue = self as *mut TaskQueue;

        // 生ポインタ化
        let task_ptr: *mut Task = task as *mut Task;

        if self.tail == ptr::null_mut() {
            // キューにタスクが無ければ先頭に設定
            task.next = task_ptr;
            self.tail = task_ptr;
        } else {
            // キューが空でないなら挿入位置を探索
            // タスク優先度を取得
            let task_pri = task.priority;

            // 先頭から探索
            let mut prev_ptr = self.tail;
            let mut prev_task = unsafe { &mut *prev_ptr };
            let mut next_ptr = prev_task.next;
            let mut next_task = unsafe { &mut *next_ptr };
            loop {
                // 優先度取り出し
                let next_pri = next_task.priority;

                if next_pri > task_pri {
                    break;
                }

                // 次を探す
                prev_ptr = next_ptr;
                prev_task = next_task;
                next_ptr = prev_task.next;
                next_task = unsafe { &mut *next_ptr };

                // 末尾なら抜ける
                if prev_ptr == self.tail {
                    self.tail = task as *mut Task;
                    break;
                }
            }

            // 挿入
            prev_task.next = task as *mut Task;
            task.next = next_ptr;
        }
    }

    /// FIFO順で追加
    pub fn push_back(&mut self, task: &mut Task) {
        // 生ポインタ化
        let task_ptr = task as *mut Task;

        // タスクに所属キューを設定
        task.queue = self as *mut TaskQueue;

        if self.tail == ptr::null_mut() {
            // キューにタスクが無ければ先頭に設定
            task.next = task_ptr;
        } else
        // キューが空でないなら末尾に追加
        {
            let tail_task = unsafe { &mut *self.tail };
            task.next = tail_task.next;
            tail_task.next = task_ptr;
        }
        self.tail = task_ptr;
    }

    /// 先頭を参照
    pub fn front(&mut self) -> Option<&mut Task> {
        if self.tail == ptr::null_mut() {
            None
        } else {
            let task_tail = unsafe { &mut *self.tail };
            Some(unsafe { &mut *task_tail.next })
        }
    }

    /// 先頭を取り出し
    pub fn pop_front(&mut self) -> Option<&mut Task> {
        if self.tail == ptr::null_mut() {
            None
        } else {
            let task_tail = unsafe { &mut *self.tail };
            let task_head = unsafe { &mut *task_tail.next };
            if self.tail == task_tail.next {
                self.tail = ptr::null_mut();
            } else {
                task_tail.next = task_head.next;
            }
            Some(task_head)
        }
    }

    // 接続位置で時間が変わるので注意
    // 先頭しか外さない or タスク数を制約するなどで時間保証可能
    // 双方向リストする手はあるので、大量タスクを扱うケースが出たら考える
    pub fn remove(&mut self, task: &mut Task) {
        // 生ポインタ化
        let task_ptr = task as *mut Task;

        // 接続位置を探索
        if task.next == task_ptr {
            /* last one */
            self.tail = ptr::null_mut();
        } else {
            let mut prev_ptr = self.tail;
            let mut prev_task = unsafe { &mut *prev_ptr };
            while prev_task.next != task_ptr {
                prev_ptr = prev_task.next;
                prev_task = unsafe { &mut *prev_ptr };
            }
            prev_task.next = task.next;
            if self.tail == task_ptr {
                self.tail = prev_ptr;
            }
        }
        // 取り外し
        task.queue = ptr::null_mut();
        task.next = ptr::null_mut();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_queue() {
        unsafe {
            static mut QUE: TaskQueue = TaskQueue {
                tail: ptr::null_mut(),
            };
            static mut STACK0: [isize; 256] = [0; 256];
            static mut STACK1: [isize; 256] = [0; 256];
            static mut STACK2: [isize; 256] = [0; 256];
            static mut TASK0: Task = task_default!();
            static mut TASK1: Task = task_default!();
            static mut TASK2: Task = task_default!();
            TASK0.create(0, task0, 0, &mut STACK0);
            TASK1.create(1, task1, 1, &mut STACK1);
            TASK2.create(2, task2, 2, &mut STACK2);

            {
                // 単純追加＆取り出し
                QUE.push_back(&mut TASK0);
                QUE.push_back(&mut TASK1);
                QUE.push_back(&mut TASK2);
                let t0 = QUE.pop_front();
                let t1 = QUE.pop_front();
                let t2 = QUE.pop_front();
                let t3 = QUE.pop_front();
                assert_eq!(t0.unwrap().priority, 0);
                assert_eq!(t1.unwrap().priority, 1);
                assert_eq!(t2.unwrap().priority, 2);
                assert_eq!(t3.is_some(), false);
            }

            {
                // 削除パターン1
                QUE.push_back(&mut TASK0);
                QUE.push_back(&mut TASK1);
                assert_eq!(QUE.tail, &mut TASK1 as *mut Task);
                TASK0.remove_queue();
                assert_eq!(QUE.tail, &mut TASK1 as *mut Task);
                TASK1.remove_queue();
                assert_eq!(QUE.tail, ptr::null_mut());

                let t0 = QUE.pop_front();
                assert_eq!(t0.is_some(), false);
            }

            {
                // 削除パターン2
                QUE.push_back(&mut TASK0);
                QUE.push_back(&mut TASK1);
                assert_eq!(QUE.tail, &mut TASK1 as *mut Task);
                TASK1.remove_queue();
                assert_eq!(QUE.tail, &mut TASK0 as *mut Task);
                TASK0.remove_queue();
                assert_eq!(QUE.tail, ptr::null_mut());

                let t0 = QUE.pop_front();
                assert_eq!(t0.is_some(), false);
            }

            {
                // 優先度順パターン1
                QUE.insert_priority_order(&mut TASK0);
                assert_eq!(QUE.front().unwrap().get_priority(), 0);
                QUE.insert_priority_order(&mut TASK1);
                assert_eq!(QUE.front().unwrap().get_priority(), 0);
                QUE.insert_priority_order(&mut TASK2);
                assert_eq!(QUE.front().unwrap().get_priority(), 0);

                let t0 = QUE.pop_front();
                let t1 = QUE.pop_front();
                let t2 = QUE.pop_front();
                let t3 = QUE.pop_front();
                assert_eq!(t0.unwrap().priority, 0);
                assert_eq!(t1.unwrap().priority, 1);
                assert_eq!(t2.unwrap().priority, 2);
                assert_eq!(t3.is_some(), false);
            }

            {
                // 優先度順パターン2
                QUE.insert_priority_order(&mut TASK2);
                assert_eq!(QUE.front().unwrap().get_priority(), 2);
                QUE.insert_priority_order(&mut TASK1);
                assert_eq!(QUE.front().unwrap().get_priority(), 1);
                QUE.insert_priority_order(&mut TASK0);
                assert_eq!(QUE.front().unwrap().get_priority(), 0);

                let t0 = QUE.pop_front();
                let t1 = QUE.pop_front();
                let t2 = QUE.pop_front();
                let t3 = QUE.pop_front();
                assert_eq!(t0.unwrap().priority, 0);
                assert_eq!(t1.unwrap().priority, 1);
                assert_eq!(t2.unwrap().priority, 2);
                assert_eq!(t3.is_some(), false);
            }
            {
                // 優先度順パターン3
                QUE.insert_priority_order(&mut TASK1);
                assert_eq!(QUE.front().unwrap().get_priority(), 1);
                QUE.insert_priority_order(&mut TASK2);
                assert_eq!(QUE.front().unwrap().get_priority(), 1);
                QUE.insert_priority_order(&mut TASK0);
                assert_eq!(QUE.front().unwrap().get_priority(), 0);

                let t0 = QUE.pop_front();
                let t1 = QUE.pop_front();
                let t2 = QUE.pop_front();
                let t3 = QUE.pop_front();
                assert_eq!(t0.unwrap().priority, 0);
                assert_eq!(t1.unwrap().priority, 1);
                assert_eq!(t2.unwrap().priority, 2);
                assert_eq!(t3.is_some(), false);
            }
            {
                // 優先度順パターン4
                QUE.insert_priority_order(&mut TASK2);
                assert_eq!(QUE.front().unwrap().get_priority(), 2);
                QUE.insert_priority_order(&mut TASK0);
                assert_eq!(QUE.front().unwrap().get_priority(), 0);
                QUE.insert_priority_order(&mut TASK1);
                assert_eq!(QUE.front().unwrap().get_priority(), 0);

                let t0 = QUE.pop_front();
                let t1 = QUE.pop_front();
                let t2 = QUE.pop_front();
                let t3 = QUE.pop_front();
                assert_eq!(t0.unwrap().priority, 0);
                assert_eq!(t1.unwrap().priority, 1);
                assert_eq!(t2.unwrap().priority, 2);
                assert_eq!(t3.is_some(), false);
            }
        }
    }

    fn task0(_exinf: isize) {}
    fn task1(_exinf: isize) {}
    fn task2(_exinf: isize) {}
}
