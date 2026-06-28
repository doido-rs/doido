use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    parse::Parser, parse_macro_input, punctuated::Punctuated, ItemFn, Lit, LitInt, LitStr,
    MetaNameValue, Token,
};

#[proc_macro_attribute]
pub fn job(attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = parse_macro_input!(item as ItemFn);
    let fn_name = &func.sig.ident;
    let enqueue_fn_name = syn::Ident::new(&format!("{}_enqueue", fn_name), Span::call_site());

    // Parse key=value attributes
    let mut queue_name = "default".to_string();
    let mut max_retries: u32 = 3;
    let mut priority: i32 = 0;
    let mut backoff = "exponential".to_string();
    let mut backoff_base: u64 = 5;
    let mut timeout: u64 = 30;

    let attr_tokens: proc_macro2::TokenStream = attr.into();
    let parser = Punctuated::<MetaNameValue, Token![,]>::parse_terminated;
    if let Ok(pairs) = parser.parse2(attr_tokens) {
        for pair in pairs {
            let key = pair
                .path
                .get_ident()
                .map(|i| i.to_string())
                .unwrap_or_default();
            match key.as_str() {
                "queue" => {
                    if let syn::Expr::Lit(expr_lit) = &pair.value {
                        if let Lit::Str(s) = &expr_lit.lit {
                            queue_name = s.value();
                        }
                    }
                }
                "max_retries" => {
                    if let syn::Expr::Lit(expr_lit) = &pair.value {
                        if let Lit::Int(n) = &expr_lit.lit {
                            max_retries = n.base10_parse().unwrap_or(3);
                        }
                    }
                }
                "priority" => {
                    if let syn::Expr::Lit(expr_lit) = &pair.value {
                        if let Lit::Int(n) = &expr_lit.lit {
                            priority = n.base10_parse().unwrap_or(0);
                        }
                    }
                }
                "backoff" => {
                    if let syn::Expr::Lit(expr_lit) = &pair.value {
                        if let Lit::Str(s) = &expr_lit.lit {
                            backoff = s.value();
                        }
                    }
                }
                "backoff_base" => {
                    if let syn::Expr::Lit(expr_lit) = &pair.value {
                        if let Lit::Int(n) = &expr_lit.lit {
                            backoff_base = n.base10_parse().unwrap_or(5);
                        }
                    }
                }
                "timeout" => {
                    if let syn::Expr::Lit(expr_lit) = &pair.value {
                        if let Lit::Int(n) = &expr_lit.lit {
                            timeout = n.base10_parse().unwrap_or(30);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    let max_retries_lit = LitInt::new(&max_retries.to_string(), Span::call_site());
    let queue_lit = LitStr::new(&queue_name, Span::call_site());
    let priority_lit = LitInt::new(&priority.to_string(), Span::call_site());
    let backoff_base_lit = LitInt::new(&backoff_base.to_string(), Span::call_site());
    let timeout_lit = LitInt::new(&timeout.to_string(), Span::call_site());

    let backoff_variant = match backoff.as_str() {
        "linear" => quote! { doido_jobs::BackoffStrategy::Linear },
        "none" => quote! { doido_jobs::BackoffStrategy::None },
        _ => quote! { doido_jobs::BackoffStrategy::Exponential },
    };

    let expanded = quote! {
        #func

        pub async fn #enqueue_fn_name(
            queue: &dyn doido_jobs::JobQueue,
            payload: serde_json::Value,
        ) -> doido_core::Result<doido_jobs::JobId> {
            let job = doido_jobs::JobPayload::new(#queue_lit, payload, #max_retries_lit)
                .with_priority(#priority_lit)
                .with_backoff(#backoff_variant, #backoff_base_lit)
                .with_timeout(#timeout_lit);
            queue.enqueue(job).await
        }
    };

    expanded.into()
}
