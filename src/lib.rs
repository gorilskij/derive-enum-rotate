use proc_macro::TokenStream;
use proc_macro_error::{abort, proc_macro_error};
use quote::{quote, ToTokens};
use syn::{parse_macro_input, Data, DeriveInput, Fields, Ident, Token};

struct IterationOrder(Vec<Ident>);
impl syn::parse::Parse for IterationOrder {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let attr_content;
        input.parse::<Token![#]>()?;
        syn::bracketed!(attr_content in input);
        assert_eq!(attr_content.parse::<Ident>()?, "iteration_order");
        let paren_content;
        syn::parenthesized!(paren_content in attr_content);

        let mut idents = vec![];
        while !paren_content.is_empty() {
            let ident = match paren_content.parse::<Ident>() {
                Ok(ident) => ident,
                Err(_) => abort!(
                    paren_content.span(),
                    "Expected identifier",
                )
            };
            idents.push(ident);
            if !paren_content.is_empty() {
                match paren_content.parse::<Token![,]>() {
                    Ok(_) => {}
                    Err(_) => abort!(
                        paren_content.span(),
                        "Expected comma (,)",
                    ),
                };
            }
        }
        Ok(Self(idents))
    }
}

#[proc_macro_error]
#[proc_macro_derive(EnumRotate, attributes(iteration_order))]
pub fn derive_enum_rotate(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let mut attr_iter = input.attrs.iter().filter(|a| {
        a.path().segments.len() == 1 && a.path().segments[0].ident == "iteration_order"
    });
    let iteration_order: Option<IterationOrder> = attr_iter
        .next()
        .map(|a| syn::parse2(a.to_token_stream()).expect("Failed to parse"));
    // Crash if multiple #[iteration_order(...)] attributes are present
    if let Some(repeated_attr) = attr_iter.next() {
        abort!(
            repeated_attr,
            "Duplicate \"iteration_order\" attribute, please specify at most one iteration order",
        );
    }

    let enum_data = match &input.data {
        Data::Enum(data) => Ok(data),
        Data::Struct(_) => Err("Struct"),
        Data::Union(_) => Err("Union"),
    };
    let enum_data = match enum_data {
        Ok(data) => data,
        Err(item) => abort!(
            input,
            "{item} {} is not an enum, EnumRotate can only be derived for enums",
            input.ident,
        ),
    };

    let variants: Vec<_> = enum_data.variants.iter().collect();

    // TODO: support empty variants: A(), A {}
    for variant in &variants {
        if !matches!(variant.fields, Fields::Unit) {
            abort!(
                variant,
                "Variant {} is not a unit variant, all variants must be unit variants to derive EnumRotate",
                variant.ident,
            );
        }
    }

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
