//! Build script for `essentia-sys`.
//!
//! Two responsibilities:
//!
//! 1. **Compile the C++ bridge.** The `cxx::build::bridge` call reads the
//!    `#[cxx::bridge]` declarations in `src/lib.rs` and generates the
//!    matching C++ headers. We then add the hand-written `.cpp` files
//!    under `bridge/` to the build, set the C++ standard, and produce a
//!    `libessentia-bridge` static library.
//! 2. **Locate the native dependencies via `pkg-config`.** Essentia
//!    itself plus a long list of audio/IO libraries (FFmpeg, FFTW,
//!    libsamplerate, taglib, libchromaprint, …) must be discoverable via
//!    `pkg-config`. Their include and link paths are fed into both the
//!    bridge build and the final Rust link command.
//!
//! The list of required libraries is hard-coded below. Missing any of
//! them aborts the build with a clear error. TensorFlow can be
//! optionally disabled via the `USE_TENSORFLOW` environment variable
//! (set it to `0`/`false`/`no`/`off`).

/// Description of one external library that the bridge needs.
struct Library {
    /// Logical name used in error messages.
    pub name: String,
    /// Name passed to `pkg-config --libs <name>`.
    pub pkg_config_name: String,
    /// Optional `cargo:rustc-link-lib=…` argument. `None` for
    /// header-only libraries (e.g. `eigen3`).
    pub link_name: Option<String>,
}

impl Library {
    fn new(name: &str, pkg_config_name: &str, link_name: Option<&str>) -> Self {
        Self {
            name: name.to_string(),
            pkg_config_name: pkg_config_name.to_string(),
            link_name: link_name.map(|name| name.to_string()),
        }
    }
}

fn main() {
    // docs.rs builds in a sandbox without the native libraries, so we
    // skip the entire build there. The published documentation is
    // generated against an empty stub.
    if std::env::var("DOCS_RS").is_ok() {
        println!("cargo:warning=Skipping build.rs on docs.rs");
        return;
    }

    // `cxx::build::bridge` parses src/lib.rs, generates the C++ side of
    // the cxx bridge, and returns a `cc::Build` ready to compile.
    let mut build = cxx_build::bridge("src/lib.rs");
    build
        // The hand-written C++ implementations live under bridge/.
        .file("bridge/bridge.cpp")
        .file("bridge/algorithm_bridge/core.cpp")
        .file("bridge/algorithm_bridge/input_output.cpp")
        .file("bridge/algorithm_bridge/introspection.cpp")
        .file("bridge/parameter_map_bridge/parameter_map_bridge.cpp")
        .file("bridge/pool_bridge/pool_bridge.cpp")
        .file("bridge/data_container/accessors.cpp")
        .file("bridge/data_container/constructors.cpp")
        .file("bridge/data_container/introspection.cpp")
        .file("bridge/common/type_mapping.cpp")
        // `.` is added so that `#include "bridge/…"` style paths resolve
        // from the crate root.
        .include(".");

    // Every library Essentia needs at link time. The order doesn't
    // matter to pkg-config, but we keep TensorFlow last so it can be
    // conditionally appended.
    let mut libraries = vec![
        Library::new("essentia", "essentia", Some("essentia")),
        Library::new("eigen3", "eigen3", None),
        Library::new("yaml", "yaml-0.1", Some("yaml")),
        Library::new("fftw3f", "fftw3f", Some("fftw3f")),
        Library::new("taglib", "taglib", Some("tag")),
        Library::new("samplerate", "samplerate", Some("samplerate")),
        Library::new("chromaprint", "libchromaprint", Some("chromaprint")),
        Library::new("avformat", "libavformat", Some("avformat")),
        Library::new("swresample", "libswresample", Some("swresample")),
        Library::new("avcodec", "libavcodec", Some("avcodec")),
        Library::new("avutil", "libavutil", Some("avutil")),
    ];

    // TensorFlow is optional. Setting USE_TENSORFLOW=0 (or false/no/off)
    // skips the dependency at the cost of disabling the algorithms that
    // rely on it.
    let use_tensorflow = match std::env::var("USE_TENSORFLOW") {
        Ok(val) => !matches!(val.to_lowercase().as_str(), "0" | "false" | "no" | "off"),
        Err(_) => true,
    };

    if use_tensorflow {
        libraries.push(Library::new("tensorflow", "tensorflow", Some("tensorflow")))
    } else {
        println!("cargo:warning=Skipping tensorflow support as USE_TENSORFLOW=0");
    }

    for library in libraries {
        let pkg_info = match pkg_config::probe_library(&library.pkg_config_name) {
            Ok(pkg_info) => pkg_info,
            Err(err) => match library.name.as_str() {
                // TensorFlow gets a more helpful error message because
                // it's the most common reason for new contributors to
                // hit a build failure.
                "tensorflow" => {
                    println!("cargo:error=Failed to find tensorflow: {}", err);
                    println!(
                        "cargo:error=If you intend to use essentia without tensorflow, set USE_TENSORFLOW=0"
                    );
                    std::process::exit(1);
                }
                _ => {
                    println!(
                        "cargo:error=Failed to find required library '{}': {}",
                        library.pkg_config_name, err
                    );
                    println!(
                        "cargo:error=Please install the library or check your pkg-config setup"
                    );
                    std::process::exit(1);
                }
            },
        };

        println!("{:?}", pkg_info);

        // Add include paths to the cxx_build invocation so the C++
        // sources can `#include` them.
        for mut include_path in pkg_info.include_paths {
            // Eigen is unusual: pkg-config reports its include dir, but
            // its headers are nested one level deeper than that.
            if library.name == "eigen3" {
                include_path.push("eigen3");
            }

            build.include(include_path);
        }

        // Forward link search paths and link names to cargo.
        for link_path in &pkg_info.link_paths {
            println!(
                "cargo:rustc-link-search=native={}",
                link_path.to_string_lossy()
            );
        }

        if let Some(link_name) = &library.link_name {
            println!("cargo:rustc-link-lib={}", link_name);
        }
    }

    // C++17 is the minimum because both Essentia and the cxx bridge use
    // features (structured bindings, std::variant, …) that require it.
    build.flag("-std=c++17").compile("essentia-bridge");

    println!("cargo:rerun-if-changed=bridge");
    println!("cargo:rerun-if-changed=src/lib.rs");
}
