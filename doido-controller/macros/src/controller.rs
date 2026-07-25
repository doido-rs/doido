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

/// A parsed filter attribute: the filter fn plus optional `only`/`except` scopes.
struct FilterAttr {
    filter: proc_macro2::Ident,
    only: Option<Vec<String>>,
    except: Option<Vec<String>>,
}

impl FilterAttr {
    /// Whether this filter should run for the action named `action`.
    fn applies_to(&self, action: &str) -> bool {
        if let Some(list) = &self.only {
            list.iter().any(|a| a == action)
        } else if let Some(list) = &self.except {
            !list.iter().any(|a| a == action)
        } else {
            true
        }
    }
}

/// Extract the `[a, b]` action list following `keyword` in the attribute tokens.
fn parse_action_list(tokens: &str, keyword: &str) -> Option<Vec<String>> {
    let after = &tokens[tokens.find(keyword)? + keyword.len()..];
    let start = after.find('[')? + 1;
    let end = after.find(']')?;
    Some(
        after[start..end]
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
    )
}

/// Parse `#[before_action(fn)]` / `#[after_action(fn)]` with an optional
/// `only = [a, b]` or `except = [a, b]` scope.
fn parse_filter_attr(attr: &syn::Attribute) -> Option<FilterAttr> {
    let path_ident = attr.meta.path().get_ident()?.to_string();
    if path_ident != "before_action" && path_ident != "after_action" {
        return None;
    }
    let Meta::List(list) = &attr.meta else {
        return None;
    };

    let tokens_str = list.tokens.to_string();
    let filter_name = tokens_str.split(',').next()?.trim().to_string();
    let filter: proc_macro2::Ident = syn::parse_str(&filter_name).ok()?;

    let only = if tokens_str.contains("only") {
        parse_action_list(&tokens_str, "only")
    } else {
        None
    };
    let except = if tokens_str.contains("except") {
        parse_action_list(&tokens_str, "except")
    } else {
        None
    };

    Some(FilterAttr {
        filter,
        only,
        except,
    })
}

/// Collect the filter names named by `#[skip_before_action(fn, ...)]` attributes.
fn parse_skip_filters(method: &syn::ImplItemFn) -> Vec<String> {
    method
        .attrs
        .iter()
        .filter_map(|attr| {
            let attr_name = attr.meta.path().get_ident()?.to_string();
            if attr_name != "skip_before_action" {
                return None;
            }
            let Meta::List(list) = &attr.meta else {
                return None;
            };
            let name = list.tokens.to_string();
            Some(name.split(',').next()?.trim().to_string())
        })
        .collect()
}

/// The single `#[around_action(fn)]` filter on a method, if present. The filter
/// receives `(&mut Context, run)` where `run` executes the action and yields its
/// `Response`, so it can bracket the action and own the result.
fn parse_around_filter(method: &syn::ImplItemFn) -> Option<proc_macro2::Ident> {
    method.attrs.iter().find_map(|attr| {
        let attr_name = attr.meta.path().get_ident()?.to_string();
        if attr_name != "around_action" {
            return None;
        }
        let Meta::List(list) = &attr.meta else {
            return None;
        };
        syn::parse_str(list.tokens.to_string().split(',').next()?.trim()).ok()
    })
}

pub fn expand_controller(_attr: TokenStream, item: TokenStream) -> Result<TokenStream> {
    let mut impl_block: ItemImpl = parse2(item)?;
    let self_ty = impl_block.self_ty.clone();

    let mut handler_fns: Vec<TokenStream> = Vec::new();
    let mut action_fns: Vec<TokenStream> = Vec::new();

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
        // The action's declared return type (`Response` or `Result<Response, _>`),
        // used on the extracted action fn so `?` resolves and the error type is pinned.
        let ret_ty = match &method.sig.output {
            syn::ReturnType::Type(_, ty) => quote! { #ty },
            syn::ReturnType::Default => quote! { ::axum::response::Response },
        };
        // The action body is moved into a private `async fn` taking `&mut Context`.
        // Using a real `&mut Context` parameter (rather than an `async {}` block,
        // which would capture `&Context` and be `!Send` because `Context: !Sync`)
        // keeps the handler future `Send` as axum's `Handler` requires.
        let action_fn = quote::format_ident!("__doido_action_{}", fn_name);
        action_fns.push(quote! {
            async fn #action_fn(ctx: &mut ::doido_controller::Context) -> #ret_ty #body
        });

        let mut before_chain: Vec<TokenStream> = Vec::new();
        let mut after_chain: Vec<TokenStream> = Vec::new();
        let skipped = parse_skip_filters(method);

        for attr in &method.attrs {
            let path_name = attr
                .meta
                .path()
                .get_ident()
                .map(|i| i.to_string())
                .unwrap_or_default();

            if path_name == "before_action" {
                if let Some(filter) = parse_filter_attr(attr) {
                    let filter_fn = &filter.filter;
                    let is_skipped = skipped.iter().any(|s| filter_fn == s);
                    if filter.applies_to(&fn_name_str) && !is_skipped {
                        before_chain.push(quote! {
                            if let Err(__early_response) = #filter_fn(&mut ctx).await {
                                return __early_response;
                            }
                        });
                    }
                }
            } else if path_name == "after_action" {
                if let Some(filter) = parse_filter_attr(attr) {
                    let filter_fn = &filter.filter;
                    let is_skipped = skipped.iter().any(|s| filter_fn == s);
                    if filter.applies_to(&fn_name_str) && !is_skipped {
                        after_chain.push(quote! {
                            #filter_fn(&mut ctx).await;
                        });
                    }
                }
            }
        }

        // With an around filter the action is run *through* it (so it can bracket
        // the call and own the response); otherwise the action runs directly.
        // `IntoActionResponse` normalises `Response`/`Result<Response, _>` (mapping
        // `Err` to a 500).
        let invoke = match parse_around_filter(method) {
            Some(around_fn) => quote! {
                let __response = #around_fn(&mut ctx, async |__ctx: &mut ::doido_controller::Context| {
                    let __action_result = Self::#action_fn(__ctx).await;
                    ::doido_controller::IntoActionResponse::into_action_response(__action_result)
                }).await;
            },
            None => quote! {
                let __action_result = Self::#action_fn(&mut ctx).await;
                let __response =
                    ::doido_controller::IntoActionResponse::into_action_response(__action_result);
            },
        };

        handler_fns.push(quote! {
            pub async fn #fn_name(
                req: ::axum::extract::Request,
            ) -> ::axum::response::Response {
                #[allow(unused_mut)]
                let mut ctx = ::doido_controller::Context::build(req).await;
                #(#before_chain)*
                #invoke
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
            #(#action_fns)*
            #(#handler_fns)*
        }
    })
}
