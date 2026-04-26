//! Generation of parameter-setter methods on `Algorithm<Initialized>`.
//!
//! For each parameter declared in an algorithm's introspection, this
//! module produces:
//!
//! * A `pub fn parameter_name<T>(self, value: T) -> Self` builder method.
//! * A doc comment combining the parameter name with the description from
//!   introspection.
//! * (For string parameters constrained to a `OneOf` set) a sealed marker
//!   trait + a Rust enum whose variants are the allowed values, so that
//!   the type system rejects any other string at compile time.
//!
//! See the doc on [`generate_parameter_functions`] for the full output
//! shape.

use convert_case::{Case, Casing};
use essentia_core::{algorithm::{Constraint, ParameterInfo}, DataType, Introspection};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::algorithm_generation::common::{
    data_type_enum_to_data_type_marker, sanitize_identifier_string, string_to_doc_comment,
};

/// Build the doc comment for a single parameter's setter method.
fn generate_parameter_function_docs(parameter: &ParameterInfo) -> TokenStream {
    let name = parameter.name();
    let description = parameter.description();
    let doc = format!("Sets the `{}` parameter.\n\n{}", name, description);

    string_to_doc_comment(&doc)
}

/// Output of [`generate_constraint`] when the constraint can be expressed
/// at the type level.
pub struct ConstraintInfo {
    /// Identifier of the sealed marker trait we generated. Used as a bound
    /// on the parameter setter so only types implementing it can be
    /// passed.
    trait_ident: syn::Ident,
    /// Token stream of supporting code: the enum + its
    /// `IntoDataContainer<String>` impl + the sealed trait + the trait's
    /// impl on the enum.
    constraint_code: TokenStream,
}

/// Emit a Rust enum and a sealed marker trait for a string parameter
/// constrained to `OneOf(["a", "b", "c", …])`.
///
/// The naming scheme is `<Algorithm><Parameter>` for the enum (e.g.
/// `WindowingType`) and `<Algorithm><Parameter>Constraint` for the
/// sealed marker trait. Each enum variant `IntoDataContainer<String>`s
/// to the original (unmodified) Essentia string, so the C++ side sees
/// the exact value it's expecting.
fn generate_string_enum_constraint(algorithm_name: &str, parameter_name: &str, options: &[String]) -> ConstraintInfo {
    let algorithm_pascal = algorithm_name.trim().to_case(Case::Pascal);
    let parameter_pascal = parameter_name.to_case(Case::Pascal);
    let constraint_trait_ident = format_ident!("{}{}Constraint", algorithm_pascal, parameter_pascal);
    let enum_ident = format_ident!("{}{}", algorithm_pascal, parameter_pascal);

    let enum_variants: Vec<_> = options
        .iter()
        .map(|option| format_ident!("{}", option.to_case(Case::Pascal)))
        .collect();

    // Each variant maps back to the *original* string value so the C++
    // side recognises it. Without this, a re-cased variant identifier
    // (e.g. BlackmanHarris62) would no longer match Essentia's expected
    // input ("blackmanharris62").
    let string_conversions: Vec<_> = options
        .iter()
        .zip(&enum_variants)
        .map(|(original_name, variant_ident)| {
            quote! { #enum_ident::#variant_ident => #original_name }
        })
        .collect();

    let constraint_code = quote! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum #enum_ident {
            #(#enum_variants,)*
        }

        impl crate::data::IntoDataContainer<crate::data_type::String> for #enum_ident {
            fn into_data_container(self) -> crate::data::DataContainer<'static, crate::data_type::String> {
                let string_value = match self {
                    #(#string_conversions,)*
                };
                string_value.into_data_container()
            }
        }

        pub trait #constraint_trait_ident {}

        impl #constraint_trait_ident for #enum_ident {}
    };

    ConstraintInfo {
        trait_ident: constraint_trait_ident,
        constraint_code,
    }
}

