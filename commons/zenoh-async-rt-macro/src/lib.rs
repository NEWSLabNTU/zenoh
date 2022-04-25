mod private;

use proc_macro::TokenStream;
use syn::ImplItemMethod;

#[proc_macro_attribute]
pub fn zfuture(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(item as ImplItemMethod);
    let tokens = private::zfuture_private(item).unwrap_or_else(|err| err.to_compile_error());
    tokens.into()
}
