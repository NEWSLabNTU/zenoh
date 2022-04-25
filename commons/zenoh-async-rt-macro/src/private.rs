use proc_macro2::TokenStream;
use quote::quote;
use syn::{Block, Error, ImplItemMethod, Result, ReturnType, Signature};

pub fn zfuture_private(item: ImplItemMethod) -> Result<TokenStream> {
    let ImplItemMethod {
        attrs,
        vis,
        defaultness,
        sig:
            Signature {
                constness,
                asyncness,
                unsafety,
                abi,
                fn_token,
                ident,
                generics,
                paren_token: _,
                inputs,
                variadic,
                output,
            },
        block: Block {
            brace_token: _,
            stmts,
        },
    } = item;

    if asyncness.is_none() {
        Error::new(ident.span(), "method must be async");
    }

    let return_ty = match output {
        ReturnType::Default => quote! { () },
        ReturnType::Type(_arrow, return_ty) => quote! { #return_ty },
    };

    let tokens = quote! {
        #(#attrs)*
        #vis #defaultness

        #constness #unsafety #abi
        #fn_token #ident #generics
        ( #inputs ) #variadic
        -> ::zenoh_async_rt::ZFuture<impl ::std::future::Future<Output = #return_ty>>

        {
            ::zenoh_async_rt::ZFuture::new(
                async move {
                    #(#stmts)*
                }
            )
        }
    };

    Ok(tokens)
}
