use thiserror::Error;

/// Returned by the fallible `Try*` conversions in
/// [`conversion_into`](super::conversion_into) and
/// [`conversion_get`](super::conversion_get) when the source data does not
/// fit the requested target shape or type.
#[derive(Debug, Error)]
pub enum ConversionError {
    /// The runtime [`DataType`](super::DataType) of the source did not match
    /// the destination's static type. In practice this is rarely reached
    /// because the typed [`DataContainer<T>`](super::DataContainer) makes
    /// the mismatch impossible at compile time — it shows up only when
    /// the static type was erased via `into_any` first.
    #[error("Type mismatch during conversion: {message}")]
    TypeMismatch { message: String },

    /// The source value carries the right [`DataType`](super::DataType) but
    /// its shape is invalid for the requested destination — for example,
    /// trying to interpret a non-rectangular `VectorVectorFloat` as a 2-D
    /// matrix.
    #[error("Invalid data format: {message}")]
    InvalidFormat { message: String },

    /// The conversion is not implemented for this combination of source
    /// and destination types.
    #[error("Conversion not supported: {message}")]
    NotSupported { message: String },
}
