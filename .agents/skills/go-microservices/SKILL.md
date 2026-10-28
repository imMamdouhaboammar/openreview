---
name: go-microservices
description: Review Go microservice code for idiomatic patterns, goroutine leaks, context propagation, and REST/gRPC best practices.
---

# Go Microservices Best Practices

## Context & Goroutine Hygiene
- Always pass `context.Context` as the first argument in functions performing I/O.
- Respect `ctx.Done()` in select loops to terminate goroutines promptly.
- Never spawn goroutines without a deterministic exit mechanism or `sync.WaitGroup` tracking.

## Error Handling & Typing
- Check errors immediately after every fallible call (`if err != nil`).
- Wrap errors with context: `fmt.Errorf("fetching user %d: %w", id, err)`.
- Use `errors.Is` and `errors.As` instead of raw string equality checks.

## HTTP & API Boundaries
- Read and close HTTP request/response bodies: `defer resp.Body.Close()`.
- Validate untrusted input at ingress boundaries before passing to domain services.
- Never log sensitive tokens, credentials, or personal user data.
