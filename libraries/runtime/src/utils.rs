pub use ::utilities::*; // reexport `utilities`` crate

#[macro_export]
macro_rules! symbol_addr {
    ($sym:ident) => {{
        unsafe extern "C" {
            #[allow(improper_ctypes)]
            static $sym: ();
        }

        ::core::ptr::addr_of!($sym)
    }};
}
