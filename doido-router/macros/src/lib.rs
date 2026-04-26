mod codegen;
mod parser;

use proc_macro::TokenStream;
use syn::parse_macro_input;

#[proc_macro]
pub fn routes(input: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(input as parser::RoutesInput);
    codegen::generate(parsed).into()
}
