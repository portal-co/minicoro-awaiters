# minicoro-awaiters

[![Crates.io](https://img.shields.io/crates/v/minicoro-awaiters.svg)](https://crates.io/crates/minicoro-awaiters)
[![License: MPL 2.0](https://img.shields.io/badge/License-MPL%202.0-brightgreen.svg)](https://opensource.org/licenses/MPL-2.0)

This crate provides integration between Rust's async/await system and the [minicoro](https://crates.io/crates/minicoroutine) coroutine library.

It allows you to await Rust futures from within a minicoro coroutine, bridging the gap between stackful coroutines and Rust's async ecosystem.

## Features

- **`no_std` compatible** - Works in embedded and bare-metal environments (requires `alloc`)
- **Seamless async/await integration** - Await any Rust future from within a coroutine
- **Works with `awaiter-trait`** - Integrates with the [awaiter-trait](https://crates.io/crates/awaiter-trait) ecosystem

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
minicoro-awaiters = "0.1"
```

## Overview

The crate provides three main types:

| Type | Description |
|------|-------------|
| `CoroutineAwaiter` | An awaiter that can be used inside a coroutine to await futures |
| `CoroutineFuture` | A future wrapper around a coroutine that can be awaited from async code |
| `CoroutineToken` | A token type implementing `awaiter_trait::Coroutine` for ergonomic usage |

For backwards compatibility, the old single-letter type aliases `R`, `C`, and `Token` are still available but deprecated.

## Usage

### Basic Usage with `CoroutineFuture`

Create a coroutine that can await futures and run it from async code:

```rust
use minicoro_awaiters::CoroutineFuture;

async fn example() {
    // Create a coroutine that can await futures
    let coro = CoroutineFuture::new(|awaiter| {
        // Inside the coroutine, use the awaiter to await futures
        let result = awaiter.r#await(Box::pin(async { 42 }));
        println!("Got: {}", result);
    });

    // Await the coroutine from async code
    coro.await;
}
```

### Using the `CoroutineToken` API

For a more ergonomic interface, use the `CoroutineToken` type with `awaiter_trait::Coroutine`:

```rust
use minicoro_awaiters::CoroutineToken;
use awaiter_trait::Coroutine;

async fn example() {
    // Execute code in a coroutine and get the result
    let result = CoroutineToken.exec(|awaiter| {
        awaiter.r#await(Box::pin(async { 42 }))
    }).await;
    
    assert_eq!(result, 42);
}
```

## How It Works

The crate bridges Rust's cooperative async/await system with minicoro's stackful coroutines:

1. When you create a `CoroutineFuture` coroutine, it wraps a closure that receives a `CoroutineAwaiter`
2. The `CoroutineAwaiter` implements `awaiter_trait::Awaiter`, allowing you to call `r#await` on futures
3. When a future returns `Pending`, the coroutine yields and saves the waker
4. When the coroutine is polled again (via its `Future` implementation), it resumes and re-polls the inner future
5. This continues until the future completes, at which point the result is returned

## License

This project is licensed under the [Mozilla Public License 2.0](https://opensource.org/licenses/MPL-2.0).

## Goals
- [ ] Add project goals

## Progress
- [ ] Initial setup

---
*AI assisted*
