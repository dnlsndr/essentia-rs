//! # essentia-core
//!
//! Generic, algorithm-agnostic Rust abstractions over the C++ Essentia
//! library. This crate sits between the raw FFI in
//! [`essentia_sys`](https://docs.rs/essentia-sys) and the user-facing
//! [`essentia`](https://docs.rs/essentia) crate.
//!
//! Three audiences read this crate:
//!
//! 1. **The user-facing `essentia` crate** — re-exports most of the types here.
//! 2. **The build-time `essentia_codegen` crate** — drives Essentia at build
//!    time to generate one Rust module per algorithm. The generated code
//!    refers back to the types in this crate.
//! 3. **Advanced users** who want to dispatch by algorithm name at runtime
//!    (rather than statically through the generated builders) can talk
//!    directly to [`Essentia`] / [`Algorithm`] from this crate.
//!
//! ## What lives here
//!
//! * [`Essentia`] — handle to the global C++ Essentia runtime, with
//!   automatic init/shutdown via reference counting.
//! * [`algorithm::Algorithm`] — generic, untyped algorithm wrapper. Holds an
//!   FFI bridge plus a typestate parameter for compile-time enforcement of
//!   the configure → compute order.
//! * [`algorithm::Introspection`] — runtime metadata describing an
//!   algorithm's parameters and inputs/outputs.
//! * [`data`] — the unified data system: a single tagged container, a
//!   compile-time type-marker family, and conversion traits between idiomatic
//!   Rust types and the FFI representation.
//! * [`parameter_map::ParameterMap`] — staged parameter values waiting to be
//!   handed off to `configure`.
//! * [`pool::Pool`] — Essentia's heterogeneous key/value store, often used to
//!   aggregate features across many frames.

// ==============================================================================
// NEW UNIFIED DATA SYSTEM - COMPILE-TIME TYPE SAFETY
// ==============================================================================

/// The unified data system: a tagged FFI container plus compile-time markers
/// and conversion traits.
///
/// Essentia uses a single C++ `Parameter`/`Value` type that carries a runtime
/// tag (`DataType`) and one of ~25 possible payloads (scalar, vector, map,
/// matrix, …). This module mirrors that design but layers a **compile-time**
/// type marker on top so that mismatches can be caught at the type level
/// rather than as runtime errors.
pub mod data;

// ==============================================================================
// CORE MODULES (updated to use new data system)
// ==============================================================================

/// Generic algorithm wrapper, introspection, and per-stage error types.
pub mod algorithm;

/// The [`Essentia`] runtime handle and its construction errors.
pub mod essentia;

/// Builder-side staging area for parameter values before `configure`.
pub mod parameter_map;

/// Essentia's [`Pool`] — a heterogeneous key/value store often used for
/// feature aggregation.
pub mod pool;

// ==============================================================================
// RE-EXPORTS - CLEAN API
// ==============================================================================

// Core data types with compile-time constraints
pub use data::{ConversionError, GetFromDataContainer, IntoDataContainer};
pub use data::{DataContainer, DataType, data_type};
pub use data::{InputOutputData, ParameterData, PoolData};

// Algorithm and execution
pub use algorithm::{Algorithm, Configured, Initialized, Introspection};
pub use essentia::{CreateAlgorithmError, Essentia};
pub use pool::{Pool, PoolError};

// Error types
pub use algorithm::{
    ComputeError, ConfigurationError, InputError, OutputError, ParameterError, ResetError,
};
