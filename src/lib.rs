use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, parse_macro_input};

#[proc_macro_derive(EnumRotate)]
pub fn rotate_enum(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let variants = if let Data::Enum(data) = &input.data {
        data.variants.iter().collect::<Vec<_>>()
    } else {
        panic!("derive(RotateEnum) must be applied to an enum");
    };

    let indices = (0..variants.len()).collect::<Vec<_>>();

    let nexts = variants
        .iter()
        .skip(1)
        .chain(variants.get(0))
        .map(|v| (&v.ident))
        .collect::<Vec<_>>();

    let variants_rev = {
        let mut tmp = variants.clone();
        tmp.reverse();
        tmp
    };

    let tokens = quote! {
        impl ::enum_rotate::EnumRotate for #name {
            fn next(self) -> Self {
                match self {
                    #( Self::#variants => Self::#nexts, )*
                }
            }

            fn prev(self) -> Self {
                match self {
                    #( Self::#nexts => Self::#variants, )*
                }
            }

            fn iter() -> ::enum_rotate::Iter<Self> {
                ::enum_rotate::Iter::new(vec![ #( Self::#variants_rev ),* ])
            }

            fn iter_from(self) -> ::enum_rotate::Iter<Self> {
                let mut tmp = vec![ #( Self::#variants_rev ),* ];
                let index = match self {
                    #( Self::#variants => #indices, )*
                };
                tmp.rotate_right(index);
                ::enum_rotate::Iter::new(tmp)
            }
        }
    };

    tokens.into()
}
