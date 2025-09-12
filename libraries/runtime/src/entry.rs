pub use runtime_macros::rust_main;

/// This is a generic hook for us to do something before the user's main function is called.
#[doc(hidden)]
#[inline(always)]
pub fn rust_load_main<T, F: Fn() -> T>(main: F) -> T {
    // TODO

    main()
}