/// Decide whether a parameter has a constraint expressible at the type
/// level.
///
/// Currently we only handle string `OneOf`. Future work could add ranged
/// numeric types, but in practice Essentia's range constraints are
/// typically validated by the C++ side at configure time.
pub fn generate_constraint(algorithm_name: &str, parameter: &ParameterInfo) -> Option<ConstraintInfo> {
    match (parameter.parameter_type(), parameter.constraint()) {
        (DataType::String, Constraint::OneOf(options)) => {
            Some(generate_string_enum_constraint(algorithm_name, parameter.name(), options))
        }
        _ => None,
    }
}

/// Bundle of token streams produced for an algorithm's parameter list.
pub struct ParameterFunctionResult {
    /// One [`TokenStream`] per parameter, each defining a setter method.
    /// These end up inside the `impl Algo<'a, Initialized> { … }` block.
    pub functions: Vec<TokenStream>,
    /// Supporting code for any constraint enums that had to be generated.
    /// Lives at module scope (above the algorithm struct).
    pub constraint_code: TokenStream,
}

/// Generate every parameter-setter method for the given algorithm.
///
/// Each method has the shape
///
/// ```ignore
/// pub fn parameter_name<T>(mut self, value: T) -> Self
/// where T: IntoDataContainer<crate::data_type::Foo> {
///     self.algorithm.set_parameter("parameterName", value).…;
///     self
/// }
/// ```
///
/// For string `OneOf` parameters, an extra trait bound is added so only
/// the generated enum can be passed.
///
/// The error branches in the body are statically unreachable because the
/// generic bound and the parameter-name string literal are both derived
/// from the same introspection that the runtime check uses — so any
/// mismatch would be a codegen bug, not a user error. Hence the explicit
/// `panic!`s.
pub fn generate_parameter_functions(algorithm_introspection: &Introspection) -> ParameterFunctionResult {
    let mut constraint_code_blocks = Vec::new();
    let algorithm_name = algorithm_introspection.name();

    let parameter_functions: Vec<TokenStream> = algorithm_introspection
        .parameters()
        .map(|parameter| {
            let parameter_name = parameter.name();
            let function_name = format_ident!("{}", sanitize_identifier_string(&parameter_name.to_case(Case::Snake)));
            let data_type_variant = data_type_enum_to_data_type_marker(&parameter.parameter_type());
            let doc_comment = generate_parameter_function_docs(parameter);

            // For constrained string parameters the bound is `IntoDataContainer<String> + <Algo><Param>Constraint`;
            // for everything else just the IntoDataContainer bound.
            let type_constraint = match generate_constraint(algorithm_name, parameter) {
                Some(constraint_info) => {
                    constraint_code_blocks.push(constraint_info.constraint_code);
                    let trait_ident = constraint_info.trait_ident;
                    quote! { crate::data::IntoDataContainer<#data_type_variant> + #trait_ident }
                }
                None => {
                    quote! { crate::data::IntoDataContainer<#data_type_variant> }
                }
            };

            quote! {
                #doc_comment
                pub fn #function_name<T>(mut self, value: T) -> Self
                where
                    T: #type_constraint
                {
                    match self.algorithm.set_parameter(#parameter_name, value) {
                        Ok(_) => {},
                        Err(essentia_core::algorithm::ParameterError::ParameterNotFound { parameter }) => {
                            panic!("Parameter '{}' not found after validation", parameter);
                        }
                        Err(essentia_core::algorithm::ParameterError::TypeMismatch { parameter, expected, actual }) => {
                            panic!("Type mismatch for parameter '{}': expected {:?}, found {:?}", parameter, expected, actual);
                        }
                    }
                    self
                }
            }
        })
        .collect();

    let constraint_code = quote! {
        #(#constraint_code_blocks)*
    };

    ParameterFunctionResult {
        functions: parameter_functions,
        constraint_code,
    }
}
