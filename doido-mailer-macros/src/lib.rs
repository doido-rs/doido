use proc_macro::TokenStream;

/// Marks a struct as a mailer. Currently a pass-through;
/// deliver_now/deliver_later are methods on Mail.
#[proc_macro_attribute]
pub fn mailer(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
