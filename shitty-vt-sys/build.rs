//! Locates libshitty_vt and emits its link line.
//!
//! Shitty builds the facade with `./build a` (static) or `./build so`
//! (shared); either lands in the build root. Point `SHITTY_VT_LIB_DIR` at it.
//!
//! The static archive cannot bundle the system libraries libstd probed for
//! (xxhash/atomic/uring vary by host), so a static link needs those named
//! explicitly via `SHITTY_VT_LINK_LIBS`.

use std::env;

fn main() {
    for var in [
        "SHITTY_VT_LIB_DIR",
        "SHITTY_VT_STATIC",
        "SHITTY_VT_LINK_LIBS",
    ] {
        println!("cargo:rerun-if-env-changed={var}");
    }

    let Ok(lib_dir) = env::var("SHITTY_VT_LIB_DIR") else {
        panic!(
            "SHITTY_VT_LIB_DIR is unset.\n\
             Build the facade in a shitty checkout (`./build a` or `./build so`)\n\
             and point SHITTY_VT_LIB_DIR at its .build directory."
        );
    };
    println!("cargo:rustc-link-search=native={lib_dir}");

    let static_link = env::var("SHITTY_VT_STATIC").is_ok_and(|v| v != "0");
    if static_link {
        println!("cargo:rustc-link-lib=static=shitty_vt");
        // The C++ core needs its runtime; nothing else pulls it in.
        println!("cargo:rustc-link-lib=dylib=stdc++");
        for lib in env::var("SHITTY_VT_LINK_LIBS")
            .unwrap_or_default()
            .split(',')
            .filter(|l| !l.is_empty())
        {
            println!("cargo:rustc-link-lib=dylib={lib}");
        }
    } else {
        println!("cargo:rustc-link-lib=dylib=shitty_vt");
        // Keep the .so findable at run time without an install step.
        println!("cargo:rustc-link-arg=-Wl,-rpath,{lib_dir}");
    }
}
