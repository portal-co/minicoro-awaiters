#![no_std]
extern crate alloc;
use core::mem::MaybeUninit;
use core::task::Context;
use core::task::Poll;
use core::task::Waker;

use alloc::boxed::Box;
use atomic_waker::AtomicWaker;
use minicoroutine::Coroutine;
use minicoroutine::CoroutineRef;
use minicoroutine::GLOBAL;
pub struct R {
    pub coro: CoroutineRef<(), (), (), AtomicWaker, GLOBAL>,
}
impl awaiter_trait::Awaiter for R {
    fn r#await<T>(&self, mut f: core::pin::Pin<&mut (dyn Future<Output = T> + '_)>) -> T {
        loop {
            let t = loop {
                match self.coro.user_data().take() {
                    Some(a) => break a,
                    None => self.coro.yield_(()),
                }
            };
            match f.as_mut().poll(&mut Context::from_waker(&t)) {
                Poll::Ready(a) => return a,
                Poll::Pending => self.coro.yield_(()),
            }
        }
    }
}
awaiter_trait::autoimpl!(<> R as Awaiter);
pub struct C {
    pub coro: Coroutine<(), (), (), AtomicWaker, GLOBAL>,
}
impl C {
    pub fn new<T: FnOnce(R)>(a: T) -> Self {
        // let a = MaybeUninit::new(a);
        let a = Box::leak(Box::new(a)) as *mut _ as *mut ();
        Self {
            coro: Coroutine::new(
                move |p| unsafe { *Box::from_raw(a as *mut T) }(R { coro: p }),
                Default::default(),
            )
            .unwrap(),
        }
    }
}
impl Future for C {
    type Output = ();

    fn poll(mut self: core::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.coro.user_data().register(&cx.waker());
        match self.coro.resume(()) {
            Some(_) => Poll::Pending,
            None => Poll::Ready(()),
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Default)]
pub struct Token;
impl awaiter_trait::Coroutine for Token {
    fn exec<T>(
        &self,
        f: impl FnOnce(&(dyn awaiter_trait::r#dyn::DynAwaiter + '_)) -> T,
    ) -> impl Future<Output = T> {
        async move {
            let mut c = MaybeUninit::uninit();
            match &mut c {
                c => {
                    C::new(move |a| {
                        let v = f(&a);
                        c.write(v);
                    })
                    .await
                }
            };
            unsafe { c.assume_init() }
        }
    }
}
awaiter_trait::autoimpl!(<> Token as Coroutine);
