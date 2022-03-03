use crate::util::tasks::waitable_task::{set_result, SharedState, TaskFinisher, WaitableTask};
use crate::AbortResult;
use crate::AbortResult::Completed;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
/// Used for awaiting the task completion
pub struct AbortableTask<T, U>(Arc<Mutex<SharedState<AbortResult<T, U>>>>, WaitableTask<()>);

impl<T, U> Clone for AbortableTask<T, U> {
    fn clone(&self) -> Self {
        AbortableTask(Arc::clone(&self.0), self.1.clone())
    }
}

impl<T: Clone, U: Clone> Future for AbortableTask<T, U> {
    type Output = AbortResult<T, U>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut waitable_task = WaitableTask(self.0.clone()); // Coerce into a waitable task
        Pin::new(&mut waitable_task).poll(cx)
    }
}

impl<T, U> AbortableTask<T, U> {
    pub fn new() -> (Self, impl AbortFinisher<T, U>) {
        let (waiter, _) = WaitableTask::new();
        let task = Self::new_with_abort_waiter(waiter);
        (task.clone(), AbortableTaskAbortFinisher(task))
    }

    pub fn new_with_abort_waiter(waiter: WaitableTask<()>) -> Self {
        AbortableTask(
            Arc::new(Mutex::new(SharedState {
                result: None,
                wakers: Vec::with_capacity(4),
            })),
            waiter,
        )
    }

    pub fn new_completed(result: T) -> Self {
        AbortableTask(
            Arc::new(Mutex::new(SharedState {
                result: Some(AbortResult::Completed(result)),
                wakers: Vec::with_capacity(0),
            })),
            WaitableTask::new().0,
        )
    }

    pub fn get_finisher(&self) -> impl AbortFinisher<T, U> {
        AbortableTaskAbortFinisher(self.clone())
    }

    pub fn get_aborter(&self) -> impl TaskAborter<T, U> {
        self.clone()
    }

    pub fn get_abort_waiter(&self) -> WaitableTask<()> {
        self.1.clone()
    }

    pub fn get_waitable_task(&self) -> WaitableTask<AbortResult<T, U>> {
        WaitableTask(Arc::clone(&self.0))
    }
}

impl<T, U> From<AbortableTask<T, U>> for WaitableTask<AbortResult<T, U>> {
    fn from(at: AbortableTask<T, U>) -> Self {
        WaitableTask(at.0)
    }
}

/// Used to abort a running task
pub trait TaskAborter<T, U> {
    fn abort(&self) -> WaitableTask<AbortResult<T, U>>;
}

impl<T, U> TaskAborter<T, U> for AbortableTask<T, U> {
    fn abort(&self) -> WaitableTask<AbortResult<T, U>> {
        let lock = self.0.lock().unwrap();
        if lock.result.is_none() {
            self.1.clone().get_finisher().finish(());
        }
        drop(lock);
        WaitableTask(self.0.clone())
    }
}

/// Used to report that the abort has finished
pub trait AbortFinisher<T, U>: TaskFinisher<T> {
    fn aborted(self, result: U);
}
struct AbortableTaskAbortFinisher<T, U>(AbortableTask<T, U>);

impl<T, U> TaskFinisher<T> for AbortableTaskAbortFinisher<T, U> {
    fn finish(self, result: T) {
        let lock = self.0 .0.lock().unwrap();
        // If finish called after aborted, don't do anything
        if !self.0 .1.is_ready() {
            set_result(lock, Completed(result));
        }
    }
}

impl<T, U> AbortFinisher<T, U> for AbortableTaskAbortFinisher<T, U> {
    fn aborted(self, result: U) {
        let lock = self.0 .0.lock().unwrap();
        set_result(lock, AbortResult::Aborted(result));
    }
}

// #[deriver(Clone)]
// pub struct Finisher<T> {
//     task: WaitableTask<T>,
// }
//
// impl<T: 'static + Clone> WaitableTask<T> {
//     pub fn new() -> (Self, Finisher<T>) {
//         let (finish_tx, _) = broadcast::channel(1);
//         let task = Self {
//             finish_tx: finish_tx.clone(),
//             done: Arc::new(Mutex::new(None)),
//         };
//
//         let finisher = Finisher { task: task.clone() };
//
//         (task, finisher)
//     }
//
//     pub fn instant_finish(v: T) -> Self {
//         let (task, finisher) = Self::new();
//         finisher.finish(v);
//         task
//     }
//
//     pub fn wait(&self) -> impl Future<Output = T> {
//         let lock = self.done.lock().unwrap();
//         let value = lock.clone();
//
//         let mut rx = self.finish_tx.subscribe();
//
//         async move {
//             if value.is_some() {
//                 value.unwrap()
//             } else {
//                 let res = rx.recv().await;
//
//                 match res {
//                     Ok(v) => v,
//                     Err(_) => panic!("Misuse"),
//                 }
//             }
//         }
//     }
//
//     /// Panics if the task has not yet finished
//     pub fn read_result(&self) -> T {
//         self.done
//             .lock()
//             .unwrap()
//             .clone()
//             .expect("read_result called on task that hasn't finished")
//     }
// }
//
// impl<T: 'static + Clone> Finisher<T> {
//     pub fn finish(self, v: T) {
//         let mut finish_lock = self.task.done.lock().unwrap();
//         *finish_lock = Some(v);
//         let _res = self.task.finish_tx.send(finish_lock.clone().unwrap());
//     }
// }
//
// pub struct AbortableTask<T> {
//     waitable_task: WaitableTask<T>,
//     abort_tx: Option<oneshot::Sender<()>>,
// }
//
// pub struct AbortListener {
//     abort_rx: oneshot::Receiver<()>,
// }
//
// impl AbortListener {
//     pub async fn wait(self) {
//         let result = self.abort_rx.await;
//         if matches!(result, Err(_)) {
//             InfiniteFuture::new().await
//         }
//     }
// }
//
// impl<T: 'static + Clone> AbortableTask<T> {
//     pub fn new() -> (Self, Finisher<T>, AbortListener) {
//         let (waitable_task, finisher) = WaitableTask::new();
//         let (abort_tx, abort_rx) = oneshot::channel();
//         (
//             Self {
//                 waitable_task,
//                 abort_tx: Some(abort_tx),
//             },
//             finisher,
//             AbortListener { abort_rx },
//         )
//     }
//
//     pub fn wait(&self) -> impl Future<Output = T> {
//         self.waitable_task.wait()
//     }
//
//     pub fn abort(&mut self) -> impl Future<Output = T> {
//         if self.abort_tx.is_some() {
//             let canceller = mem::replace(&mut self.abort_tx, None);
//             let _res = canceller.unwrap().send(());
//         }
//
//         self.wait()
//     }
//
//     pub fn read_result(&self) -> T {
//         self.waitable_task.read_result()
//     }
// }
