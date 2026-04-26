//! Runtime [`DataType`] tag and its compile-time mirror under [`data_type`].
//!
//! Essentia is dynamically typed; every value carries a runtime tag selected
//! from a fixed set of ~25 types. This file:
//!
//! 1. Defines [`DataType`], the Rust-side mirror of the C++ enum.
//! 2. Defines a parallel set of *zero-sized marker structs* under [`data_type`]
//!    — `data_type::Float`, `data_type::VectorInt`, etc. — that act as
//!    compile-time tokens. Each marker has exactly one corresponding
//!    [`DataType`] variant.
//! 3. Defines the [`HasDataType`] trait, the bridge between the two: every
//!    marker implements it with a `const DATA_TYPE: DataType` so that generic
//!    code over `T: HasDataType` can recover the runtime tag at compile time.
//!
//! The point of this dual representation is that the compile-time markers can
//! be used as type parameters (e.g. `DataContainer<data_type::Float>`) so the
//! type checker enforces correctness, while the runtime tag is still available
//! for cases where the value's type is only known dynamically (introspection,
//! generic `DataContainer::data_type()` queries, …).

use essentia_sys::ffi;
use std::fmt;

/// Runtime tag identifying which payload an Essentia value carries.
///
/// Mirrors the C++ enum on the FFI side. Each variant has a corresponding
/// zero-sized marker in the [`data_type`] module and a `HasDataType`
/// implementation linking the two.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataType {
    /// Single-precision floating point (`f32` in Rust, `Real` in Essentia).
    Float,
    /// UTF-8 string.
    String,
    /// Boolean.
    Bool,
    /// Signed 32-bit integer.
    Int,
    /// Unsigned 32-bit integer (used by some I/O algorithms for sample
    /// counts, etc.).
    UnsignedInt,
    /// Signed 64-bit integer.
    Long,
    /// Stereo audio sample — a `(left, right)` pair of `f32`.
    StereoSample,
    /// Single complex number (`re + im·i`, both `f32`).
    Complex,
    /// 4-D tensor of `f32` (used by Essentia's TensorFlow integration).
    TensorFloat,
    /// `Vec<f32>` — the most common audio-data carrier (a frame, a spectrum, …).
    VectorFloat,
    /// `Vec<String>`.
    VectorString,
    /// `Vec<bool>`.
    VectorBool,
    /// `Vec<i32>`.
    VectorInt,
    /// `Vec<StereoSample>` — typically a stereo audio buffer.
    VectorStereoSample,
    /// `Vec<Complex>` — e.g. an FFT output.
    VectorComplex,
    /// `Vec<Vec<f32>>` — e.g. a sequence of frames.
    VectorVectorFloat,
    /// `Vec<Vec<String>>`.
    VectorVectorString,
    /// `Vec<Vec<StereoSample>>`.
    VectorVectorStereoSample,
    /// `Vec<Vec<Complex>>`.
    VectorVectorComplex,
    /// `Vec<MatrixFloat>` — sequence of 2-D matrices.
    VectorMatrixFloat,
    /// `Map<String, Vec<f32>>` — feature aggregations keyed by name.
    MapVectorFloat,
    /// `Map<String, Vec<String>>`.
    MapVectorString,
    /// `Map<String, Vec<i32>>`.
    MapVectorInt,
    /// `Map<String, Vec<Complex>>`.
    MapVectorComplex,
    /// `Map<String, f32>`.
    MapFloat,
    /// 2-D matrix of `f32`, in row-major order.
    MatrixFloat,
    /// An entire [`Pool`](crate::Pool) — an algorithm can both produce and
    /// consume one as a single value.
    Pool,
}

impl DataType {
    /// Stable, human-readable name for this type. Matches the variant
    /// identifier (`"Float"`, `"VectorFloat"`, …) — handy for error messages
    /// and for displaying introspection results.
    pub fn as_str(&self) -> &'static str {
        match self {
            DataType::Float => "Float",
            DataType::String => "String",
            DataType::Bool => "Bool",
            DataType::Int => "Int",
            DataType::UnsignedInt => "UnsignedInt",
            DataType::Long => "Long",
            DataType::StereoSample => "StereoSample",
            DataType::Complex => "Complex",
            DataType::TensorFloat => "TensorFloat",
            DataType::VectorFloat => "VectorFloat",
            DataType::VectorString => "VectorString",
            DataType::VectorBool => "VectorBool",
            DataType::VectorInt => "VectorInt",
            DataType::VectorStereoSample => "VectorStereoSample",
            DataType::VectorComplex => "VectorComplex",
            DataType::VectorVectorFloat => "VectorVectorFloat",
            DataType::VectorVectorString => "VectorVectorString",
            DataType::VectorVectorStereoSample => "VectorVectorStereoSample",
            DataType::VectorVectorComplex => "VectorVectorComplex",
            DataType::VectorMatrixFloat => "VectorMatrixFloat",
            DataType::MapVectorFloat => "MapVectorFloat",
            DataType::MapVectorString => "MapVectorString",
            DataType::MapVectorInt => "MapVectorInt",
            DataType::MapVectorComplex => "MapVectorComplex",
            DataType::MapFloat => "MapFloat",
            DataType::MatrixFloat => "MatrixFloat",
            DataType::Pool => "Pool",
        }
    }
}

impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<ffi::DataType> for DataType {
    /// Convert from the raw FFI enum to this Rust-side mirror.
    ///
    /// Panics if the C++ side ever introduces a new variant that hasn't been
    /// added here — that would mean the bindings are out of sync.
    fn from(ffi_type: ffi::DataType) -> Self {
        match ffi_type {
            ffi::DataType::Float => DataType::Float,
            ffi::DataType::String => DataType::String,
            ffi::DataType::Bool => DataType::Bool,
            ffi::DataType::Int => DataType::Int,
            ffi::DataType::UnsignedInt => DataType::UnsignedInt,
            ffi::DataType::Long => DataType::Long,
            ffi::DataType::StereoSample => DataType::StereoSample,
            ffi::DataType::Complex => DataType::Complex,
            ffi::DataType::TensorFloat => DataType::TensorFloat,
            ffi::DataType::VectorFloat => DataType::VectorFloat,
            ffi::DataType::VectorString => DataType::VectorString,
            ffi::DataType::VectorBool => DataType::VectorBool,
            ffi::DataType::VectorInt => DataType::VectorInt,
            ffi::DataType::VectorStereoSample => DataType::VectorStereoSample,
            ffi::DataType::VectorComplex => DataType::VectorComplex,
            ffi::DataType::VectorVectorFloat => DataType::VectorVectorFloat,
            ffi::DataType::VectorVectorString => DataType::VectorVectorString,
            ffi::DataType::VectorVectorStereoSample => DataType::VectorVectorStereoSample,
            ffi::DataType::VectorVectorComplex => DataType::VectorVectorComplex,
            ffi::DataType::VectorMatrixFloat => DataType::VectorMatrixFloat,
            ffi::DataType::MapVectorFloat => DataType::MapVectorFloat,
            ffi::DataType::MapVectorString => DataType::MapVectorString,
            ffi::DataType::MapVectorInt => DataType::MapVectorInt,
            ffi::DataType::MapVectorComplex => DataType::MapVectorComplex,
            ffi::DataType::MapFloat => DataType::MapFloat,
            ffi::DataType::MatrixFloat => DataType::MatrixFloat,
            ffi::DataType::Pool => DataType::Pool,
            _ => panic!("Encountered unknown FFI DataType: {:?}", ffi_type),
        }
    }
}

impl From<DataType> for ffi::DataType {
    /// Convert back to the raw FFI enum, used when the Rust side needs to
    /// hand a runtime type tag to C++ (e.g. `setup_output`).
    fn from(data_type: DataType) -> Self {
        match data_type {
            DataType::Float => ffi::DataType::Float,
            DataType::String => ffi::DataType::String,
            DataType::Bool => ffi::DataType::Bool,
            DataType::Int => ffi::DataType::Int,
            DataType::UnsignedInt => ffi::DataType::UnsignedInt,
            DataType::Long => ffi::DataType::Long,
            DataType::StereoSample => ffi::DataType::StereoSample,
            DataType::Complex => ffi::DataType::Complex,
            DataType::TensorFloat => ffi::DataType::TensorFloat,
            DataType::VectorFloat => ffi::DataType::VectorFloat,
            DataType::VectorString => ffi::DataType::VectorString,
            DataType::VectorBool => ffi::DataType::VectorBool,
            DataType::VectorInt => ffi::DataType::VectorInt,
            DataType::VectorStereoSample => ffi::DataType::VectorStereoSample,
            DataType::VectorComplex => ffi::DataType::VectorComplex,
            DataType::VectorVectorFloat => ffi::DataType::VectorVectorFloat,
            DataType::VectorVectorString => ffi::DataType::VectorVectorString,
            DataType::VectorVectorStereoSample => ffi::DataType::VectorVectorStereoSample,
            DataType::VectorVectorComplex => ffi::DataType::VectorVectorComplex,
            DataType::VectorMatrixFloat => ffi::DataType::VectorMatrixFloat,
            DataType::MapVectorFloat => ffi::DataType::MapVectorFloat,
            DataType::MapVectorString => ffi::DataType::MapVectorString,
            DataType::MapVectorInt => ffi::DataType::MapVectorInt,
            DataType::MapVectorComplex => ffi::DataType::MapVectorComplex,
            DataType::MapFloat => ffi::DataType::MapFloat,
            DataType::MatrixFloat => ffi::DataType::MatrixFloat,
            DataType::Pool => ffi::DataType::Pool,
        }
    }
}

