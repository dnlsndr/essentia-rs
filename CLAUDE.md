# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & test

`cargo build`, `cargo test --all`, and `cargo doc --no-deps --all-features --workspace` all require the Essentia C++ library and its transitive dependencies to be discoverable through `pkg-config`. The full list lives in `essentia_sys/build.rs`: `essentia`, `eigen3`, `yaml-0.1`, `fftw3f`, `taglib`, `samplerate`, `libchromaprint`, `libavformat`, `libswresample`, `libavcodec`, `libavutil`, plus `tensorflow` by default. CI runs inside the `lagmoellertim/essentia:latest-tensorflow` Docker image — **that is the canonical build environment for this repo**. On hosts without the native libs every build will fail at the link step (the `essentia_sys` library is pulled in transitively by `essentia/build.rs` via `essentia_codegen → essentia_core → essentia_sys`).

Set `USE_TENSORFLOW=0` to skip the TensorFlow dependency. `DOCS_RS=1` short-circuits both `build.rs` files (used by docs.rs); on this path `essentia/build.rs` writes a stub `generated/mod.rs` so the rest of the crate still compiles.

There are no unit tests in the workspace yet, so `cargo test --all` mainly compiles every crate (which is the real cost — `essentia_sys` rebuilds the C++ bridge, `essentia` regenerates ~hundreds of algorithm modules).

To run the release workflow locally, `cargo ws version --no-git-commit --force "*" --exact --yes <patch|minor|major>` mirrors what CI does. Publish order is **strict**: `essentia-sys` → `essentia-core` → `essentia-codegen` → `essentia`, and the final `essentia` publish uses `--no-verify` because the code it would otherwise verify is generated into the local `src/` tree at build time.

## The generated algorithm tree

`essentia/src/algorithm/generated/` holds one `<category>/<algorithm>.rs` per Essentia algorithm. The directory is **gitignored** but its path is stable across `cargo clean`s, so `rust-analyzer`/`cargo doc`/etc. resolve it like any normal `mod`.

Two ways to (re)generate it:

* **Implicit, on every build** — `essentia/build.rs` calls `essentia_codegen::generate_code` and writes into `<crate>/src/algorithm/generated/`. This wipes and recreates the directory, so removed algorithms (e.g. after an Essentia upgrade) don't leave stale files.
* **Explicit, on demand** — `cargo run -p essentia-codegen [target_dir]` runs the same codegen without going through the build script. Useful for inspecting the output (point it at `/tmp/foo`) or forcing a regen without rebuilding the whole world. The default target, when no arg is given, is the same `essentia/src/algorithm/generated/` path used by the build script.

Both invocations require Essentia to be loadable at runtime — the codegen literally instantiates each C++ algorithm to read its introspection.

**Do not hand-edit anything under `essentia/src/algorithm/generated/`.** It is overwritten on every build. Edits to the *generator* belong in `essentia_codegen/src/algorithm_generation/`.

## Workspace layout — four crates, one pipeline

The four crates form a strict bottom-up stack. Touching a lower crate forces a rebuild of the ones above it, including the codegen step.

1. **`essentia_sys`** — Pure FFI. `src/lib.rs` is a `#[cxx::bridge]` module declaring every C++ type and free function; the implementations live in `bridge/*.cpp` and are compiled in by `build.rs`. The C++ side is organised per-concept (`algorithm_bridge/`, `data_container/`, `parameter_map_bridge/`, `pool_bridge/`, `common/type_mapping.*`). All cross-language data types are reduced to a tagged `DataContainer` plus a `DataType` enum — there is no per-type FFI surface.

