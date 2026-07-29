use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    parse::Parser, parse_macro_input, punctuated::Punctuated, ItemFn, Lit, LitInt, LitStr,
    MetaNameValue, Token,
};

/// Convert a snake_case identifier to PascalCase (`send_email` → `SendEmail`).
fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

#[proc_macro_attribute]
pub fn job(attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = parse_macro_input!(item as ItemFn);
    let fn_name = &func.sig.ident;
    let enqueue_fn_name = syn::Ident::new(&format!("{}_enqueue", fn_name), Span::call_site());
    let builder_name = syn::Ident::new(
        &format!("{}Job", to_pascal_case(&fn_name.to_string())),
        Span::call_site(),
    );

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

    // The job's payload type — the last typed parameter (the application context,
    // when present, comes first). The enqueue helper takes this type and
    // serializes it into the `serde_json::Value` every backend persists, so the
    // payload must be `Serialize`. Defaults to `serde_json::Value` for untyped,
    // context-free jobs (back-compat).
    let typed_inputs: Vec<syn::Type> = func
        .sig
        .inputs
        .iter()
        .filter_map(|arg| match arg {
            syn::FnArg::Typed(pat_type) => Some((*pat_type.ty).clone()),
            syn::FnArg::Receiver(_) => None,
        })
        .collect();
    let payload_ty: syn::Type = typed_inputs
        .last()
        .cloned()
        .unwrap_or_else(|| syn::parse_quote!(serde_json::Value));

    // The job's registered name (its function name) — stamped onto every enqueued
    // payload so the worker's `JobRegistry` can route it back to this function.
    let job_name_lit = LitStr::new(&fn_name.to_string(), Span::call_site());

    // Generated handler that deserializes a reserved payload and invokes the job.
    // The engine hands it the shared `Arc<JobContext>`; context-carrying jobs take
    // `&JobContext` as their first parameter (see the `job` generator template),
    // context-free jobs take just the payload. Registered at link time via
    // `inventory`, so defining a `#[job]` is all that's needed to make it runnable.
    let handler_ident = syn::Ident::new(
        &format!("__doido_job_handler_{}", fn_name),
        Span::call_site(),
    );
    let call = match typed_inputs.len() {
        0 => quote! { #fn_name().await },
        1 => quote! { #fn_name(__payload).await },
        _ => quote! { #fn_name(&*_ctx, __payload).await },
    };
    let deserialize = if typed_inputs.is_empty() {
        quote! {}
    } else {
        quote! {
            let __payload: #payload_ty = serde_json::from_value(_job.payload)
                .map_err(|e| doido_core::anyhow::anyhow!(
                    "failed to deserialize job `{}` payload: {e}", #job_name_lit))?;
        }
    };

    let expanded = quote! {
        #func

        #[doc(hidden)]
        fn #handler_ident(
            _job: doido_jobs::JobPayload,
            _ctx: ::std::sync::Arc<doido_jobs::JobContext>,
        ) -> doido_jobs::HandlerFuture {
            ::std::boxed::Box::pin(async move {
                #deserialize
                #call
            })
        }

        doido_jobs::inventory::submit! {
            doido_jobs::JobRegistration {
                name: #job_name_lit,
                handler: #handler_ident,
            }
        }

        pub async fn #enqueue_fn_name(
            queue: &dyn doido_jobs::JobQueue,
            payload: #payload_ty,
        ) -> doido_core::Result<doido_jobs::JobId> {
            let payload = serde_json::to_value(payload)
                .map_err(|e| doido_core::anyhow::anyhow!("failed to serialize job payload: {e}"))?;
            let job = doido_jobs::JobPayload::new(#queue_lit, payload, #max_retries_lit)
                .with_name(#job_name_lit)
                .with_priority(#priority_lit)
                .with_backoff(#backoff_variant, #backoff_base_lit)
                .with_timeout(#timeout_lit);
            queue.enqueue(job).await
        }

        /// Fluent per-instance enqueue builder for the `#fn_name` job (Rails
        /// `Job.set(...).perform_later`). Carries the job's compile-time policy
        /// (queue/retries/backoff/timeout), overridable via the setters.
        pub struct #builder_name {
            payload: #payload_ty,
            queue: String,
            run_at: Option<doido_jobs::chrono::DateTime<doido_jobs::chrono::Utc>>,
        }

        impl #builder_name {
            /// Start a builder with the job's default queue.
            pub fn new(payload: #payload_ty) -> Self {
                Self { payload, queue: #queue_lit.to_string(), run_at: None }
            }

            /// Route to a different queue (Rails `set(queue:)`).
            pub fn on_queue(mut self, queue: impl Into<String>) -> Self {
                self.queue = queue.into();
                self
            }

            /// Delay eligibility by `secs` from now (Rails `set(wait: n.seconds)`).
            pub fn wait(mut self, secs: i64) -> Self {
                self.run_at = Some(doido_jobs::chrono::Utc::now() + doido_jobs::chrono::Duration::seconds(secs));
                self
            }

            /// Make it eligible at an absolute time (Rails `set(wait_until:)`).
            pub fn at(mut self, at: doido_jobs::chrono::DateTime<doido_jobs::chrono::Utc>) -> Self {
                self.run_at = Some(at);
                self
            }

            fn into_payload(self) -> doido_core::Result<doido_jobs::JobPayload> {
                let payload = serde_json::to_value(self.payload)
                    .map_err(|e| doido_core::anyhow::anyhow!("failed to serialize job payload: {e}"))?;
                let mut job = doido_jobs::JobPayload::new(self.queue, payload, #max_retries_lit)
                    .with_name(#job_name_lit)
                    .with_priority(#priority_lit)
                    .with_backoff(#backoff_variant, #backoff_base_lit)
                    .with_timeout(#timeout_lit);
                if let Some(at) = self.run_at {
                    job = job.with_run_at(at);
                }
                Ok(job)
            }

            /// Enqueue for immediate eligibility (unless `wait`/`at` was set).
            pub async fn enqueue(self, queue: &dyn doido_jobs::JobQueue) -> doido_core::Result<doido_jobs::JobId> {
                let job = self.into_payload()?;
                queue.enqueue(job).await
            }

            /// Enqueue, eligible no earlier than `at`.
            pub async fn enqueue_at(
                mut self,
                at: doido_jobs::chrono::DateTime<doido_jobs::chrono::Utc>,
                queue: &dyn doido_jobs::JobQueue,
            ) -> doido_core::Result<doido_jobs::JobId> {
                self.run_at = Some(at);
                self.enqueue(queue).await
            }
        }
    };

    expanded.into()
}
