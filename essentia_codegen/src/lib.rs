//! # essentia-codegen
//!
//! Build-time code generator that emits one Rust module per Essentia
//! algorithm.
//!
//! ## Why this crate exists
//!
//! Essentia exposes hundreds of algorithms, each with its own set of
//! parameters, inputs and outputs. Maintaining hand-written Rust wrappers
//! for all of them would be infeasible — and would also drift out of date
//! every time Essentia is upgraded. Instead, this crate is invoked from the
//! [`essentia` crate's build script](https://docs.rs/essentia) and
//! programmatically generates a typed Rust API by *introspecting the C++
//! library at build time*.
//!
//! ## How it works
//!
//! 1. [`generate_code`] starts up Essentia (via `essentia_core::Essentia`)
//!    and asks it for the list of registered algorithm names.
//! 2. For each algorithm, it instantiates the C++ side, pulls the
//!    [`Introspection`](essentia_core::Introspection) (parameter types,
//!    input/output names, descriptions, constraints), and emits one Rust
//!    file under `<out_dir>/<category>/<algorithm>.rs` describing a typed
//!    builder for that algorithm.
//! 3. It then writes a per-category `mod.rs` listing every algorithm
//!    module in that category, and finally a top-level `mod.rs` listing
//!    every category.
//! 4. The `essentia` crate `include!`s the top-level `mod.rs`, exposing
//!    the whole tree as part of its public API.
//!
//! All generated code is formatted with `prettyplease` so that errors and
//! macro expansions remain readable.
//!
//! ## Crate layout
//!
//! * [`algorithm_generation`] — turns a single
//!   [`Introspection`](essentia_core::Introspection) into a Rust
//!   [`syn::File`].
//! * [`module_generation`] — produces the `mod.rs` glue files that wire
//!   the algorithm modules together.
//! * [`generate_code`] — the entry point used by `essentia/build.rs`.

mod algorithm_generation;
mod module_generation;

use algorithm_generation::{GeneratedAlgorithm, generate_algorithm_module_file};
use essentia_core::essentia::Essentia;
use std::collections::HashMap;
use std::path::Path;

use crate::module_generation::category_module::generate_category_module_file;
use crate::module_generation::main_module::generate_main_module_file;

/// Emit the per-category `mod.rs` files and the top-level `mod.rs` that ties
/// every category together.
///
/// The mapping from category to its algorithms is reconstructed from the
/// flat list of [`GeneratedAlgorithm`] entries returned by the algorithm
/// generation step.
fn generate_module_files(
    out_dir: &Path,
    generated_algorithms: &[GeneratedAlgorithm],
) -> std::io::Result<()> {
    // Bucket algorithms by their category so we know what to put in each
    // category's mod.rs.
    let mut categories: HashMap<String, Vec<String>> = HashMap::new();
    for result in generated_algorithms {
        categories
            .entry(result.category_module_name.clone())
            .or_default()
            .push(result.algorithm_module_name.clone());
    }

    // Sorted ordering keeps the generated output deterministic across
    // builds, which makes both diffs and debug output easier to read.
    let mut sorted_categories: Vec<String> = categories.keys().cloned().collect();
    sorted_categories.sort();

    for category in &sorted_categories {
        if let Some(algo_vec) = categories.get(category) {
            generate_category_module_file(out_dir, category, algo_vec)?;
        }
    }

    generate_main_module_file(out_dir, &sorted_categories)?;

    Ok(())
}

/// Generate the entire algorithm tree under `out_dir`.
///
/// Called from `essentia/build.rs` with `out_dir = $OUT_DIR/algorithms`.
/// On success, `out_dir` will contain:
///
/// ```text
/// out_dir/
/// ├── mod.rs                    -- one `pub mod <category>;` per category
/// ├── rhythm/
/// │   ├── mod.rs                -- one `pub mod <algo>;` + re-exports
/// │   ├── beat_tracker_degara.rs
/// │   ├── …
/// ├── spectral/
/// │   ├── mod.rs
/// │   ├── …
/// └── …
/// ```
///
/// Internally requires the C++ Essentia runtime to be loadable — the
/// generator instantiates every algorithm in turn to ask it for its
/// metadata.
pub fn generate_code(out_dir: &Path) -> std::io::Result<()> {
    let essentia = Essentia::new();

    // For each registered algorithm, instantiate it, grab its
    // introspection, and emit a Rust source file describing it. The
    // returned `GeneratedAlgorithm` records the module-path components
    // used (category and algorithm names) so we can wire up mod.rs files
    // in a second pass.
    let results: Vec<GeneratedAlgorithm> = essentia
        .available_algorithms()
        .map(|algorithm_name| {
            let algorithm = essentia.create_algorithm(algorithm_name).unwrap();
            let introspection = algorithm.introspection();

            generate_algorithm_module_file(introspection, out_dir)
        })
        .collect::<std::io::Result<_>>()?;

    generate_module_files(out_dir, &results)?;

    Ok(())
}
