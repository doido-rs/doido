use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemStruct};

/// Marks a struct as a mailer.
///
/// Generates a [`doido_mailer::Mailer`] implementation that carries the
/// snake_case mailer name (`UserMailer` → `"user_mailer"`) used for template
/// resolution (`mailers/<mailer_name>/<action>`). The struct's own action
/// methods — which build `Mail` values and call `deliver_now`/`deliver_later` —
/// are re-emitted untouched.
#[proc_macro_attribute]
pub fn mailer(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let ident = &input.ident;
    let name = to_snake_case(&ident.to_string());

    quote! {
        #input

        impl doido_mailer::Mailer for #ident {
            fn mailer_name() -> &'static str {
                #name
            }
        }
    }
    .into()
}

/// PascalCase/CamelCase → snake_case (`UserMailer` → `user_mailer`).
fn to_snake_case(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for (i, ch) in s.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if i != 0 {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}
