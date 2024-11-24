mod expend;
use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn main(args: TokenStream, item: TokenStream) -> TokenStream {
    expend::nio_main(false, args.into(), syn::parse_macro_input!(item)).into()
}

#[proc_macro_attribute]
pub fn test(args: TokenStream, item: TokenStream) -> TokenStream {
    expend::nio_main(true, args.into(), syn::parse_macro_input!(item)).into()
}
