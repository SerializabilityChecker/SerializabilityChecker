//! Code that uses the ISL library.

/// Based on https://rust-lang.github.io/rust-bindgen/tutorial-4.html
#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
pub mod bindings {
    include!(concat!(env!("OUT_DIR"), "/isl_bindings.rs"));
}
pub use bindings::*;

/// Get the (thread-local, unique) ISL ctx.
///
/// This is preferred over manually calling isl_ctx_alloc() to make sure there's only one isl_ctx.
pub fn get_ctx() -> *mut isl_ctx {
    thread_local! {
        static ISL_CTX: *mut isl_ctx = unsafe { isl_ctx_alloc() };
    }
    ISL_CTX.with(|ctx| *ctx)
}
