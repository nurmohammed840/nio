mod expend;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn main(args: TokenStream, item: TokenStream) -> TokenStream {
    expend::nio_main(
        crate_path(),
        false,
        args.into(),
        syn::parse_macro_input!(item),
    )
    .into()
}

#[proc_macro_attribute]
pub fn test(args: TokenStream, item: TokenStream) -> TokenStream {
    expend::nio_main(
        crate_path(),
        true,
        args.into(),
        syn::parse_macro_input!(item),
    )
    .into()
}

fn crate_path() -> quote2::proc_macro2::TokenStream {
    use quote2::*;
    let mut out = proc_macro2::TokenStream::new();
    quote!(out, { ::nio_rt });
    out
}
