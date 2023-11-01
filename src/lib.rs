use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, parse_macro_input};

#[proc_macro_derive(RotateEnum)]
pub fn rotate_enum(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let variants = if let Data::Enum(data) = &input.data {
        data.variants.iter().collect::<Vec<_>>()
    } else {
        panic!("derive(RotateEnum) must be applied to an enum");
    };

    let nexts = variants
        .iter()
        .skip(1)
        .chain(variants.get(0))
        .map(|v| (&v.ident))
        .collect::<Vec<_>>();

    let tokens = quote! {
        impl RotateEnum for #name {
            fn next(self) -> Self {
                match self {
                    #(Self::#variants => Self::#nexts, )*
                }
            }

            fn prev(self) -> Self {
                match self {
                    #(Self::#nexts => Self::#variants, )*
                }
            }
        }
    };

    tokens.into()
}
