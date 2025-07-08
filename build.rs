use std::env;
use std::path::PathBuf;

fn main() {
    // ... (ISL_PREFIX, link-search, link-lib setup as before) ...
    let isl_prefix_str = env::var("ISL_PREFIX").unwrap_or_else(|_| "/usr".to_string());
    let isl_prefix = PathBuf::from(&isl_prefix_str);
    let include_path = isl_prefix.join("include");

    println!(
        "cargo:rustc-link-search={}",
        isl_prefix.join("lib").display()
    );
    println!("cargo:rustc-link-lib=isl");
    println!("cargo:rerun-if-changed=src/isl_wrapper.h");
    println!("cargo:rerun-if-changed=src/isl_helpers.c"); // Rerun if C code changes

    // --- Compile C helper file ---
    cc::Build::new()
        .file("src/isl_helpers.c") // Your C helper file
        .include(&include_path) // Tell C compiler where ISL headers are
        .compile("isl_helpers"); // Resulting static lib name (libisl_helpers.a)
    // --- End C compilation ---

    let bindings = bindgen::Builder::default()
        .header("src/isl_wrapper.h")
        .clang_arg(format!("-I{}", include_path.display()))
        .allowlist_type("isl_.*")
        .allowlist_function("isl_.*")
        // --- Allowlist your C helper function(s) ---
        .allowlist_function("rust_harmonize_sets") // Original function
        .allowlist_function("rust_harmonize_sets_with_mapping") // New improved function
        // --- End allowlist ---
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    // ... (write bindings file) ...
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("isl_bindings.rs"))
        .expect("Couldn't write bindings!");
}
