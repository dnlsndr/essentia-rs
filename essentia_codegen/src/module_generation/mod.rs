//! Generation of the `mod.rs` glue files that knit the per-algorithm
//! source files together into a navigable module tree.
//!
//! After `algorithm_generation` has produced one `.rs` file per algorithm
//! under `<out_dir>/<category>/<algorithm>.rs`, this layer adds:
//!
//! * One `mod.rs` per category — declaring each algorithm's submodule and
//!   re-exporting its public items, so users can write
//!   `essentia::algorithm::rhythm::BeatTrackerDegara` without going through
//!   the per-algorithm sub-path.
//! * One top-level `mod.rs` listing every category, included via
//!   `include!` from the user-facing `essentia` crate.

pub mod category_module;
pub mod main_module;
