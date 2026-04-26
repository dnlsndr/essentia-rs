use thiserror::Error;

/// Returned by [`Essentia::create_algorithm`](super::Essentia::create_algorithm)
/// when the requested algorithm name is unknown.
///
/// Algorithm names are matched case-sensitively against the names registered
/// with the C++ Essentia runtime. The full list is available via
/// [`Essentia::available_algorithms`](super::Essentia::available_algorithms).
#[derive(Debug, Error)]
pub enum CreateAlgorithmError {
    /// No algorithm with that name is registered. Common causes:
    /// typo, case mismatch, or trying to use a TensorFlow-only algorithm in
    /// a build that disabled `USE_TENSORFLOW`.
    #[error("algorithm not found: {name}")]
    AlgorithmNotFound {
        /// The unknown name supplied by the caller.
        name: String,
    },
}