/// Compile-time tags that mirror every variant of [`DataType`].
///
/// Each marker struct here is *zero-sized* and exists only at the type level.
/// They are used as type arguments to [`DataContainer<T>`](crate::DataContainer)
/// and as the right-hand side of conversion traits (`IntoDataContainer<T>`,
/// `GetFromDataContainer<T>`).
///
/// The mapping is one-to-one:
///
/// | runtime variant            | compile-time marker |
/// |----------------------------|---------------------|
/// | [`DataType::Float`]        | [`Float`]           |
/// | [`DataType::VectorFloat`]  | [`VectorFloat`]     |
/// | [`DataType::Pool`]         | [`Pool`]            |
/// | … etc.                     | …                   |
///
/// Why not just use `DataType` as a const generic? Because `DataType` is an
/// enum, and using it as `DataContainer<const T: DataType>` would require
/// `feature(adt_const_params)`. Using a marker type per variant keeps the
/// crate on stable Rust and lets each marker independently implement traits
/// like [`HasDataType`], [`ParameterData`](super::ParameterData),
/// [`InputOutputData`](super::InputOutputData), and
/// [`PoolData`](super::PoolData).
pub mod data_type {
    /// Type-erased marker — used by [`DataContainer::into_any`] to opt out
    /// of static type checking. A `DataContainer<Any>` still carries a
    /// runtime tag.
    pub struct Any;

    // -- Scalars --------------------------------------------------------------
    /// Boolean. Maps to [`super::DataType::Bool`].
    pub struct Bool;
    /// UTF-8 string. Maps to [`super::DataType::String`].
    pub struct String;
    /// Signed 32-bit integer. Maps to [`super::DataType::Int`].
    pub struct Int;
    /// Single-precision float. Maps to [`super::DataType::Float`].
    pub struct Float;
    /// Unsigned 32-bit integer. Maps to [`super::DataType::UnsignedInt`].
    pub struct UnsignedInt;
    /// Signed 64-bit integer. Maps to [`super::DataType::Long`].
    pub struct Long;
    /// `(left, right)` stereo audio sample. Maps to
    /// [`super::DataType::StereoSample`].
    pub struct StereoSample;
    /// Single complex number. Maps to [`super::DataType::Complex`].
    pub struct Complex;
    /// 4-D float tensor (TensorFlow integration). Maps to
    /// [`super::DataType::TensorFloat`].
    pub struct TensorFloat;

    // -- Vectors --------------------------------------------------------------
    /// `Vec<bool>`. Maps to [`super::DataType::VectorBool`].
    pub struct VectorBool;
    /// `Vec<String>`. Maps to [`super::DataType::VectorString`].
    pub struct VectorString;
    /// `Vec<i32>`. Maps to [`super::DataType::VectorInt`].
    pub struct VectorInt;
    /// `Vec<f32>` — the most common audio carrier. Maps to
    /// [`super::DataType::VectorFloat`].
    pub struct VectorFloat;
    /// `Vec<StereoSample>`. Maps to [`super::DataType::VectorStereoSample`].
    pub struct VectorStereoSample;
    /// `Vec<Complex>`. Maps to [`super::DataType::VectorComplex`].
    pub struct VectorComplex;

    // -- Nested vectors -------------------------------------------------------
    /// `Vec<Vec<f32>>`. Maps to [`super::DataType::VectorVectorFloat`].
    pub struct VectorVectorFloat;
    /// `Vec<Vec<String>>`. Maps to [`super::DataType::VectorVectorString`].
    pub struct VectorVectorString;
    /// `Vec<Vec<StereoSample>>`. Maps to
    /// [`super::DataType::VectorVectorStereoSample`].
    pub struct VectorVectorStereoSample;
    /// `Vec<Vec<Complex>>`. Maps to [`super::DataType::VectorVectorComplex`].
    pub struct VectorVectorComplex;
    /// `Vec<MatrixFloat>`. Maps to [`super::DataType::VectorMatrixFloat`].
    pub struct VectorMatrixFloat;

    // -- Matrices -------------------------------------------------------------
    /// 2-D matrix of `f32`, row-major. Maps to [`super::DataType::MatrixFloat`].
    pub struct MatrixFloat;

