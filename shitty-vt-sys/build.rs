//! Locates libshitty_vt and emits its link line.
//!
//! Preferred: a release tree from shitty's `./build tgz`, which carries a
//! pkg-config file whose `Libs.private` records exactly what the build
//! probed for on that host. Point `PKG_CONFIG_PATH` at its `lib/pkgconfig`.
//!
//! Fallback: `SHITTY_VT_LIB_DIR` pointing straight at a build directory, for
//! working against a checkout without packaging it first. That path cannot
//! know the probed backends, so a static link there needs them named through
//! `SHITTY_VT_LINK_LIBS`.

use std::env;

fn main() {
    for var in [
        "SHITTY_VT_LIB_DIR",
        "SHITTY_VT_STATIC",
        "SHITTY_VT_LINK_LIBS",
    ] {
        println!("cargo:rerun-if-env-changed={var}");
    }

    // docs.rs has no libshitty_vt and does not link anything: rustdoc only
    // needs the crate to compile. Emitting no link line keeps the docs
    // build from failing on a library it could never have.
    if env::var_os("DOCS_RS").is_some() {
        return;
    }

    let static_link = env::var("SHITTY_VT_STATIC").is_ok_and(|v| v != "0");

    if let Ok(lib_dir) = env::var("SHITTY_VT_LIB_DIR") {
        link_from_directory(&lib_dir, static_link);
        return;
    }

    match pkg_config::Config::new()
        .statik(static_link)
        .probe("shitty_vt")
    {
        Ok(_) => {
            // The .pc deliberately does not guess between libstdc++ and
            // libc++, so a static link still needs a C++ runtime named here.
            if static_link {
                println!("cargo:rustc-link-lib=dylib={}", cxx_runtime());
            }
        }
        Err(error) => panic!(
            "could not find shitty_vt.

Build a release tree in a shitty checkout and point pkg-config at it:

    ./build tgz
    tar xzf .build/shitty_vt.tgz
    export PKG_CONFIG_PATH=$PWD/shitty_vt-<version>/lib/pkgconfig

Or set SHITTY_VT_LIB_DIR to a build directory directly.

pkg-config said: {error}"
        ),
    }
}

/// Links against a build directory, where nothing records what the build
/// probed for.
fn link_from_directory(lib_dir: &str, static_link: bool) {
    println!("cargo:rustc-link-search=native={lib_dir}");
    if static_link {
        println!("cargo:rustc-link-lib=static=shitty_vt");
        println!("cargo:rustc-link-lib=dylib={}", cxx_runtime());
        for lib in env::var("SHITTY_VT_LINK_LIBS")
            .unwrap_or_default()
            .split(',')
            .filter(|lib| !lib.is_empty())
        {
            println!("cargo:rustc-link-lib=dylib={lib}");
        }
    } else {
        println!("cargo:rustc-link-lib=dylib=shitty_vt");
        // Keep the .so findable at run time without an install step.
        println!("cargo:rustc-link-arg=-Wl,-rpath,{lib_dir}");
    }
}

fn cxx_runtime() -> &'static str {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        "c++"
    } else {
        "stdc++"
    }
}
