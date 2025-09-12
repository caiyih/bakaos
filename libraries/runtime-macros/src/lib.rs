use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro_crate::{crate_name, FoundCrate};
use quote::{format_ident, quote};
use syn::{parse_macro_input, Error, ItemFn};

/// Attribute macro #[rust_main]
/// Generates a function named `main` that calls the user's original `main` function.
/// Allowing the same entry for both baremetal and std executables.
#[proc_macro_attribute]
pub fn rust_main(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // parse the annotated item as a function
    let input_fn = parse_macro_input!(item as ItemFn);
    let sig = &input_fn.sig;

    // Basic checks: no generics, not async, not const
    if !sig.generics.params.is_empty() {
        return Error::new_spanned(
            sig.generics.clone(),
            "rust_main: generic functions are not supported",
        )
        .to_compile_error()
        .into();
    }
    if sig.asyncness.is_some() {
        return Error::new_spanned(
            sig.asyncness,
            "rust_main: async functions are not supported (wrap in a sync function)",
        )
        .to_compile_error()
        .into();
    }
    if sig.constness.is_some() {
        return Error::new_spanned(sig.constness, "rust_main: main function cannot be const")
            .to_compile_error()
            .into();
    }

    // Requires function name to be `main`
    // TODO: should we allow other names?
    if sig.ident != "main" {
        return Error::new_spanned(
            sig.ident.clone(),
            "rust_main: function must be named 'main'",
        )
        .to_compile_error()
        .into();
    }

    let impl_ident = format_ident!("__rust_main_impl"); // Transformed function

    // take the original function but rename it to impl_ident and make it private
    // keep its visibility as-is but we will declare it non-mangled regular fn
    let vis = &input_fn.vis;
    let attrs = &input_fn.attrs;
    let block = &input_fn.block;
    let inputs = &input_fn.sig.inputs;
    let output = &input_fn.sig.output;

    // Require no arguments for simplicity; if you need arguments (argc/argv),
    // you can extend this macro to accept them and pass via runtime.
    if !inputs.is_empty() {
        return Error::new_spanned(
            inputs.clone(),
            "rust_main: function must have no parameters",
        )
        .to_compile_error()
        .into();
    }

    // Resolve the `runtime` crate path (handles dependency renames).
    let runtime_path: syn::Path = match crate_name("runtime") {
        Ok(FoundCrate::Itself) => syn::parse_quote!(crate),
        Ok(FoundCrate::Name(name)) => {
            let ident = syn::Ident::new(&name, Span::call_site());
            syn::parse_quote!(#ident)
        }
        Err(_) => syn::parse_quote!(runtime), // fallback
    };

    // Compose generated tokens:
    // 1) the renamed function with original body
    // 2) generated real main that calls runtime::rust_load_main
    let expanded = quote! {
        // keep user's attributes (except our attribute) on the implementation
        #(#attrs)*
        #vis fn #impl_ident() #output {
            #block
        }

        #[doc(hidden)]
        #[doc = "Generated entry point for rust standalone executable"]
        #[deprecated(note = concat!("Use `", stringify!(#impl_ident), "` instead"))]  
        #[inline(always)] // try to inline this function to `rust_main_entry`
        fn main() #output {
            // Delegate to runtime and return its value.
            #runtime_path::rust_load_main(#impl_ident)
        }

        #[doc(hidden)]
        mod __runtime_generated {
            /// generated entry for runtime crate to load the user's main
            #[unsafe(no_mangle)]
            extern "Rust" fn rust_main_entry() {
                #[allow(deprecated)]
                let _ = super::main();
            }
        }
    };

    TokenStream::from(expanded)
}
