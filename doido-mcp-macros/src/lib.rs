use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn tool(_attr: TokenStream, item: TokenStream) -> TokenStream { item }

#[proc_macro_attribute]
pub fn resource(_attr: TokenStream, item: TokenStream) -> TokenStream { item }

#[proc_macro_attribute]
pub fn mcp_resource(_attr: TokenStream, item: TokenStream) -> TokenStream { item }

#[proc_macro]
pub fn mcp_server(item: TokenStream) -> TokenStream { item }
