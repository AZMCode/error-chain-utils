//! Utilities for `error-chain`
//!
//! This crate contains macros for use with the `error-chain` crate, and by
//! extension depends on the version of the crate these macros are designed for
//!
//! So far, these utilities are available:
//!
//! - `error_chain_quick`: Extension for convenience to the `error-chain` crate
//!

use proc_macro;
use error_chain_utils_lib::quick::main as ecq_main;


/// Utility that expands to `error_chain!{...}`
///
/// This macro can be used to replace `error_chain!` blocks, with the exact same
/// syntax, except that within one can use the `quick!` macro inside the 
/// `errors` block, to make writing error chains much faster.
///
/// // Example of input
/// ```
/// #[allow(unused_imports)]
/// use error_chain_utils::error_chain_quick;
/// error_chain_quick!{
///     types {
///         CustomError, CustomErrorEnum, CustomErrorTrait, CustomErrorResult;
///     }
/// 
///     errors {
///         quick!(ErrWithoutArgs, "Error Without Arguments")
///         quick!(ErrWithArgs,     "Error With Arguments",  (arg1,arg2))
///     }
/// };
/// ```
/// 
/// // Which would be processed into the following
/// ```
/// use error_chain::error_chain;
/// error_chain!{
///     types {
///         CustomError, CustomErrorEnum, CustomErrorTrait, CustomErrorResult;
///     }
/// 
///     errors {
///         ErrWithoutArgs {
///             description("Error Without Arguments")
///             display("Error Without Arguments")
///         }
///         ErrWithArgs (arg1: String, arg2: String){
///             description("Error With Arguments")
///             display("Error With Arguments: {}, {}", arg1, arg2)
///         }
///     }
/// }
/// ```
/// 
/// Trailing commas are supported inside of the `quick!` macro, and wherever else
/// `error_chain!` supports them.
/// 
/// Normal errors and `quick!` macro errors are supported in the same `errors` block.
/// 
/// Probably due to the double-expansion needed to make this happen, Rust considers
/// this macro unused, even when it actually is being used. To bypass the diagnostics
/// stemming from this, add #[allow(unused_imports)] before the import statement, as shown above.

#[proc_macro]
pub fn error_chain_quick(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    match ecq_main(input.into()) {
        Ok(val) => val,
        Err(e) => e.into_compile_error()
    }.into()
}