use quote2::{Quote, proc_macro2::TokenStream, quote, utils::quote_rep};
use syn::{
    Attribute, MacroDelimiter, Meta, MetaNameValue, Signature, Token, Visibility,
    parse::{Parse, ParseStream, Parser},
    punctuated::Punctuated,
};

type AttributeArgs = Punctuated<Meta, Token![,]>;

/// Same as: [`syn::ItemFn`]
///
/// But [`ItemFn::body`] is [`TokenStream`] instead of [`syn::Block`]
pub struct ItemFn {
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub sig: Signature,
    pub body: TokenStream,
}

impl Parse for ItemFn {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        Ok(Self {
            attrs: input.call(Attribute::parse_outer)?,
            vis: input.parse()?,
            sig: input.parse()?,
            body: input.parse()?,
        })
    }
}

pub fn nio_main(
    mut crate_path: TokenStream,
    is_test: bool,
    args: TokenStream,
    item_fn: ItemFn,
) -> TokenStream {
    let _metadata = match AttributeArgs::parse_terminated.parse2(args) {
        Ok(args) => args,
        Err(err) => return err.into_compile_error(),
    };

    let mut metadata = Vec::new();
    for meta in _metadata {
        match meta {
            Meta::Path(path) if path.is_ident("crate") => {
                crate_path = TokenStream::new();
                quote!(crate_path, { crate });
            }
            Meta::NameValue(MetaNameValue { path, value, .. }) if path.is_ident("crate") => {
                crate_path = quote2::ToTokens::to_token_stream(&value);
            }
            _ => metadata.push(meta),
        }
    }

    let config = quote_rep(metadata, |t, meta| match meta {
        Meta::List(list) if matches!(list.delimiter, MacroDelimiter::Paren(_)) => {
            quote!(t, { .#list });
        }
        #[allow(unused)]
        Meta::NameValue(MetaNameValue { path, value, .. }) => {
            quote!(t, { .#path(#value) });
        }
        Meta::Path(path) => {
            quote!(t, { .#path() });
        }
        _ => {}
    });

    let ItemFn {
        attrs,
        vis,
        mut sig,
        body,
    } = item_fn;

    let async_keyword = sig.asyncness.take();
    let attrs = quote_rep(attrs, |t, attr| {
        quote!(t, { #attr });
    });

    let test_attr = quote(|t| {
        if is_test {
            quote!(t, { #[::core::prelude::v1::test] });
        }
    });

    let test_config = quote(|t| {
        if is_test {
            quote!(t, { .worker_threads(1) });
        }
    });

    let mut out = TokenStream::new();

    if sig.inputs.is_empty() {
        quote!(out, {
            #attrs
            #test_attr
            #vis #sig {
                let body = #async_keyword #body;

                #crate_path::RuntimeBuilder::new()
                    #test_config
                    #config
                    .rt()
                    .block_on(body)
            }
        });
    } else {
        let name = &sig.ident;
        let args = quote_rep(&sig.inputs, |t, arg| match arg {
            syn::FnArg::Receiver(_) => {
                panic!("using the `self` keyword is not supported; try `this: Type` instead");
            }
            syn::FnArg::Typed(pat_type) => {
                let arg = &pat_type.pat;
                quote!(t, { #arg, });
            }
        });

        quote!(out, {
            #attrs
            #test_attr
            #vis #sig {
                #async_keyword #sig #body

                #crate_path::RuntimeBuilder::new()
                    #test_config
                    #config
                    .rt()
                    .block_on(#name(#args))
            }
        });
    }
    out
}