2. **`essentia_core`** — Idiomatic Rust over the FFI, but **generic over algorithms**. The interesting pieces:
   - `essentia/essentia.rs` — `Essentia` is a refcounted handle around a global `EssentiaLifecycle` (stored as a `Mutex<Weak<…>>`). `init_essentia` is called when the first handle is created; `shutdown_essentia` runs in `Drop` once the last handle goes away. **Do not call `ffi::init_essentia` / `ffi::shutdown_essentia` directly** — go through `Essentia::new()` so the refcount stays consistent. The set of available algorithm names is cached in a `Lazy<HashSet<String>>` after the first init.
   - `algorithm/algorithm.rs` — `Algorithm<'a, State>` is a typestate machine: `Initialized` (parameters being set) → `Configured` (inputs/compute). State transitions consume `self`. The `'a` lifetime ties the algorithm to the `Essentia` handle that created it.
   - `data/types.rs` — A `DataType` enum (runtime tag) plus zero-sized marker structs in `data_type::*` (compile-time tag) wired together by `trait HasDataType { const DATA_TYPE: DataType; }`.
   - `data/constraints.rs` — Capability traits `ParameterData`, `InputOutputData`, `PoolData` restrict which markers are valid where. Essentia allows different sets of types in each role, so these traits enforce that asymmetry at compile time.
   - `data/container.rs` — `DataContainer<'a, T>` wraps an FFI container with a phantom `T` marker. Internally `Owned(UniquePtr<…>)` or `Borrowed(&…)`; `into_owned_ptr` deep-copies if borrowed (see `copy_to_owned` — every supported `DataType` must be branched there).
   - `data/conversion_into.rs` / `conversion_get.rs` — `IntoDataContainer<T>` / `GetFromDataContainer<T>` are the user-facing conversion traits between idiomatic Rust types (`f32`, `&[f32]`, `ndarray::Array2/Array4`, `num::Complex`, `HashMap<String, …>`, etc.) and the typed container.
   - Type checks happen **twice**: `set_parameter` / `set_input` / `output` first verify against introspection (returns a typed error on mismatch), then static `HasDataType` ensures the user's `T` is correct. The generated `essentia` crate exploits this by panicking on the introspection-failure branches — they are statically unreachable from the typed builder API.

3. **`essentia_codegen`** — Library + binary, used both by `essentia/build.rs` and by hand via `cargo run -p essentia-codegen`. `generate_code(out_dir)` instantiates `essentia_core::Essentia`, walks every algorithm via `available_algorithms() / create_algorithm`, calls `introspection()`, and emits one `.rs` file per algorithm under `<out_dir>/<category>/<algo>.rs`. It also writes per-category `mod.rs` files and a top-level `mod.rs`. The `essentia` crate consumes that tree as a regular `mod generated;` declaration in `src/algorithm/mod.rs` (pointing at `essentia/src/algorithm/generated/`). Categories and algorithm names come straight from C++ Essentia and are converted with `convert_case` (Pascal for types, snake for modules/methods); `common.rs::sanitize_identifier_string` appends `_` to Rust keywords.

4. **`essentia`** — The published, user-facing crate. It re-exports `Essentia`, `DataContainer`, the `data_type::*` markers, `Pool`, etc. from `essentia_core`, and adds the generated, per-algorithm builder structs. Each generated algorithm follows the same shape: `Algo<'a, Initialized>` with one `param_name(value)` method per parameter (typed via `IntoDataContainer<data_type::Foo>`), `.configure() -> Result<Algo<'a, Configured>, …>`, then `.compute(input1, input2, …) -> Result<AlgoResult<'…>, …>` whose methods return `DataContainer<'…, data_type::Foo>` for each output. String parameters constrained by Essentia to a `{a,b,c}` set get a generated enum + sealed marker trait so only valid values type-check (see `parameter_functions.rs::generate_string_enum_constraint`).

## When changing things

- **Adding a new Essentia data type** is cross-cutting: you need to extend the `cxx::bridge` enum + functions in `essentia_sys/src/lib.rs`, the matching C++ in `essentia_sys/bridge/`, the `DataType` enum + `data_type::*` marker + `HasDataType` impl in `essentia_core/src/data/types.rs`, the capability impls in `constraints.rs`, the relevant arms in `container.rs::copy_to_owned`, the conversion traits in `conversion_into.rs` / `conversion_get.rs`, and finally a match arm in `essentia_codegen/src/algorithm_generation/common.rs::data_type_enum_to_data_type_marker`. Compilation will fail loudly at each missing site.
- **Do not edit files under `essentia/src/algorithm/generated/`** — the directory is wiped and rewritten on every `cargo build` (and on every `cargo run -p essentia-codegen`). Edits to the codegen itself live in `essentia_codegen/src/algorithm_generation/`.
- **Algorithm builders panic on introspection mismatches** (parameter/input/output not found, type mismatch). Those branches are unreachable from the generated typed API and indicate either a stale codegen output or a bug in the codegen — not a user error. Don't paper over them with `?`.
- The Rust API uses snake_case for parameter and input/output names but `set_parameter` / `set_input` / `output` always pass the **original** Essentia names (see how the codegen captures `parameter.name()` as a string literal alongside the snake_case method name).
- `essentia/build.rs` calls into `essentia_codegen::generate_code`, which itself loads C++ Essentia. That means a clean build of `essentia` requires the same system libraries as `essentia_sys` even though `essentia` doesn't link them directly. The same applies to running `cargo run -p essentia-codegen` by hand.
