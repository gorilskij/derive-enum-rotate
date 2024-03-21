use proc_macro::TokenStream;
use proc_macro_error::{abort, proc_macro_error};
use quote::{quote, ToTokens};
use syn::spanned::Spanned;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Ident, Token};

struct IterationOrder(Vec<Ident>);
impl syn::parse::Parse for IterationOrder {
    // TODO: prettify errors
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let attr_content;
        input.parse::<Token![#]>()?;
        syn::bracketed!(attr_content in input);
        let ident = attr_content.parse::<Ident>()?;
        assert_eq!(ident, "iteration_order");
        let paren_content;
        syn::parenthesized!(paren_content in attr_content);
        let mut idents = vec![];

        while !paren_content.is_empty() {
            let ident = paren_content.parse::<Ident>()?;
            idents.push(ident);
            if !paren_content.is_empty() {
                paren_content.parse::<Token![,]>()?;
            }
        }
        Ok(Self(idents))
    }
}

#[proc_macro_error]
#[proc_macro_derive(EnumRotate, attributes(iteration_order))]
pub fn derive_enum_rotate(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let input_span = input.span();

    // TODO: crash if multiple attributes are present
    let iteration_order: Option<IterationOrder> = input
        .attrs
        .iter()
        .filter(|a| a.path().segments.len() == 1 && a.path().segments[0].ident == "iteration_order")
        .next()
        .map(|a| syn::parse2(a.to_token_stream()).expect("Failed to parse"));

    let enum_data = match input.data {
        Data::Enum(data) => Ok(data),
        Data::Struct(_) => Err("Struct"),
        Data::Union(_) => Err("Union"),
    };
    let enum_data = match enum_data {
        Ok(data) => data,
        Err(item) => abort!(
            input_span,
            "{item} {} is not an enum, EnumRotate can only be derived for enums",
            input.ident,
        ),
    };

    let variants: Vec<_> = enum_data.variants.iter().collect();

    for variant in &variants {
        if !matches!(variant.fields, Fields::Unit) {
            abort!(
                variant.span(),
                "Variant {} is not a unit variant, all variants must be unit variants to derive EnumRotate",
                variant.ident,
            );
        }
    }

    // TODO: validate custom iteration order

    let name = input.ident;
    let indices = (0..variants.len()).collect::<Vec<_>>();
    let map_from = iteration_order
        .map(|io| io.0)
        .unwrap_or_else(|| variants.iter().map(|var| var.ident.clone()).collect());
    let map_to = if map_from.is_empty() {
        vec![]
    } else {
        let mut vec = map_from.clone();
        vec.rotate_left(1);
        vec
    };

    let tokens = quote! {
        impl ::enum_rotate::EnumRotate for #name {
            fn next(self) -> Self {
                match self {
                    #( Self::#map_from => Self::#map_to, )*
                }
            }

            fn prev(self) -> Self {
                match self {
                    #( Self::#map_to => Self::#map_from, )*
                }
            }

            fn iter() -> impl Iterator<Item=Self> {
                vec![ #( Self::#map_from ),* ].into_iter()
            }

            fn iter_from(self) -> impl Iterator<Item=Self> {
                let mut tmp = vec![ #( Self::#map_from ),* ];
                let index = match self {
                    #( Self::#map_from => #indices, )*
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
