//! # essentia-sys
//!
//! Raw FFI bindings to the C++ Essentia library.
//!
//! Everything in this crate is intentionally low-level. The user-facing
//! safety, type checking, and ergonomic adapters live in
//! [`essentia_core`](https://docs.rs/essentia-core) and
//! [`essentia`](https://docs.rs/essentia).
//!
//! ## What `cxx::bridge` is doing here
//!
//! [`cxx`](https://cxx.rs) lets us write a single Rust module annotated
//! with `#[cxx::bridge]`; the macro then synthesises matching C++ headers
//! and Rust extern definitions on either side of the FFI boundary. The
//! bodies of every function declared in `unsafe extern "C++" { … }` block
//! are compiled from the C++ files under `bridge/` (see `build.rs`).
//!
//! ## Wire-level design
//!
//! Essentia's API is dynamically typed — every value flowing through it is
//! one of ~25 possible payload shapes. Rather than expose a separate FFI
//! function per type, this bridge funnels everything through:
//!
//! * [`ffi::DataContainer`] — the C++-side tagged variant.
//! * [`ffi::DataType`] — its runtime tag, mirrored as a Rust enum.
//! * One `create_data_container_from_<type>` constructor per supported
//!   payload (e.g. [`ffi::create_data_container_from_float`]) and one
//!   `get_<type>` accessor per payload
//!   (e.g. [`ffi::DataContainer::get_float`]).
//!
//! [`ffi::AlgorithmBridge`], [`ffi::ParameterMapBridge`] and
//! [`ffi::PoolBridge`] are thin wrappers around their C++ counterparts —
//! their methods just forward to the actual Essentia API.
//!
//! ## Helper structs
//!
//! Several plain-old-data structs in the bridge ([`ffi::SliceFloat`],
//! [`ffi::MatrixFloat`], [`ffi::TensorFloat`], the various `MapEntry…`)
//! exist purely as serialisable layouts for nested data — `cxx` cannot
//! pass `&[&[T]]` or `HashMap<String, Vec<T>>` directly, so we hand-craft
//! a flat representation.

/// FFI module synthesised by `#[cxx::bridge]`.
///
/// Every declaration here corresponds to a binding generated on both
/// sides of the FFI: Rust-callable C++ code on the C++ side, and the
/// extern declarations that let Rust call into them on this side.
#[cxx::bridge(namespace = "essentia_bridge")]
pub mod ffi {

    // ===== Helper Structs =====

