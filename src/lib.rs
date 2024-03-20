use proc_macro::TokenStream;
use proc_macro_error::{abort, proc_macro_error};
use quote::quote;
use syn::spanned::Spanned;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_error]
#[proc_macro_derive(EnumRotate)]
pub fn derive_enum_rotate(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let variants: Vec<_> = if let Data::Enum(data) = &input.data {
        data.variants.iter().collect()
    } else {
        let item = match input.data {
            Data::Struct(_) => "Struct",
            Data::Union(_) => "Union",
            Data::Enum(_) => unreachable!(),
        };
        abort!(
            input.span(),
            "{item} {} is not an enum, EnumRotate can only be derived for enums",
            input.ident,
        );
    };

    for variant in &variants {
        if !matches!(variant.fields, Fields::Unit) {
            abort!(
                variant.span(),
                "Variant {} is not a unit variant, all variants must be unit variants to derive EnumRotate",
                variant.ident,
            );
        }
    }

    let name = input.ident;

    let indices = (0..variants.len()).collect::<Vec<_>>();

    let nexts = variants
        .iter()
        .skip(1)
        .chain(variants.get(0))
        .map(|v| &v.ident)
        .collect::<Vec<_>>();

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

            fn iter() -> impl Iterator<Item=Self> {
                vec![ #( Self::#variants ),* ].into_iter()
            }

            fn iter_from(self) -> impl Iterator<Item=Self> {
                let mut tmp = vec![ #( Self::#variants ),* ];
                let index = match self {
                    #( Self::#variants => #indices, )*
                };
                // If the enum has no variants, the match statement will have no branches
                // and thus all the following code is unreachable, that is ok because if
                // the enum is empty, this method cannot be called in the first place
                #[allow(unreachable_code)]
                {
                    tmp.rotate_left(index);
                    tmp.into_iter()
                }
            }
        }
    };

    tokens.into()
}
