// SPDX-License-Identifier: GPL-3.0-or-later
fn main() {
    #[cfg(windows)]
    if std::env::var_os("CARGO_FEATURE_LOCAL_LLM").is_some() {
        link_llama_helpers();
    }
    tauri_build::build()
}

/// llama-cpp-sys-4 0.10.0 builds `cpp-httplib` + `vendor-hash` but never links them on Windows
/// (LNK1120). Compile both from the crate's vendored sources with llama.cpp's CMake defines. Remove when the crate
/// links them itself.
#[cfg(windows)]
fn link_llama_helpers() {
    let home = std::env::var("CARGO_HOME").map(std::path::PathBuf::from).unwrap_or_else(|_| {
        std::path::PathBuf::from(std::env::var("USERPROFILE").expect("USERPROFILE")).join(".cargo")
    });
    let vendor = std::fs::read_dir(home.join("registry").join("src"))
        .expect("cargo registry")
        .flatten()
        .map(|d| d.path().join("llama-cpp-sys-4-0.10.0").join("llama.cpp").join("vendor"))
        .find(|p| p.exists())
        .expect("llama-cpp-sys-4 0.10.0 sources not found in the cargo registry (run cargo fetch)");
    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .file(vendor.join("cpp-httplib").join("httplib.cpp"))
        .include(vendor.join("cpp-httplib"))
        .define("CPPHTTPLIB_FORM_URL_ENCODED_PAYLOAD_MAX_LENGTH", "1048576")
        .define("CPPHTTPLIB_LISTEN_BACKLOG", "512")
        .define("CPPHTTPLIB_REQUEST_URI_MAX_LENGTH", "32768")
        .define("CPPHTTPLIB_TCP_NODELAY", "1")
        .warnings(false)
        .compile("fo-cpp-httplib");
    let hash = vendor.join("hash");
    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .flag("/TP")
        .files(["hash.cpp", "sha1/sha1.c"].iter().map(|f| hash.join(f)))
        .include(&hash)
        .warnings(false)
        .compile("fo-vendor-hash-cpp");
    cc::Build::new()
        .files(["xxhash/xxhash.c", "sha256/sha256.c"].iter().map(|f| hash.join(f)))
        .include(&hash)
        .warnings(false)
        .compile("fo-vendor-hash-c");
    println!("cargo:rustc-link-lib=ws2_32");
    println!("cargo:rustc-link-lib=crypt32");
}
