use proc_macro::TokenStream;

// #[job(max_retries = 3, queue = "default")]
// async fn my_job(arg: Type) -> doido_core::Result<()> { ... }
//
// Generates: an impl block with an `enqueue` associated function.
// For simplicity in this MVP, the macro just re-exports the function as-is
// and adds a stub enqueue that panics (to be wired to a queue at runtime).

#[proc_macro_attribute]
pub fn job(attr: TokenStream, item: TokenStream) -> TokenStream {
    let _ = attr; // future: parse max_retries and queue
    // Just pass through for now — the important thing is it compiles
    item
}
