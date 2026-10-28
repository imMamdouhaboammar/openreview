---
name: rust-best-practices
description: Review Rust code for idiomatic style, memory safety, error handling, Tokio concurrency, and zero-cost abstractions.
---

# Rust Best Practices

## Error Handling & Contracts
- Avoid `unwrap()` and `expect()` in production request paths; use `Result<T, E>` and `?` operator.
- Prefer `thiserror` for library / module errors and `anyhow` for top-level application orchestration.
- Implement `From` or use `#[from]` for clean error conversions.

## Tokio & Concurrency
- Never block the Tokio async runtime with CPU-heavy loops or synchronous file/network I/O; use `tokio::task::spawn_blocking` when synchronous operations are unavoidable.
- Set explicit timeouts using `tokio::time::timeout` on external calls (databases, HTTP, sub-processes).
- Gracefully handle cancellation and dropped futures.

## Memory & Performance
- Avoid unnecessary cloning; pass references (`&str`, `&[T]`) when ownership is not required.
- Use `Arc<T>` and `RwLock<T>` / `Mutex<T>` only where concurrent shared access is strictly needed.
- Enforce clean boundary validation at network boundaries; trust internal types.
