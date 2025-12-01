//! # minicoro-awaiters
//!
//! This crate provides integration between Rust's async/await system and the
//! [`minicoro`](https://crates.io/crates/minicoroutine) coroutine library.
//!
//! It allows you to await Rust futures from within a minicoro coroutine, bridging
//! the gap between stackful coroutines and Rust's async ecosystem.
//!
//! ## Overview
//!
//! The crate provides three main types:
//!
//! - [`CoroutineAwaiter`]: An awaiter that can be used inside a coroutine to await futures
//! - [`CoroutineFuture`]: A future wrapper around a coroutine that can be awaited from async code
//! - [`CoroutineToken`]: A token type implementing `awaiter_trait::Coroutine` for ergonomic usage
//!
//! For backwards compatibility, the old single-letter type aliases [`R`], [`C`], and [`Token`]
//! are still available.
//!
//! ## Example
//!
//! ```ignore
//! use minicoro_awaiters::{CoroutineFuture, CoroutineToken};
//! use awaiter_trait::Coroutine;
//!
//! async fn example() {
//!     // Create a coroutine that can await futures
//!     let coro = CoroutineFuture::new(|awaiter| {
//!         // Inside the coroutine, use the awaiter to await futures
//!         let result = awaiter.r#await(Box::pin(async { 42 }));
//!         assert_eq!(result, 42);
//!     });
//!
//!     // Await the coroutine from async code
//!     coro.await;
//! }
//! ```
//!
//! ## Features
//!
//! - `no_std` compatible (requires `alloc`)
//! - Seamless integration with Rust's async/await
//! - Works with the `awaiter-trait` ecosystem

#![no_std]
extern crate alloc;
use core::future::Future;
use core::mem::MaybeUninit;
use core::task::Context;
use core::task::Poll;

use alloc::boxed::Box;
use atomic_waker::AtomicWaker;
use minicoroutine::Coroutine;
use minicoroutine::CoroutineRef;
use minicoroutine::GLOBAL;

/// An awaiter that allows awaiting futures from within a minicoro coroutine.
///
/// This struct implements [`awaiter_trait::Awaiter`], enabling futures to be
/// awaited in a blocking fashion within a coroutine. When a future is polled
/// and returns `Pending`, the coroutine yields and will be resumed when the
/// future's waker is invoked.
///
/// # Example
///
/// ```ignore
/// use minicoro_awaiters::CoroutineFuture;
///
/// let coro = CoroutineFuture::new(|awaiter| {
///     // The `awaiter` parameter is of type `CoroutineAwaiter`
///     let value = awaiter.r#await(Box::pin(async { "hello" }));
///     println!("Got: {}", value);
/// });
/// ```
pub struct CoroutineAwaiter {
    /// The underlying coroutine reference used for yielding and accessing user data.
    pub coro: CoroutineRef<(), (), (), AtomicWaker, GLOBAL>,
}

/// Type alias for backwards compatibility.
#[deprecated(since = "0.2.0", note = "Use `CoroutineAwaiter` instead")]
pub type R = CoroutineAwaiter;

impl awaiter_trait::Awaiter for CoroutineAwaiter {
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

awaiter_trait::autoimpl!(<> CoroutineAwaiter as Awaiter);

/// A future wrapper around a minicoro coroutine.
///
/// This struct wraps a coroutine and implements [`Future`], allowing the coroutine
/// to be awaited from async code. When polled, it resumes the underlying coroutine
/// and registers a waker to be notified when the coroutine should be resumed.
///
/// # Creating a Coroutine
///
/// Use [`CoroutineFuture::new`] to create a new coroutine with a closure that receives
/// a [`CoroutineAwaiter`] for awaiting futures inside the coroutine.
///
/// # Example
///
/// ```ignore
/// use minicoro_awaiters::CoroutineFuture;
///
/// async fn run() {
///     let coro = CoroutineFuture::new(|awaiter| {
///         // Do work inside the coroutine
///         let result = awaiter.r#await(Box::pin(some_async_fn()));
///     });
///     
///     coro.await; // Run the coroutine to completion
/// }
/// ```
pub struct CoroutineFuture {
    /// The underlying minicoro coroutine.
    pub coro: Coroutine<(), (), (), AtomicWaker, GLOBAL>,
}

/// Type alias for backwards compatibility.
#[deprecated(since = "0.2.0", note = "Use `CoroutineFuture` instead")]
pub type C = CoroutineFuture;

impl CoroutineFuture {
    /// Creates a new coroutine that can await futures.
    ///
    /// The provided closure receives a [`CoroutineAwaiter`] that can be used to
    /// await futures from within the coroutine.
    ///
    /// # Arguments
    ///
    /// * `a` - A closure that will be executed inside the coroutine. It receives
    ///   a [`CoroutineAwaiter`] for awaiting futures.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use minicoro_awaiters::CoroutineFuture;
    ///
    /// let coro = CoroutineFuture::new(|awaiter| {
    ///     let value = awaiter.r#await(Box::pin(async { 42 }));
    ///     println!("Got: {}", value);
    /// });
    /// ```
    pub fn new<T: FnOnce(CoroutineAwaiter)>(a: T) -> Self {
        // let a = MaybeUninit::new(a);
        let a = Box::leak(Box::new(a)) as *mut _ as *mut ();
        Self {
            coro: Coroutine::new(
                move |p| unsafe { *Box::from_raw(a as *mut T) }(CoroutineAwaiter { coro: p }),
                Default::default(),
            )
            .unwrap(),
        }
    }
}

impl Future for CoroutineFuture {
    type Output = ();

    fn poll(mut self: core::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.coro.user_data().register(&cx.waker());
        match self.coro.resume(()) {
            Some(_) => Poll::Pending,
            None => Poll::Ready(()),
        }
    }
}
/// A token type for creating coroutines through the `awaiter_trait::Coroutine` interface.
///
/// This zero-sized type implements [`awaiter_trait::Coroutine`], providing an ergonomic
/// way to create coroutines that can await futures using the `exec` method.
///
/// # Example
///
/// ```ignore
/// use minicoro_awaiters::CoroutineToken;
/// use awaiter_trait::Coroutine;
///
/// async fn example() {
///     let result = CoroutineToken.exec(|awaiter| {
///         awaiter.r#await(Box::pin(async { 42 }))
///     }).await;
///     assert_eq!(result, 42);
/// }
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Default)]
pub struct CoroutineToken;

/// Type alias for backwards compatibility.
#[deprecated(since = "0.2.0", note = "Use `CoroutineToken` instead")]
pub type Token = CoroutineToken;

impl awaiter_trait::Coroutine for CoroutineToken {
    fn exec<T>(
        &self,
        f: impl FnOnce(&(dyn awaiter_trait::r#dyn::DynAwaiter + '_)) -> T,
    ) -> impl Future<Output = T> {
        async move {
            let mut c = MaybeUninit::uninit();
            match &mut c {
                c => {
                    CoroutineFuture::new(move |a| {
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

awaiter_trait::autoimpl!(<> CoroutineToken as Coroutine);
