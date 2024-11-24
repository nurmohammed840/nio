use std::cell::Cell;

use quote2::{proc_macro2::TokenStream, quote, utils::quote_rep, Quote};
use syn::{
    parse::{Parse, ParseStream, Parser},
    punctuated::Punctuated,
    Attribute, Meta, MetaNameValue, Signature, Token, Visibility,
};

type AttributeArgs = Punctuated<Meta, Token![,]>;

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

pub fn nio_main(is_test: bool, args: TokenStream, item_fn: ItemFn) -> TokenStream {
    let metadata = match AttributeArgs::parse_terminated.parse2(args) {
        Ok(args) => args,
        Err(err) => return err.into_compile_error(),
    };
    let has_worker_threads = Cell::new(false);

    let config = quote_rep(metadata, |t, meta| {
        if let Meta::NameValue(MetaNameValue { path, value, .. }) = &meta {
            if path.is_ident("worker_threads") {
                has_worker_threads.set(true);
            }
            quote!(t, { .#path(#value) });
        }
    });

    let ItemFn {
        attrs,
        vis,
        mut sig,
        body,
    } = item_fn;

    let name = &sig.ident;
    let args = quote_rep(&sig.inputs, |t, arg| match arg {
        syn::FnArg::Receiver(receiver) => {
            quote!(t, { #receiver, });
        }
        syn::FnArg::Typed(pat_type) => {
            let pat = &pat_type.pat;
            quote!(t, { #pat, });
        }
    });

    let async_keyword = sig.asyncness.take();
    let attrs = quote_rep(attrs, |t, attr| {
        quote!(t, { #attr });
    });

    let test_attr = quote(|t| {
        if is_test {
            quote!(t, { #[::core::prelude::v1::test] });
        }
    });

    let test_conf = quote(|t| {
        if is_test && !has_worker_threads.get() {
            quote!(t, { .worker_threads(1) });
        }
    });

    let mut out = TokenStream::new();

    if !sig.inputs.is_empty() {
        quote!(out, {
            #attrs
            #test_attr
            #vis #sig {
                let body = #async_keyword #body;

                nio::runtime::Builder::new_multi_thread()
                    #config
                    #test_conf
                    .enable_all()
                    .build()
                    .expect("Failed building the Runtime")
                    .block_on(body)
            }
        });
    } else {
        quote!(out, {
            #attrs
            #test_attr
            #vis #sig {
                #async_keyword #sig #body

                nio::runtime::Builder::new_multi_thread()
                    #config
                    #test_conf
                    .enable_all()
                    .build()
                    .expect("Failed building the Runtime")
                    .block_on(#name(#args))
            }
        });
    }
    out
}
