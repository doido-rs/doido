use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse2, FnArg, ImplItem, ItemImpl, Meta, Pat, PatIdent, PatType, Result};

fn is_action_method(method: &syn::ImplItemFn) -> bool {
    if method.sig.asyncness.is_none() {
        return false;
    }
    method.sig.inputs.iter().any(|arg| {
        if let FnArg::Typed(PatType { pat, .. }) = arg {
            if let Pat::Ident(PatIdent { ident, .. }) = pat.as_ref() {
                return ident == "ctx";
            }
        }
        false
    })
}

/// Parse `#[before_action(fn_name)]` or `#[before_action(fn_name, only = [a, b])]`
fn parse_filter_attr(attr: &syn::Attribute) -> Option<(proc_macro2::Ident, Option<Vec<String>>)> {
    let path_ident = attr.meta.path().get_ident()?.to_string();
    if path_ident != "before_action" && path_ident != "after_action" {
        return None;
    }
    let Meta::List(list) = &attr.meta else {
        return None;
    };

    let tokens_str = list.tokens.to_string();
    let filter_name = tokens_str.split(',').next()?.trim().to_string();
    let filter_ident: proc_macro2::Ident = syn::parse_str(&filter_name).ok()?;

    let only = if tokens_str.contains("only") {
        let start = tokens_str.find('[')? + 1;
        let end = tokens_str.find(']')?;
        let inner = &tokens_str[start..end];
        let actions: Vec<String> = inner
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        Some(actions)
    } else {
        None
    };

    Some((filter_ident, only))
}

pub fn expand_controller(_attr: TokenStream, item: TokenStream) -> Result<TokenStream> {
    let mut impl_block: ItemImpl = parse2(item)?;
    let self_ty = impl_block.self_ty.clone();

    let mut handler_fns: Vec<TokenStream> = Vec::new();

    for impl_item in &impl_block.items {
        let ImplItem::Fn(method) = impl_item else {
            continue;
        };
        if !is_action_method(method) {
            continue;
        }

        let fn_name = &method.sig.ident;
        let fn_name_str = fn_name.to_string();
        let body = &method.block;

        let mut before_chain: Vec<TokenStream> = Vec::new();
        let mut after_chain: Vec<TokenStream> = Vec::new();

        for attr in &method.attrs {
            let path_name = attr
                .meta
                .path()
                .get_ident()
                .map(|i| i.to_string())
                .unwrap_or_default();

            if path_name == "before_action" {
                if let Some((filter_fn, only)) = parse_filter_attr(attr) {
                    let should_apply = match &only {
                        None => true,
                        Some(list) => list.iter().any(|a| a == &fn_name_str),
                    };
                    if should_apply {
                        before_chain.push(quote! {
                            if let Err(__early_response) = #filter_fn(&mut ctx).await {
                                return __early_response;
                            }
                        });
                    }
                }
            } else if path_name == "after_action" {
                if let Some((filter_fn, _)) = parse_filter_attr(attr) {
                    after_chain.push(quote! {
                        #filter_fn(&mut ctx).await;
                    });
                }
            }
        }

        handler_fns.push(quote! {
            pub async fn #fn_name(
                req: ::axum::extract::Request,
            ) -> ::axum::response::Response {
                let (parts, body) = req.into_parts();
                #[allow(unused_mut)]
                let mut ctx = ::doido_controller::Context::from_request(parts, body);
                #(#before_chain)*
                let __response = { #body };
                #(#after_chain)*
                __response
            }
        });
    }

    impl_block.items.retain(|item| {
        if let ImplItem::Fn(method) = item {
            return !is_action_method(method);
        }
        true
    });

    Ok(quote! {
        #impl_block
        impl #self_ty {
            #(#handler_fns)*
        }
    })
}
