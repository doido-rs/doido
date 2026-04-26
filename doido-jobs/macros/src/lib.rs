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
                _ => {}
            }
        }
    }

    let max_retries_lit = LitInt::new(&max_retries.to_string(), Span::call_site());
    let queue_lit = LitStr::new(&queue_name, Span::call_site());

    let expanded = quote! {
        #func

        pub async fn #enqueue_fn_name(
            queue: &dyn doido_jobs::JobQueue,
            payload: serde_json::Value,
        ) -> doido_core::Result<()> {
            queue.enqueue(doido_jobs::JobPayload::new(#queue_lit, payload, #max_retries_lit)).await
        }
    };

    expanded.into()
}
