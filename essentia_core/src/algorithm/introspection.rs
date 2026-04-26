//! Runtime introspection of an Essentia algorithm.
//!
//! Every Essentia algorithm exposes a description of itself — its name,
//! category (e.g. `"Rhythm"`, `"Spectral"`), free-form description, and the
//! list of parameters and inputs/outputs along with their types and
//! constraints. This module captures that snapshot in plain Rust types so
//! that:
//!
//! * The build-time [`essentia_codegen`](https://docs.rs/essentia-codegen)
//!   can read the metadata and emit one Rust struct per algorithm.
//! * The runtime [`Algorithm`](super::Algorithm) can validate user-supplied
//!   parameter/input/output names and types without round-tripping through
//!   the FFI on every call.
//!
//! [`Introspection`] is built once when an algorithm is created and stays
//! immutable for the algorithm's lifetime.

use std::collections::HashMap;

use essentia_sys::ffi;

use crate::data::DataType;

/// Snapshot of an algorithm's metadata, indexed by name for cheap lookup.
///
/// Built once from a freshly-constructed [`ffi::AlgorithmBridge`] and shared
/// for the algorithm's whole lifetime via
/// [`Algorithm::introspection`](super::Algorithm::introspection).
#[derive(Debug, Clone)]
pub struct Introspection {
    /// Algorithm name as registered on the C++ side (e.g. `"BeatTrackerDegara"`).
    name: String,
    /// Top-level category the algorithm belongs to (e.g. `"Rhythm"`,
    /// `"Spectral"`). Used by the codegen to organise algorithms into
    /// per-category sub-modules.
    category: String,
    /// Free-form English description, used as the algorithm's Rust doc
    /// comment in the generated code.
    description: String,
    /// All inputs, keyed by name.
    input_infos: HashMap<String, InputOutputInfo>,
    /// All outputs, keyed by name.
    output_infos: HashMap<String, InputOutputInfo>,
    /// All parameters, keyed by name.
    parameter_infos: HashMap<String, ParameterInfo>,
}

impl Introspection {
    /// Build an [`Introspection`] from a live FFI algorithm bridge.
    ///
    /// Calls the various `get_*_infos` accessors on the C++ bridge once,
    /// then stashes the results in HashMaps for O(1) lookup by name.
    pub fn from_algorithm_bridge(algorithm_bridge: &ffi::AlgorithmBridge) -> Self {
        let input_info = algorithm_bridge
            .get_input_infos()
            .into_iter()
            .map(|info| {
                let info: InputOutputInfo = info.into();
                (info.name.clone(), info)
            })
            .collect();

        let output_info = algorithm_bridge
            .get_output_infos()
            .into_iter()
            .map(|info| {
                let info: InputOutputInfo = info.into();
                (info.name.clone(), info)
            })
            .collect();

        let parameter_info = algorithm_bridge
            .get_parameter_infos()
            .into_iter()
            .map(|info| {
                let info: ParameterInfo = info.into();
                (info.name.clone(), info)
            })
            .collect();

        Self {
            name: algorithm_bridge.get_name(),
            category: algorithm_bridge.get_category(),
            description: algorithm_bridge.get_description(),
            input_infos: input_info,
            output_infos: output_info,
            parameter_infos: parameter_info,
        }
    }

    /// Algorithm name (e.g. `"BeatTrackerDegara"`).
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Algorithm category (e.g. `"Rhythm"`).
    pub fn category(&self) -> &str {
        &self.category
    }
    /// Free-form English description.
    pub fn description(&self) -> &str {
        &self.description
    }
    /// Iterate over every declared input.
    pub fn inputs(&self) -> impl Iterator<Item = &InputOutputInfo> {
        self.input_infos.values()
    }
    /// Iterate over every declared output.
    pub fn outputs(&self) -> impl Iterator<Item = &InputOutputInfo> {
        self.output_infos.values()
    }
    /// Iterate over every declared parameter.
    pub fn parameters(&self) -> impl Iterator<Item = &ParameterInfo> {
        self.parameter_infos.values()
    }

    /// Look up a parameter by name. Returns `None` if no such parameter
    /// exists.
    pub fn get_parameter(&self, name: &str) -> Option<&ParameterInfo> {
        self.parameter_infos.get(name)
    }
    /// Look up an input by name. Returns `None` if no such input exists.
    pub fn get_input(&self, name: &str) -> Option<&InputOutputInfo> {
        self.input_infos.get(name)
    }
    /// Look up an output by name. Returns `None` if no such output exists.
    pub fn get_output(&self, name: &str) -> Option<&InputOutputInfo> {
        self.output_infos.get(name)
    }
}

/// Description of a single input or output of an algorithm.
///
/// Inputs and outputs share the same shape — only the direction of data
/// flow differs — so they're represented by one struct.
#[derive(Debug, Clone)]
pub struct InputOutputInfo {
    /// Input/output name as the C++ side knows it (e.g. `"signal"`,
    /// `"frame"`, `"beats"`).
    name: String,
    /// Declared payload type.
    data_type: DataType,
    /// Free-form English description.
    description: String,
}

