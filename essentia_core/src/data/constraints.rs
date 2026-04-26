//! Capability traits and validators that restrict which data types are valid
//! in which *role*.
//!
//! Essentia's runtime enforces that not every [`DataType`] is allowed in every
//! position. For instance:
//!
//! * **Parameters** can be scalars, vectors, matrices, maps, … but not the
//!   [`Pool`](crate::Pool) itself, and not `UnsignedInt`/`Long`/`Complex`.
//! * **Inputs / outputs** are the most permissive, including `Pool`,
//!   `UnsignedInt`, `Long`, `Complex`, `TensorFloat`.
//! * **Pool entries** are limited to a small subset of types (the pool is
//!   designed for accumulating features, not arbitrary data).
//!
//! This module encodes those restrictions at the type level via three
//! capability traits — [`ParameterData`], [`InputOutputData`] and
//! [`PoolData`] — each implemented for the subset of compile-time markers
//! from [`data_type`] that are valid in that role. Code that takes a
//! `T: ParameterData` will refuse at compile time anything that is not
//! parameter-shaped.
//!
//! The `ValidateConstraint` machinery and the `is_valid_*_type` helpers
//! provide the same information to generic code that needs to query
//! validity from a runtime [`DataType`] tag rather than from a generic
//! parameter.

use crate::data::types::{DataType, HasDataType, data_type};

/// Marker trait: implemented for every compile-time type marker that is
/// allowed as an algorithm **parameter**.
///
/// Used as a bound in [`Algorithm::set_parameter`](crate::Algorithm) and on
/// the generated per-algorithm builder methods.
pub trait ParameterData: HasDataType {}

/// Marker trait: implemented for every compile-time type marker that is
/// allowed as an algorithm **input** or **output**.
///
/// Used as a bound in [`Algorithm::set_input`](crate::Algorithm),
/// [`ComputeResult::output`](crate::algorithm::ComputeResult::output), and
/// the generated `compute(...)` / output-accessor methods.
pub trait InputOutputData: HasDataType {}

/// Marker trait: implemented for every compile-time type marker that is
/// allowed as a value inside a [`Pool`](crate::Pool).
///
/// Used as a bound in [`Pool::set`](crate::Pool::set) and
/// [`Pool::get`](crate::Pool::get).
pub trait PoolData: HasDataType {}

/// Compile-time predicate over a marker `T`. Implementations give the
/// constant `IS_VALID = true`; the absence of an impl means the constraint
/// is not satisfied. The companion [`Self::validate`] turns the constant
/// into a runtime `Result`, so that generic code can either trust the
/// constant statically or surface a uniform error.
pub trait ValidateConstraint<T> {
    /// `true` when `T` satisfies the constraint represented by `Self`.
    const IS_VALID: bool;

    /// Runtime form of [`Self::IS_VALID`].
    fn validate() -> Result<(), &'static str> {
        if Self::IS_VALID {
            Ok(())
        } else {
            Err("Type constraint violation")
        }
    }
}

// -- ParameterData impls --------------------------------------------------
// Note: Pool, UnsignedInt, Long, Complex, VectorComplex, VectorVectorComplex,
// MapVectorComplex, TensorFloat are **not** valid parameter types.

impl ParameterData for data_type::Float {}
impl ParameterData for data_type::String {}
impl ParameterData for data_type::Bool {}
impl ParameterData for data_type::Int {}
impl ParameterData for data_type::StereoSample {}
impl ParameterData for data_type::VectorFloat {}
impl ParameterData for data_type::VectorString {}
impl ParameterData for data_type::VectorBool {}
impl ParameterData for data_type::VectorInt {}
impl ParameterData for data_type::VectorStereoSample {}
impl ParameterData for data_type::VectorVectorFloat {}
impl ParameterData for data_type::VectorVectorString {}
impl ParameterData for data_type::VectorVectorStereoSample {}
impl ParameterData for data_type::VectorMatrixFloat {}
impl ParameterData for data_type::MapVectorFloat {}
impl ParameterData for data_type::MapVectorString {}
impl ParameterData for data_type::MapVectorInt {}
impl ParameterData for data_type::MapFloat {}
impl ParameterData for data_type::MatrixFloat {}

// -- InputOutputData impls -----------------------------------------------
// Includes everything ParameterData includes plus the I/O-only types
// (UnsignedInt, Long, Complex, VectorComplex, …, TensorFloat, Pool).

