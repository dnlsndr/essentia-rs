//! Aggregated error types for the user-facing crate.
//!
//! These are convenience aggregations that wrap the more granular errors
//! defined alongside their respective stages (parameter, configuration,
//! compute). Library users that want a single, top-level error type for their
//! own code can use [`EssentiaError`].

use thiserror::Error;

/// Errors that may occur while *configuring* an algorithm.
///
/// Today this only forwards [`ConfigurationError`] from the C++ side; the
/// wrapper exists to keep room for higher-level configuration errors that may
/// be added later (e.g. parameter dependency checks).
#[derive(Debug, Error)]
pub enum ConfigureError {
    /// Configuration validation or initialisation failed in C++ Essentia.
    #[error("configuration error: {0}")]
    Configuration(#[from] ConfigurationError),
}

/// Errors that may occur while *computing* an already-configured algorithm.
#[derive(Debug, Error)]
pub enum ComputeError {
    /// The C++ side raised an exception during the actual numeric work.
    #[error("computation error: {0}")]
    Computation(#[from] CoreComputeError),
}

/// All errors that can come out of an algorithm's lifecycle, lumped into one
/// enum.
///
/// Useful for callers that don't care which stage failed and just want a
/// single `?` chain.
#[derive(Debug, Error)]
pub enum AlgorithmError {
    /// Failure during the configure step.
    #[error("configuration error: {0}")]
    Configure(#[from] ConfigureError),

    /// Failure during the compute step.
    #[error("computation error: {0}")]
    Compute(#[from] ComputeError),
}

/// Top-level error type aggregating both runtime (`Core`) failures and
/// algorithm-related failures.
///
/// This is the most general error a caller of this crate can ever see.
#[derive(Debug, Error)]
pub enum EssentiaError {
    /// A failure in the Essentia core (runtime initialisation, registry
    /// lookup, etc.).
    #[error("core error: {0}")]
    Core(#[from] CoreError),

    /// A failure in an algorithm's lifecycle.
    #[error("algorithm error: {0}")]
    Algorithm(#[from] AlgorithmError),
}
