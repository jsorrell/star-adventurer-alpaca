use std::future::Future;
use std::mem;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard};
use std::task::{Context, Poll, Waker};

pub(in crate::util::tasks) struct SharedState<T> {
    pub(in crate::util::tasks) result: Option<T>,
    pub(in crate::util::tasks) wakers: Vec<Waker>,
}

pub struct WaitableTask<T>(pub(in crate::util::tasks) Arc<Mutex<SharedState<T>>>);
pub struct WaitableTaskFinisher<T>(WaitableTask<T>);

impl<T> Clone for WaitableTask<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T> WaitableTask<T> {
    pub fn new() -> (Self, WaitableTaskFinisher<T>) {
        let task = Self(Arc::new(Mutex::new(SharedState {
            result: None,
            wakers: Vec::with_capacity(4),
        })));

        (task.clone(), WaitableTaskFinisher(task))
    }

    pub fn new_completed(result: T) -> Self {
        Self(Arc::new(Mutex::new(SharedState {
            result: Some(result),
            wakers: Vec::with_capacity(0),
        })))
    }

    pub fn is_ready(&self) -> bool {
        let lock = self.0.lock().unwrap();
        lock.result.is_some()
    }

    pub(in crate::util::tasks) fn get_finisher(&self) -> WaitableTaskFinisher<T> {
        WaitableTaskFinisher(self.clone())
    }
}

pub(in crate::util::tasks) fn set_result<U>(mut lock: MutexGuard<SharedState<U>>, result: U) {
    if lock.result.is_some() {
        panic!("Finish called twice");
    }

    lock.result = Some(result);
    let wakers = mem::take(&mut lock.wakers);
    wakers.into_iter().for_each(Waker::wake);
}

impl<T: Clone> Future for WaitableTask<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut lock = self.0.lock().unwrap();
        match &lock.result {
            Some(result) => Poll::Ready(result.clone()),
            None => {
                if !lock.wakers.iter().any(|w| cx.waker().will_wake(w)) {
                    lock.wakers.push(cx.waker().clone());
                }
                Poll::Pending
            }
        }
    }
}

pub trait TaskFinisher<T> {
    fn finish(self, result: T);
}

impl<T> TaskFinisher<T> for WaitableTaskFinisher<T> {
    fn finish(self, result: T) {
        let lock = self.0 .0.lock().unwrap();
        set_result(lock, result)
    }
}
