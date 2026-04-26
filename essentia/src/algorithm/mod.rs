//! Auto-generated algorithm builders.
//!
//! At build time, [`essentia_codegen`](https://docs.rs/essentia-codegen)
//! walks every algorithm registered with the C++ Essentia runtime and emits
//! a Rust struct for it under `src/algorithm/generated/<category>/<algorithm>.rs`.
//! The [`generated`] sub-module declared below pulls those files into the
//! public API of this crate via ordinary `mod` statements — no
//! `include!`/`OUT_DIR` indirection, so IDEs see every algorithm as plain
//! Rust source.
//!
//! Each generated algorithm struct follows the same shape:
//!
//! ```ignore
//! pub struct Foo<'a, State = Initialized> { /* ... */ }
//!
//! impl<'a> Foo<'a, Initialized> {
//!     pub fn parameter_name<T>(self, value: T) -> Self { /* ... */ }
//!     // …one method per Essentia parameter…
//!     pub fn configure(self) -> Result<Foo<'a, Configured>, ConfigurationError> { /* ... */ }
//! }
//!
//! impl<'a> Foo<'a, Configured> {
//!     pub fn compute(&mut self, input1: …, input2: …)
//!         -> Result<FooResult<'a, '_>, ComputeError> { /* ... */ }
//! }
//!
//! pub struct FooResult<'algo, 'res> { /* ... */ }
//! impl FooResult { pub fn output_name(&self) -> DataContainer<'…, …> { /* ... */ } }
//! ```
//!
//! The two-state pattern (`Initialized` → `Configured`) prevents at compile
//! time misuses such as setting parameters after configuration or computing
//! before configuration.
//!
//! ## Regenerating
//!
//! The tree is regenerated automatically by the build script whenever the
//! crate is compiled. To regenerate it explicitly (e.g. after upgrading
//! Essentia, or to inspect the output without building everything else),
//! run:
//!
//! ```sh
//! cargo run -p essentia-codegen
//! ```

pub use essentia_core::algorithm::{Configured, Initialized};

mod error;
pub use error::*;

use crate::Essentia;

/// Trait implemented by every generated algorithm builder so that
/// [`Essentia::create`](crate::Essentia::create) can construct it generically.
///
/// This is **not** something user code is expected to implement directly. The
/// implementation is emitted by `essentia_codegen` for each algorithm.
pub trait CreateAlgorithm<'a> {
    /// Build a fresh, unconfigured algorithm bound to the given `Essentia`
    /// runtime handle.
    fn create(essentia: &'a Essentia) -> Self;
}

// The generated tree lives under `src/algorithm/generated/`. It is
// gitignored and rewritten on every build, but its path is stable
// across `cargo clean`s, which means `rust-analyzer` and `cargo doc`
// resolve it like any other module.
mod generated;
pub use generated::*;
