use thiserror::Error;

/// Returned when [`configure`](crate::algorithm::Initialized) fails.
///
/// Configuration is the moment when Essentia validates the parameter values
/// against the algorithm's runtime expectations (ranges, mutually-exclusive
/// options, file existence for I/O algorithms, …) and prepares its internal
/// state. Failures from the C++ side surface here.
#[derive(Debug, Error)]
pub enum ConfigurationError {
    /// The C++ side rejected the configuration. The wrapped [`cxx::Exception`]
    /// carries Essentia's original error message.
    #[error("Configuration failed: {0}")]
    Internal(#[from] cxx::Exception),
}

/// Returned when `compute` fails on a configured algorithm.
///
/// Failures here are produced by Essentia itself during the actual analysis
/// (numerical errors, unexpected input shapes, model file missing, etc.). The
/// surrounding Rust code statically guarantees that input names and types are
/// correct — anything that reaches this error is a domain-level failure on the
/// C++ side.
#[derive(Debug, Error)]
pub enum ComputeError {
    /// The C++ side raised an exception during computation. The wrapped
    /// [`cxx::Exception`] carries Essentia's original error message.
    #[error("Computation failed: {0}")]
    Compute(#[from] cxx::Exception),
}

/// Returned when an algorithm cannot be reset to its post-configure state.
///
/// Some algorithms accumulate state across multiple `compute` calls (running
/// statistics, FFT buffers, …) and offer a `reset` to discard it without
/// re-creating the algorithm. This error is the C++ side's response if that
/// reset fails.
#[derive(Debug, Error)]
pub enum ResetError {
    /// The C++ side raised an exception during reset.
    #[error("Reset failed: {0}")]
    Internal(#[from] cxx::Exception),
}
