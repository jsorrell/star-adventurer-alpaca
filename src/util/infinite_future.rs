use std::future::Future;
use std::marker;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct InfiniteFuture<T> {
    _data: marker::PhantomData<T>,
}

impl<T> InfiniteFuture<T> {
    pub fn new() -> Self {
        Self {
            _data: marker::PhantomData,
        }
    }
}

impl<T> Future for InfiniteFuture<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::<T>::Pending
    }
}
