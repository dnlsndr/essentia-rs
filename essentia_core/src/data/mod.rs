//! The unified data system shared by every Essentia value.
//!
//! Essentia is a dynamically-typed C++ library: every parameter, input,
//! output, and pool value is a tagged variant that can hold one of about 25
//! different data types — scalars (`Float`, `Int`, `Bool`, `String`, …),
//! vectors, nested vectors, matrices, complex numbers, maps, the
//! [`Pool`](crate::Pool) itself, and so on.
//!
//! This module mirrors that variant in Rust **and** layers a compile-time
//! type marker over it, so that values can be type-checked statically at
//! the API boundary even though the underlying representation is dynamic.
//!
//! ## Conceptual layout
//!
//! ```text
//! +-------------------------------+   +-----------------------+
//! | DataContainer<'a, T>          |   | T: HasDataType        |
//! |   (typed, compile-time)       |---|   data_type::Float    |
//! +---------------+---------------+   |   data_type::VectorInt|
//!                 |                   |   …                   |
//!                 v                   +-----------------------+
//! +-------------------------------+
//! | ffi::DataContainer            |
//! |   (untyped, runtime tag)      |
//! +-------------------------------+
//!                 |
//!                 v
//! +-------------------------------+
//! | C++ essentia::Parameter /     |
//! |   Value with std::variant     |
//! +-------------------------------+
//! ```
//!
//! ## Three layers, three responsibilities
//!
//! * [`types`] defines the runtime [`DataType`] enum *and* its
//!   compile-time mirror under [`data_type`]. The two are linked by the
//!   [`HasDataType`](types::HasDataType) trait.
//! * [`constraints`] adds **role-specific capability traits** — Essentia
//!   does not allow every type everywhere (e.g. only a subset can be used
//!   as parameters; `Pool` can be an input/output but not a parameter,
//!   etc.). [`ParameterData`], [`InputOutputData`] and [`PoolData`] encode
//!   those role restrictions at the type level.
//! * [`container::DataContainer`] is the actual typed handle to a value.
//!   It internally holds either an owned [`cxx::UniquePtr`] or a borrowed
//!   reference to an FFI container, with a phantom `T` to remember the
//!   compile-time type.
//!
//! ## Conversions
//!
//! Idiomatic Rust types do not match Essentia's variant payloads
//! one-to-one. The conversion traits live in their own files:
//!
//! * [`IntoDataContainer<T>`] — `f32 → DataContainer<data_type::Float>`,
//!   `&[f32] → DataContainer<data_type::VectorFloat>`, etc.
//! * [`GetFromDataContainer<T>`] — the reverse direction, used to read
//!   outputs and pool entries back into Rust types.
//! * Their `Try*` variants exist for conversions that can fail at runtime
//!   (e.g. turning `&[Vec<f32>]` into a [`MatrixFloat`](data_type::MatrixFloat)
//!   when rows are non-rectangular).

pub mod constraints;
pub mod container;
mod conversion_error;
mod conversion_get;
mod conversion_into;

pub mod types;

pub use constraints::{InputOutputData, ParameterData, PoolData, ValidateConstraint};
pub use container::DataContainer;
pub use conversion_error::ConversionError;
pub use conversion_get::{GetFromDataContainer, TryGetFromDataContainer};
pub use conversion_into::{IntoDataContainer, TryIntoDataContainer};
pub use types::{DataType, data_type};
