# embedded-error-chain

***This library is currently very WIP.***

A rust library implementing easy error handling for embedded devices. An `Error` value is
only a single `u32` in size and supports up to 4 chained error codes. Each error code can
have a value from `0` to `15` (4 bits). All error codes come from an enum that implements
the trait `ErrorCategory`. This trait is also used to implement debug printing and
equality for each error code.

This library was inspired by [`error-chain`](https://crates.io/crates/error-chain) and
[`anyhow`](https://crates.io/crates/anyhow), though its goal is to work in `no_std` and
`no_alloc` environments with very little memory overhead.