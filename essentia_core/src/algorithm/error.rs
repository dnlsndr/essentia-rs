//! Per-stage error types for the [`Algorithm`](super::Algorithm) lifecycle.
//!
//! Each error type covers a single phase of the lifecycle so callers can
//! pattern-match precisely on what went wrong without resorting to a giant
//! catch-all enum.

use thiserror::Error;

use crate::data::DataType;

/// Returned by the parameter-setter methods on
/// [`Algorithm<Initialized>`](super::Algorithm).
///
/// Both variants here are introspection failures — they happen *before*
/// anything is sent to C++ Essentia. The corresponding C++-side error
/// (configuration failure, range violation, …) shows up at
/// [`configure`](super::Algorithm::configure) time as
/// [`ConfigurationError`].
#[derive(Debug, Error)]
pub enum ParameterError {
    /// No parameter with that name exists on this algorithm. The
    /// introspection metadata enumerates the valid names.
    #[error("Parameter '{parameter}' not found")]
    ParameterNotFound { parameter: String },

    /// The supplied Rust type's [`DataType`] doesn't match the parameter's
    /// declared type.
    #[error("Type mismatch for parameter '{parameter}': expected {expected}, found {actual}")]
    TypeMismatch {
        /// Name of the offending parameter.
        parameter: String,
        /// Type the algorithm declares for this parameter (per
        /// introspection).
        expected: DataType,
        /// Type the caller actually supplied.
        actual: DataType,
    },
}

/// Returned by [`Algorithm::configure`](super::Algorithm::configure) when
/// C++ Essentia rejects the staged parameters.
///
/// Today it only forwards the C++ exception verbatim; structured variants
/// could be added later if/when Essentia surfaces structured error types.
#[derive(Debug, Error)]
pub enum ConfigurationError {
    /// The C++ side raised an exception during configuration. The wrapped
    /// [`cxx::Exception`] carries Essentia's original error message.
    #[error("Configuration failed: {0}")]
    Internal(#[from] cxx::Exception),
}

/// Returned by [`Algorithm::set_input`](super::Algorithm::set_input) and
/// [`Algorithm::input`](super::Algorithm::input).
#[derive(Debug, Error)]
pub enum InputError {
    /// No input with that name exists on this algorithm.
    #[error("Input '{input}' not found")]
    InputNotFound { input: String },

    /// The supplied Rust type's [`DataType`] doesn't match the input's
    /// declared type.
    #[error("Type mismatch for input '{input}': expected {expected}, found {actual}")]
    TypeMismatch {
        /// Name of the offending input.
        input: String,
        /// Type the algorithm declares for this input (per introspection).
        expected: DataType,
        /// Type the caller actually supplied.
        actual: DataType,
    },
}

/// Returned by
/// [`ComputeResult::output`](super::ComputeResult::output) when the caller
/// asks for an output that doesn't exist or with the wrong static type.
#[derive(Debug, Error)]
pub enum OutputError {
    /// No output with that name exists on this algorithm.
    #[error("Output '{output}' not found")]
    OutputNotFound { output: String },

    /// The requested Rust type's [`DataType`] doesn't match the output's
    /// declared type.
    #[error("Type mismatch for output '{output}': expected {expected}, found {actual}")]
    TypeMismatch {
        /// Name of the offending output.
        output: String,
        /// Type the algorithm declares for this output (per introspection).
        expected: DataType,
        /// Type the caller asked for.
        actual: DataType,
    },
}

/// Returned by [`Algorithm::compute`](super::Algorithm::compute) when the
/// C++ side fails to actually produce results.
///
/// Statically-checked errors (input not found, type mismatch) cannot reach
/// this point in the generated code — they have already been caught at
/// `set_input`. An error here therefore reflects a domain failure inside
/// Essentia (numerical, I/O, malformed audio data, …).
#[derive(Debug, Error)]
pub enum ComputeError {
    /// The C++ side raised an exception during computation. The wrapped
    /// [`cxx::Exception`] carries Essentia's original error message.
    #[error("Computation failed: {0}")]
    Compute(#[from] cxx::Exception),
}

/// Returned by [`Algorithm::reset`](super::Algorithm::reset).
#[derive(Debug, Error)]
pub enum ResetError {
    /// The C++ side raised an exception during reset.
    #[error("Reset failed: {0}")]
    Internal(#[from] cxx::Exception),
}
