//! Generic algorithm wrapper, runtime introspection, and per-stage error
//! types.
//!
//! Where the user-facing [`essentia`](https://docs.rs/essentia) crate exposes
//! one Rust struct per Essentia algorithm — each with statically-typed
//! parameter, input and output methods — this module exposes the *generic*
//! machinery underneath:
//!
//! * [`Algorithm`] / [`Initialized`] / [`Configured`] — typestate-driven
//!   algorithm wrapper. The same struct is used regardless of which Essentia
//!   algorithm is running; the per-algorithm specifics come from
//!   introspection at runtime.
//! * [`Introspection`], [`InputOutputInfo`], [`ParameterInfo`],
//!   [`Constraint`] — read-only metadata describing a particular algorithm's
//!   parameters and inputs/outputs (their names, data types, descriptions
//!   and any constraints attached to them).
//! * [`ParameterError`], [`ConfigurationError`], [`InputError`],
//!   [`OutputError`], [`ComputeError`], [`ResetError`] — errors keyed to the
//!   stage of the lifecycle in which they can occur.
//! * [`ComputeResult`] — a short-lived handle returned by `compute()` that
//!   keeps an algorithm's outputs alive for as long as the result is in
//!   scope.
//!
//! Most users will not touch this module directly — it's the abstraction the
//! generated builders are built on top of.

mod algorithm;
mod error;
mod introspection;

pub use algorithm::{Algorithm, ComputeResult, Configured, Initialized};
pub use error::*;
pub use introspection::{Constraint, InputOutputInfo, Introspection, ParameterInfo};