impl InputOutputData for data_type::Float {}
impl InputOutputData for data_type::UnsignedInt {}
impl InputOutputData for data_type::Long {}
impl InputOutputData for data_type::String {}
impl InputOutputData for data_type::Bool {}
impl InputOutputData for data_type::Int {}
impl InputOutputData for data_type::StereoSample {}
impl InputOutputData for data_type::Complex {}
impl InputOutputData for data_type::TensorFloat {}
impl InputOutputData for data_type::VectorFloat {}
impl InputOutputData for data_type::VectorString {}
impl InputOutputData for data_type::VectorBool {}
impl InputOutputData for data_type::VectorInt {}
impl InputOutputData for data_type::VectorStereoSample {}
impl InputOutputData for data_type::VectorComplex {}
impl InputOutputData for data_type::VectorVectorFloat {}
impl InputOutputData for data_type::VectorVectorString {}
impl InputOutputData for data_type::VectorVectorStereoSample {}
impl InputOutputData for data_type::VectorVectorComplex {}
impl InputOutputData for data_type::VectorMatrixFloat {}
impl InputOutputData for data_type::MapVectorFloat {}
impl InputOutputData for data_type::MapVectorString {}
impl InputOutputData for data_type::MapVectorInt {}
impl InputOutputData for data_type::MapVectorComplex {}
impl InputOutputData for data_type::MapFloat {}
impl InputOutputData for data_type::MatrixFloat {}
impl InputOutputData for data_type::Pool {}

// -- PoolData impls -------------------------------------------------------
// Pools deliberately accept only a small subset of types — the ones useful
// for feature aggregation. New impls here must be added to the C++ side
// of the pool bridge as well.

impl PoolData for data_type::Float {}
impl PoolData for data_type::String {}
impl PoolData for data_type::StereoSample {}
impl PoolData for data_type::VectorFloat {}
impl PoolData for data_type::VectorString {}
impl PoolData for data_type::VectorStereoSample {}
impl PoolData for data_type::TensorFloat {}

/// Phantom struct used as a witness that `T` is a valid parameter type.
///
/// `ParameterConstraint::<T>::IS_VALID` is true exactly when
/// `T: ParameterData`; the bound on the impl below is what enforces that.
pub struct ParameterConstraint<T>(std::marker::PhantomData<T>);

impl<T: ParameterData> ValidateConstraint<T> for ParameterConstraint<T> {
    const IS_VALID: bool = true;
}

/// Phantom struct used as a witness that `T` is a valid input/output type.
pub struct InputOutputConstraint<T>(std::marker::PhantomData<T>);

impl<T: InputOutputData> ValidateConstraint<T> for InputOutputConstraint<T> {
    const IS_VALID: bool = true;
}

/// Phantom struct used as a witness that `T` is a valid pool value type.
pub struct PoolConstraint<T>(std::marker::PhantomData<T>);

impl<T: PoolData> ValidateConstraint<T> for PoolConstraint<T> {
    const IS_VALID: bool = true;
}

/// Runtime equivalent of the `ParameterData` trait — given a [`DataType`]
/// tag, decide whether it is allowed as a parameter.
///
/// `const fn`, so usable in `static` initialisers and other compile-time
/// contexts.
pub const fn is_valid_parameter_type(data_type: DataType) -> bool {
    matches!(
        data_type,
        DataType::Float
            | DataType::String
            | DataType::Bool
            | DataType::Int
            | DataType::StereoSample
            | DataType::VectorFloat
            | DataType::VectorString
            | DataType::VectorBool
            | DataType::VectorInt
            | DataType::VectorStereoSample
            | DataType::VectorVectorFloat
            | DataType::VectorVectorString
            | DataType::VectorVectorStereoSample
            | DataType::VectorMatrixFloat
            | DataType::MapVectorFloat
            | DataType::MapVectorString
            | DataType::MapVectorInt
            | DataType::MapFloat
            | DataType::MatrixFloat
    )
}

/// Runtime equivalent of the `InputOutputData` trait.
pub const fn is_valid_input_output_type(data_type: DataType) -> bool {
    matches!(
        data_type,
        DataType::Float
            | DataType::UnsignedInt
            | DataType::Long
            | DataType::String
            | DataType::Bool
            | DataType::Int
            | DataType::StereoSample
            | DataType::Complex
            | DataType::TensorFloat
            | DataType::VectorFloat
            | DataType::VectorString
            | DataType::VectorBool
            | DataType::VectorInt
            | DataType::VectorStereoSample
            | DataType::VectorComplex
            | DataType::VectorVectorFloat
            | DataType::VectorVectorString
            | DataType::VectorVectorStereoSample
            | DataType::VectorVectorComplex
            | DataType::VectorMatrixFloat
            | DataType::MapVectorFloat
            | DataType::MapVectorString
            | DataType::MapVectorInt
            | DataType::MapVectorComplex
            | DataType::MapFloat
            | DataType::MatrixFloat
            | DataType::Pool
    )
}

/// Runtime equivalent of the `PoolData` trait.
pub const fn is_valid_pool_type(data_type: DataType) -> bool {
    matches!(
        data_type,
        DataType::Float
            | DataType::String
            | DataType::StereoSample
            | DataType::VectorFloat
            | DataType::VectorString
            | DataType::VectorStereoSample
            | DataType::TensorFloat
    )
}
