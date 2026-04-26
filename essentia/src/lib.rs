//! # essentia-rs
//!
//! Idiomatic Rust bindings for the [Essentia](https://essentia.upf.edu/) C++ audio
//! analysis library.
//!
//! ## What is Essentia?
//!
//! Essentia is an open-source C++ library developed at the Music Technology Group
//! (Universitat Pompeu Fabra). It contains hundreds of audio analysis *algorithms*
//! covering domains such as:
//!
//! * **Signal processing** — windowing, FFT, filtering, resampling, …
//! * **Spectral features** — MFCC, mel-bands, spectral centroid, spectral contrast, …
//! * **Rhythm** — beat tracking, tempo estimation, onset detection, …
//! * **Tonal** — pitch tracking, key/chord detection, HPCP, …
//! * **High-level** — danceability, mood, genre classification (often via TensorFlow
//!   models), …
//! * **I/O** — audio file loading/saving, metadata extraction, …
//!
//! The official C++ API is large and untyped: every algorithm is an instance of a
//! generic `Algorithm` class whose parameters, inputs and outputs are stored as
//! `std::variant`-like tagged values. This crate wraps that API in a Rust-friendly
//! form with two important guarantees:
//!
//! 1. **Compile-time type safety** — each algorithm in Essentia exposes a fixed set
//!    of named parameters/inputs/outputs, each with a specific data type. The
//!    [`essentia_codegen`](https://docs.rs/essentia-codegen) build dependency reads
//!    Essentia's runtime introspection and generates one Rust struct per algorithm,
//!    so the compiler refuses code that passes (say) a `String` where Essentia
//!    expects a `Float`.
//! 2. **RAII lifecycle** — Essentia must be globally initialised before any
//!    algorithm is used and torn down at the end. [`Essentia`] handles both via
//!    reference counting; you only ever interact with its safe Rust API.
//!
//! ## The four-step workflow
//!
//! Every Essentia algorithm follows the same shape, mirrored in this crate's
//! generated builder API:
//!
//! 1. **Create** the algorithm from an [`Essentia`] handle.
//! 2. **Set parameters** (configuration knobs such as `sampleRate`, `frameSize`,
//!    `windowType`). Parameters use `snake_case` builder methods on the
//!    `<Initialized>` state.
//! 3. **Configure** the algorithm with [`configure()`]. This consumes the builder
//!    and returns it in the `<Configured>` typestate, statically preventing
//!    parameter changes from this point on.
//! 4. **Compute** by calling `compute(...)` with the algorithm's inputs as
//!    positional arguments. The returned result struct exposes typed accessors
//!    for every output.
//!
//! ```ignore
//! use essentia::Essentia;
//! // (algorithm names below are illustrative — real ones are auto-generated)
//!
//! let essentia = Essentia::new();
//! let mut windowing = essentia
//!     .create::<essentia::algorithm::standard::Windowing>()
//!     .type_(essentia::algorithm::standard::WindowingType::Hann)
//!     .size(1024_i32)
//!     .configure()?;
//!
//! let result = windowing.compute(&audio_frame[..])?;
//! let windowed: Vec<f32> = result.frame().get();
//! ```
//!
//! ## Crate layout
//!
//! This is the user-facing crate. It re-exports the generic core types from
//! [`essentia_core`] and adds the per-algorithm builder structs that
//! [`essentia_codegen`](https://docs.rs/essentia-codegen) generates at build time
//! from Essentia's introspection. The generated code lives under `OUT_DIR` and is
//! pulled in via an `include!` in [`mod algorithm`].
//!
//! See the project README for build prerequisites — Essentia and its native
//! dependencies must be discoverable through `pkg-config`.
//!
//! [`configure()`]: algorithm::Configured

/// Auto-generated, per-algorithm builder structs, organised by Essentia category.
///
/// The contents of this module are produced at build time by
/// [`essentia_codegen`](https://docs.rs/essentia-codegen) and `include!`d here.
/// One Rust struct is generated per Essentia algorithm, plus one sub-module per
/// Essentia category (e.g. `rhythm`, `spectral`, `tonal`, …).
pub mod algorithm;

/// The [`Essentia`] handle that owns the global C++ Essentia lifecycle.
pub mod essentia;

/// Re-exports of the generic data system from [`essentia_core`].
///
/// These types and traits underpin every value flowing in or out of an Essentia
/// algorithm. They are generic — *not* tied to any one algorithm — and are the
/// same building blocks used by the auto-generated code.
pub use essentia_core::{data, parameter_map, pool};

pub use data::{
    ConversionError, DataContainer, DataType, GetFromDataContainer, InputOutputData,
    IntoDataContainer, ParameterData, PoolData, TryGetFromDataContainer, TryIntoDataContainer,
    data_type,
};

pub use algorithm::{Configured, Initialized};
pub use essentia::Essentia;

pub use pool::{Pool, PoolError};
