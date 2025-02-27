use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};

pub struct FromResult<T>(pub T);

#[allow(non_upper_case_globals)]
pub const CompletedFuture: FromResult<()> = FromResult(());

impl<T> Future for FromResult<T>
where
    T: Copy,
{
    type Output = T;

    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.0)
    }
}

pub struct YieldFuture(bool);

pub fn yield_now() -> YieldFuture {
    YieldFuture(false)
}

#[macro_export]
macro_rules! yield_return {
    () => {
        threading::yield_now().await;
    };
}

impl Future for YieldFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.0 {
            true => Poll::Ready(()),
            false => {
                self.0 = true;
                ctx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }
}

pub fn block_run<T>(future: &mut T) -> <T as Future>::Output
where
    T: Future,
{
    let mut future = unsafe { Pin::new_unchecked(future) };
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);

    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => return value,
            Poll::Pending => continue,
        }
    }
}

#[macro_export]
macro_rules! block_on {
    () => {
        ()
    };
    ($future:expr) => {
        block_run(&mut core::future::join!($future))
    };
    ($future:expr, $($futures:expr),+) => {
        block_run(&mut core::future::join!($future, $($futures),+))
    };
}
