use proc_macro::TokenStream;

/// Marks a struct as a WebSocket channel.
#[proc_macro_attribute]
pub fn channel(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