impl InputOutputInfo {
    /// The name as Essentia knows it. The codegen converts this to
    /// snake_case for the Rust API but always stores the original here.
    pub fn name(&self) -> &str {
        &self.name
    }
    /// The declared payload type.
    pub fn input_output_type(&self) -> DataType {
        self.data_type
    }
    /// Free-form English description.
    pub fn description(&self) -> &str {
        &self.description
    }
}

impl From<ffi::InputOutputInfo> for InputOutputInfo {
    fn from(value: ffi::InputOutputInfo) -> Self {
        let data_type = DataType::from(value.data_type);

        InputOutputInfo {
            name: value.name,
            data_type,
            description: value.description,
        }
    }
}

/// Parsed form of an Essentia parameter constraint string.
///
/// On the C++ side every parameter has a free-form constraint string such
/// as `"[0,1]"`, `"(0,inf)"`, or `"{hann,hamming,blackmanharris62,…}"`. This
/// enum captures the common shapes; anything that doesn't fit a known
/// pattern is preserved verbatim as [`Constraint::Custom`].
#[derive(Debug, Clone, PartialEq)]
pub enum Constraint {
    /// No constraint declared (empty string on the C++ side).
    Any,
    /// Real value strictly greater than zero (`"(0,inf)"`).
    PositiveReal,
    /// Real value greater than or equal to zero (`"[0,inf)"`).
    NonNegativeReal,
    /// Integer in a closed range (`"[min,max]"`).
    IntRange {
        /// Inclusive lower bound.
        min: i32,
        /// Inclusive upper bound.
        max: i32,
    },
    /// Integer ≥ 0. (Currently never emitted by [`Constraint::from`] but
    /// kept available for users that want to construct it manually.)
    NonNegativeInt,
    /// Integer > 0. (Currently never emitted by [`Constraint::from`].)
    PositiveInt,
    /// One of a fixed set of string values (`"{hann,hamming,…}"`).
    ///
    /// String parameters with this constraint get a generated Rust enum
    /// in the user-facing crate so the choices are type-checked.
    OneOf(Vec<String>),
    /// Anything not recognised by the parser. The original constraint
    /// string is preserved.
    Custom(String),
}

impl From<&str> for Constraint {
    fn from(s: &str) -> Self {
        if s.is_empty() {
            return Constraint::Any;
        }
        match s {
            "(0,inf)" => Constraint::PositiveReal,
            "[0,inf)" => Constraint::NonNegativeReal,
            // `{a,b,c}` → OneOf
            s if s.starts_with('{') && s.ends_with('}') => Self::parse_one_of_constraint(s),
            // `[min,max]` → IntRange (or Custom if the bounds aren't integers)
            s if s.starts_with('[') && s.ends_with(']') => {
                Self::parse_int_range_constraint(s).unwrap_or_else(|| Self::Custom(s.to_string()))
            }
            _ => Constraint::Custom(s.to_string()),
        }
    }
}

impl Constraint {
    /// Parse `"{a,b,c}"` into a [`Constraint::OneOf`]. Whitespace around the
    /// values is trimmed.
    fn parse_one_of_constraint(s: &str) -> Self {
        let inner = &s[1..s.len() - 1];
        let values = inner.split(',').map(|v| v.trim().to_string()).collect();
        Self::OneOf(values)
    }
    /// Parse `"[min,max]"` into a [`Constraint::IntRange`]. Returns `None`
    /// if either bound fails to parse as `i32`.
    fn parse_int_range_constraint(s: &str) -> Option<Self> {
        let inner = &s[1..s.len() - 1];
        let (min_str, max_str) = inner.split_once(',')?;
        let min = min_str.trim().parse::<i32>().ok()?;
        let max = max_str.trim().parse::<i32>().ok()?;
        Some(Self::IntRange { min, max })
    }
}

/// Description of a single parameter of an algorithm.
#[derive(Debug, Clone)]
pub struct ParameterInfo {
    /// Parameter name as the C++ side knows it (e.g. `"sampleRate"`).
    name: String,
    /// Declared payload type.
    data_type: DataType,
    /// Free-form English description.
    description: String,
    /// Parsed constraint string (range, allowed values, …).
    constraint: Constraint,
    /// Default value as a string. Empty when no default is declared, in
    /// which case the parameter must be set explicitly. See
    /// [`Self::optional`].
    default_value: String,
}

impl ParameterInfo {
    /// The name as Essentia knows it.
    pub fn name(&self) -> &str {
        &self.name
    }
    /// The declared payload type.
    pub fn parameter_type(&self) -> DataType {
        self.data_type
    }
    /// Free-form English description.
    pub fn description(&self) -> &str {
        &self.description
    }
    /// Parsed constraint string. See [`Constraint`].
    pub fn constraint(&self) -> &Constraint {
        &self.constraint
    }
    /// Default value as a string, or empty string if the parameter is
    /// required.
    pub fn default_value(&self) -> &str {
        &self.default_value
    }
    /// `true` if the parameter has a default and may be omitted.
    pub fn optional(&self) -> bool {
        !self.default_value.is_empty()
    }
}

impl From<ffi::ParameterInfo> for ParameterInfo {
    fn from(value: ffi::ParameterInfo) -> Self {
        let data_type = DataType::from(value.data_type);

        ParameterInfo {
            name: value.name,
            data_type,
            description: value.description,
            constraint: Constraint::from(value.constraint.as_str()),
            default_value: value.default_value,
        }
    }
}