    // -- Maps -----------------------------------------------------------------
    /// `Map<String, Vec<f32>>`. Maps to [`super::DataType::MapVectorFloat`].
    pub struct MapVectorFloat;
    /// `Map<String, Vec<String>>`. Maps to
    /// [`super::DataType::MapVectorString`].
    pub struct MapVectorString;
    /// `Map<String, Vec<i32>>`. Maps to [`super::DataType::MapVectorInt`].
    pub struct MapVectorInt;
    /// `Map<String, Vec<Complex>>`. Maps to
    /// [`super::DataType::MapVectorComplex`].
    pub struct MapVectorComplex;
    /// `Map<String, f32>`. Maps to [`super::DataType::MapFloat`].
    pub struct MapFloat;

    /// An entire [`Pool`](crate::Pool) used as a value. Maps to
    /// [`super::DataType::Pool`].
    pub struct Pool;
}

/// Bridges the compile-time markers in [`data_type`] to their runtime
/// [`DataType`] variant.
///
/// Implemented exactly once for each marker. Generic code can therefore go
/// from `T: HasDataType` to a [`DataType`] at no runtime cost — the value
/// is `T::DATA_TYPE`, a `const`.
///
/// Example use: when `Algorithm::set_parameter::<T>` validates a user-supplied
/// value against the introspected parameter type, it calls `T::data_type()`
/// to obtain the runtime tag for `T`.
pub trait HasDataType {
    /// The [`DataType`] variant this marker corresponds to.
    const DATA_TYPE: DataType;

    /// Convenience accessor returning [`Self::DATA_TYPE`] as a value.
    fn data_type() -> DataType {
        Self::DATA_TYPE
    }
}

impl HasDataType for data_type::Bool {
    const DATA_TYPE: DataType = DataType::Bool;
}

impl HasDataType for data_type::String {
    const DATA_TYPE: DataType = DataType::String;
}

impl HasDataType for data_type::Int {
    const DATA_TYPE: DataType = DataType::Int;
}

impl HasDataType for data_type::Float {
    const DATA_TYPE: DataType = DataType::Float;
}

impl HasDataType for data_type::UnsignedInt {
    const DATA_TYPE: DataType = DataType::UnsignedInt;
}

impl HasDataType for data_type::Long {
    const DATA_TYPE: DataType = DataType::Long;
}

impl HasDataType for data_type::StereoSample {
    const DATA_TYPE: DataType = DataType::StereoSample;
}

impl HasDataType for data_type::Complex {
    const DATA_TYPE: DataType = DataType::Complex;
}

impl HasDataType for data_type::TensorFloat {
    const DATA_TYPE: DataType = DataType::TensorFloat;
}

impl HasDataType for data_type::VectorBool {
    const DATA_TYPE: DataType = DataType::VectorBool;
}

impl HasDataType for data_type::VectorString {
    const DATA_TYPE: DataType = DataType::VectorString;
}

impl HasDataType for data_type::VectorInt {
    const DATA_TYPE: DataType = DataType::VectorInt;
}

impl HasDataType for data_type::VectorFloat {
    const DATA_TYPE: DataType = DataType::VectorFloat;
}

impl HasDataType for data_type::VectorStereoSample {
    const DATA_TYPE: DataType = DataType::VectorStereoSample;
}

impl HasDataType for data_type::VectorComplex {
    const DATA_TYPE: DataType = DataType::VectorComplex;
}

impl HasDataType for data_type::VectorVectorFloat {
    const DATA_TYPE: DataType = DataType::VectorVectorFloat;
}

impl HasDataType for data_type::VectorVectorString {
    const DATA_TYPE: DataType = DataType::VectorVectorString;
}

impl HasDataType for data_type::VectorVectorStereoSample {
    const DATA_TYPE: DataType = DataType::VectorVectorStereoSample;
}

impl HasDataType for data_type::VectorVectorComplex {
    const DATA_TYPE: DataType = DataType::VectorVectorComplex;
}

impl HasDataType for data_type::VectorMatrixFloat {
    const DATA_TYPE: DataType = DataType::VectorMatrixFloat;
}

impl HasDataType for data_type::MatrixFloat {
    const DATA_TYPE: DataType = DataType::MatrixFloat;
}

impl HasDataType for data_type::MapVectorFloat {
    const DATA_TYPE: DataType = DataType::MapVectorFloat;
}

impl HasDataType for data_type::MapVectorString {
    const DATA_TYPE: DataType = DataType::MapVectorString;
}

impl HasDataType for data_type::MapVectorInt {
    const DATA_TYPE: DataType = DataType::MapVectorInt;
}

impl HasDataType for data_type::MapVectorComplex {
    const DATA_TYPE: DataType = DataType::MapVectorComplex;
}

impl HasDataType for data_type::MapFloat {
    const DATA_TYPE: DataType = DataType::MapFloat;
}

impl HasDataType for data_type::Pool {
    const DATA_TYPE: DataType = DataType::Pool;
}
