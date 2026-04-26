use proc_macro::TokenStream;

/// Marks a struct as a Kafka consumer group.
#[proc_macro_attribute]
pub fn consumer(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Marks a method as a topic handler.
#[proc_macro_attribute]
pub fn topic(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