    /// Borrowed `&[f32]` packaged for the cxx bridge — used as element type
    /// of a `Vec<SliceFloat>` to represent `Vec<Vec<f32>>` without
    /// copying.
    pub struct SliceFloat<'a> {
        slice: &'a [f32],
    }

    /// Owned `Vec<String>` packaged for the cxx bridge — used as element
    /// type of `Vec<VecString>` to represent `Vec<Vec<String>>`.
    pub struct VecString {
        vec: Vec<String>,
    }

    /// Borrowed `&[StereoSample]`, used the same way as
    /// [`SliceFloat`] but for stereo audio buffers.
    pub struct SliceStereoSample<'a> {
        slice: &'a [StereoSample],
    }

    /// Borrowed view of a 2-D matrix as a flat slice plus its dimensions.
    /// Row-major layout: index `(r, c)` is `slice[r * dim2 + c]`.
    pub struct MatrixFloat<'a> {
        slice: &'a [f32],
        dim1: usize,
        dim2: usize,
    }

    /// Borrowed view of an N-D tensor as a flat slice plus its shape
    /// (currently always 4 dimensions, matching Essentia's TensorFlow
    /// integration).
    pub struct TensorFloat<'a> {
        slice: &'a [f32],
        shape: &'a [usize],
    }

    /// One entry in a map whose values are `Vec<f32>`. The value is
    /// borrowed.
    pub struct MapEntryVectorFloat<'a> {
        key: String,
        value: &'a [f32],
    }

    /// One entry in a map whose values are `Vec<String>`. The value is
    /// owned.
    pub struct MapEntryVectorString {
        key: String,
        value: Vec<String>,
    }

    /// One entry in a map whose values are `Vec<i32>`. The value is
    /// borrowed.
    pub struct MapEntryVectorInt<'a> {
        key: String,
        value: &'a [i32],
    }

    /// One entry in a map whose values are scalar `f32`.
    pub struct MapEntryFloat {
        key: String,
        value: f32,
    }

    /// `(left, right)` stereo audio sample.
    #[derive(Clone, Debug)]
    pub struct StereoSample {
        left: f32,
        right: f32,
    }

    /// Single complex number. Mirrors `num::Complex<f32>` but lives on the
    /// FFI side, so we have to repack at the boundary.
    #[derive(Clone, Debug)]
    pub struct Complex {
        real: f32,
        imag: f32,
    }

    /// Owned `Vec<Complex>` packaged for the cxx bridge.
    pub struct VecComplex {
        vec: Vec<Complex>,
    }

    /// One entry in a map whose values are `Vec<Complex>`. The value is
    /// borrowed.
    pub struct MapEntryVectorComplex<'a> {
        key: String,
        value: &'a [Complex],
    }

    // ===== Data Type Enum =====

    /// Runtime tag identifying which payload an Essentia value carries.
    ///
    /// One-to-one mirror of the Rust-side
    /// [`DataType`](essentia_core::DataType). The two are kept separate
    /// so user code never needs to depend on `essentia-sys` directly.
    #[derive(Debug, Clone, Copy)]
    pub enum DataType {
        Float,
        String,
        Bool,
        Int,
        UnsignedInt,
        Long,
        StereoSample,
        Complex,
        TensorFloat,
        VectorFloat,
        VectorString,
        VectorBool,
        VectorInt,
        VectorStereoSample,
        VectorComplex,
        VectorVectorFloat,
        VectorVectorString,
        VectorVectorStereoSample,
        VectorVectorComplex,
        VectorMatrixFloat,
        MapVectorFloat,
        MapVectorString,
        MapVectorInt,
        MapVectorComplex,
        MapFloat,
        MatrixFloat,
        Pool,
    }

    // ===== Introspection Structs =====

    /// Description of a single algorithm parameter, returned by
    /// [`AlgorithmBridge::get_parameter_infos`].
    pub struct ParameterInfo {
        /// Parameter name as the C++ side knows it.
        name: String,
        /// Declared payload type.
        data_type: DataType,
        /// Free-form constraint string (e.g. `"(0,inf)"`,
        /// `"{hann,hamming,…}"`). Parsed Rust-side into
        /// [`Constraint`](essentia_core::algorithm::Constraint).
        constraint: String,
        /// Free-form English description.
        description: String,
        /// Default value rendered as a string. Empty for required
        /// parameters.
        default_value: String,
    }

    /// Description of a single algorithm input or output, returned by
    /// [`AlgorithmBridge::get_input_infos`] /
    /// [`AlgorithmBridge::get_output_infos`].
    pub struct InputOutputInfo {
        name: String,
        data_type: DataType,
        description: String,
    }

    // ===== C++ Bridge =====

    unsafe extern "C++" {
        include!("bridge/bridge.h");

        // ===== Core types =====
        // Each of these is an opaque C++ type — we can hold pointers to
        // it and call its methods, but the layout is hidden from Rust.

        /// FFI handle to a single Essentia algorithm instance.
        pub type AlgorithmBridge;

        /// FFI handle to an Essentia parameter map (a staging area for
        /// configuration values).
        pub type ParameterMapBridge;

        /// FFI handle to an Essentia [`Pool`](essentia_core::Pool) (a
        /// heterogeneous key/value store).
        pub type PoolBridge;

        /// FFI handle to a single Essentia value of any [`DataType`].
        pub type DataContainer;

        // ===== Essentia Initialisation =====

        /// Initialise the Essentia C++ runtime. **Must** be called
        /// exactly once before any other call. The Rust side wraps this
        /// in a refcounted RAII guard, so user code never calls it
        /// directly.
        pub fn init_essentia();

        /// Tear down the Essentia C++ runtime. **Must** be called once
        /// after the last algorithm has been dropped.
        pub fn shutdown_essentia();

        // ===== Algorithm Bridge Creation =====

        /// Return the names of every algorithm registered with the
        /// runtime.
        pub fn get_algorithm_names() -> Vec<String>;

        /// Construct a fresh algorithm by name. Errors if the name is
        /// unknown.
        pub fn create_algorithm_bridge(name: &str) -> Result<UniquePtr<AlgorithmBridge>>;

        // ===== Algorithm Bridge Introspection =====

        /// Algorithm name (typically the same string used to construct
        /// it).
        pub fn get_name(self: &AlgorithmBridge) -> String;
        /// Top-level category the algorithm belongs to.
        pub fn get_category(self: &AlgorithmBridge) -> String;
        /// Free-form English description.
        pub fn get_description(self: &AlgorithmBridge) -> String;
        /// Metadata for every declared parameter.
        pub fn get_parameter_infos(self: &AlgorithmBridge) -> Vec<ParameterInfo>;
        /// Metadata for every declared input.
        pub fn get_input_infos(self: &AlgorithmBridge) -> Vec<InputOutputInfo>;
        /// Metadata for every declared output.
        pub fn get_output_infos(self: &AlgorithmBridge) -> Vec<InputOutputInfo>;

        // ===== Algorithm Bridge Configuration & Execution =====

        /// Configure the algorithm with the given parameter map. This
        /// transitions Essentia's internal state into "ready to compute".
        pub fn configure(
            self: Pin<&mut AlgorithmBridge>,
            parameter_map_bridge: UniquePtr<ParameterMapBridge>,
        ) -> Result<()>;
        /// Run the algorithm against its currently-set inputs.
        pub fn compute(self: Pin<&mut AlgorithmBridge>) -> Result<()>;
        /// Discard any state accumulated across previous compute calls.
        pub fn reset(self: Pin<&mut AlgorithmBridge>) -> Result<()>;

        // ===== Algorithm Bridge Input/Output =====

        /// Provide an input value by name.
        pub fn set_input(
            self: Pin<&mut AlgorithmBridge>,
            input_name: &str,
            data_container: UniquePtr<DataContainer>,
        ) -> Result<()>;
        /// Tell Essentia what type to materialise an output as. Must be
        /// called before [`compute`](Self::compute) for every output.
        pub fn setup_output(
            self: Pin<&mut AlgorithmBridge>,
            output_name: &str,
            data_type: DataType,
        ) -> Result<()>;
        /// Read an output value by name. Returned reference is borrowed
        /// from the algorithm's internal buffer.
        pub fn get_output(self: &AlgorithmBridge, output_name: &str) -> Result<&DataContainer>;

        // ===== Data Container Constructors =====
        // One per supported payload shape. Each takes the natural C++/
        // bridge representation and wraps it in a fresh `DataContainer`.

        pub fn create_data_container_from_bool(value: bool) -> UniquePtr<DataContainer>;
        pub fn create_data_container_from_string(value: &str) -> UniquePtr<DataContainer>;
        pub fn create_data_container_from_float(value: f32) -> UniquePtr<DataContainer>;
        pub fn create_data_container_from_int(value: i32) -> UniquePtr<DataContainer>;
        pub fn create_data_container_from_unsigned_int(value: u32) -> UniquePtr<DataContainer>;
        pub fn create_data_container_from_long(value: i64) -> UniquePtr<DataContainer>;
        pub fn create_data_container_from_stereo_sample(
            value: StereoSample,
        ) -> UniquePtr<DataContainer>;
        pub fn create_data_container_from_complex(value: Complex) -> UniquePtr<DataContainer>;
        pub fn create_data_container_from_vector_bool(value: &[bool]) -> UniquePtr<DataContainer>;
        pub fn create_data_container_from_vector_int(value: &[i32]) -> UniquePtr<DataContainer>;
        pub fn create_data_container_from_vector_string(value: &[&str])
        -> UniquePtr<DataContainer>;
        pub fn create_data_container_from_vector_float(value: &[f32]) -> UniquePtr<DataContainer>;
        pub fn create_data_container_from_vector_stereo_sample(
            value: &[StereoSample],
        ) -> UniquePtr<DataContainer>;
        pub fn create_data_container_from_vector_complex(
            value: &[Complex],
        ) -> UniquePtr<DataContainer>;
        pub fn create_data_container_from_vector_vector_float(
            value: Vec<SliceFloat>,
        ) -> UniquePtr<DataContainer>;
        pub fn create_data_container_from_matrix_float(
            value: MatrixFloat,
        ) -> UniquePtr<DataContainer>;
        pub fn create_data_container_from_tensor_float(
            value: TensorFloat,
        ) -> UniquePtr<DataContainer>;
        pub fn create_data_container_from_vector_vector_string(
            value: Vec<VecString>,
        ) -> UniquePtr<DataContainer>;
        pub fn create_data_container_from_vector_vector_stereo_sample(
            value: Vec<SliceStereoSample>,
        ) -> UniquePtr<DataContainer>;
        pub fn create_data_container_from_vector_vector_complex(
            value: Vec<VecComplex>,
        ) -> UniquePtr<DataContainer>;
        pub fn create_data_container_from_vector_matrix_float(
            value: Vec<MatrixFloat>,
        ) -> UniquePtr<DataContainer>;
        pub fn create_data_container_from_map_vector_float(
            value: Vec<MapEntryVectorFloat>,
        ) -> UniquePtr<DataContainer>;
        pub fn create_data_container_from_map_vector_string(
            value: Vec<MapEntryVectorString>,
        ) -> UniquePtr<DataContainer>;
        pub fn create_data_container_from_map_vector_int(
            value: Vec<MapEntryVectorInt>,
        ) -> UniquePtr<DataContainer>;
        pub fn create_data_container_from_map_vector_complex(
            value: Vec<MapEntryVectorComplex>,
        ) -> UniquePtr<DataContainer>;
        pub fn create_data_container_from_map_float(
            value: Vec<MapEntryFloat>,
        ) -> UniquePtr<DataContainer>;
        pub fn create_data_container_from_pool(
            value: UniquePtr<PoolBridge>,
        ) -> UniquePtr<DataContainer>;

        // ===== Data Container Introspection =====

        /// Read the runtime tag stored in a container.
        pub fn get_data_type(self: &DataContainer) -> DataType;

        // ===== Data Container Accessors =====
        // One per supported payload shape. Each `Result<…>` returns an
        // error if the runtime tag doesn't match the requested type.
        // The Rust-side wrapper [`DataContainer<T>`](essentia_core::DataContainer)
        // makes those mismatches statically impossible, so the unwraps
        // in the conversion code are safe.

        pub fn get_bool(self: &DataContainer) -> Result<bool>;
        pub fn get_string(self: &DataContainer) -> Result<String>;
        pub fn get_float(self: &DataContainer) -> Result<f32>;
        pub fn get_int(self: &DataContainer) -> Result<i32>;
        pub fn get_unsigned_int(self: &DataContainer) -> Result<u32>;
        pub fn get_long(self: &DataContainer) -> Result<i64>;
        pub fn get_stereo_sample(self: &DataContainer) -> Result<StereoSample>;
        pub fn get_complex(self: &DataContainer) -> Result<Complex>;
        pub fn get_vector_bool(self: &DataContainer) -> Result<Vec<bool>>;
        pub fn get_vector_int(self: &DataContainer) -> Result<&[i32]>;
        pub fn get_vector_string(self: &DataContainer) -> Result<Vec<String>>;
        pub fn get_vector_float(self: &DataContainer) -> Result<&[f32]>;
        pub fn get_vector_stereo_sample(self: &DataContainer) -> Result<&[StereoSample]>;
        pub fn get_vector_complex(self: &DataContainer) -> Result<&[Complex]>;
        pub fn get_vector_vector_float(self: &DataContainer) -> Result<Vec<SliceFloat>>;
        pub fn get_matrix_float(self: &DataContainer) -> Result<MatrixFloat>;
        pub fn get_tensor_float(self: &DataContainer) -> Result<TensorFloat>;
        pub fn get_vector_vector_string(self: &DataContainer) -> Result<Vec<VecString>>;
        pub fn get_vector_vector_stereo_sample(
            self: &DataContainer,
        ) -> Result<Vec<SliceStereoSample>>;
        pub fn get_vector_vector_complex(self: &DataContainer) -> Result<Vec<VecComplex>>;
        pub fn get_vector_matrix_float(self: &DataContainer) -> Result<Vec<MatrixFloat>>;
        pub fn get_map_vector_float(self: &DataContainer) -> Result<Vec<MapEntryVectorFloat>>;
        pub fn get_map_vector_string(self: &DataContainer) -> Result<Vec<MapEntryVectorString>>;
        pub fn get_map_vector_int(self: &DataContainer) -> Result<Vec<MapEntryVectorInt>>;
        pub fn get_map_vector_complex(self: &DataContainer) -> Result<Vec<MapEntryVectorComplex>>;
        pub fn get_map_float(self: &DataContainer) -> Result<Vec<MapEntryFloat>>;
        pub fn get_pool(self: &DataContainer) -> &PoolBridge;

        // ===== Parameter Map Bridge =====

        /// Construct an empty parameter map.
        pub fn create_parameter_map_bridge() -> UniquePtr<ParameterMapBridge>;
        /// Insert a key/value pair into the parameter map. The value
        /// passes ownership of the underlying C++ container.
        pub fn add(
            self: Pin<&mut ParameterMapBridge>,
            key: &str,
            data_container: UniquePtr<DataContainer>,
        ) -> Result<()>;

        // ===== Pool Bridge =====

        /// Construct an empty pool.
        pub fn create_pool_bridge() -> UniquePtr<PoolBridge>;
        /// Make an independent copy of this pool. The C++ implementation
        /// is reference-counted, so this is cheaper than a deep copy.
        pub fn clone(self: &PoolBridge) -> UniquePtr<PoolBridge>;
        /// Set a value under a key. Replaces any existing value.
        pub fn set(self: Pin<&mut PoolBridge>, key: &str, data_container: UniquePtr<DataContainer>);
        /// Read a value out of the pool. Returns an owned container —
        /// the caller becomes responsible for the underlying C++ object.
        pub fn get(self: &PoolBridge, key: &str) -> Result<UniquePtr<DataContainer>>;
        /// `true` if the pool contains an entry for `key`.
        pub fn contains(self: &PoolBridge, key: &str) -> bool;
        /// Snapshot of every key currently stored in the pool.
        pub fn keys(self: &PoolBridge) -> Vec<String>;
    }
}
