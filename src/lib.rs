use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro_error::{abort, proc_macro_error};
use quote::{quote, ToTokens};
use syn::{parse_macro_input, Data, DeriveInput, Fields, Ident, Token};

struct IterationOrder {
    idents: Vec<Ident>,
    idents_span: Span,
}

impl syn::parse::Parse for IterationOrder {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let attr_content;
        input.parse::<Token![#]>()?;
        syn::bracketed!(attr_content in input);
        assert_eq!(attr_content.parse::<Ident>()?, "iteration_order");
        let paren_content;

        // TODO: output a nice error message if this fails ("#[iteration_order]")
        syn::parenthesized!(paren_content in attr_content);

        let mut idents = vec![];
        while !paren_content.is_empty() {
            let ident = match paren_content.parse::<Ident>() {
                Ok(ident) => ident,
                Err(_) => abort!(paren_content.span(), "Expected identifier",),
            };
            idents.push(ident);
            if !paren_content.is_empty() {
                match paren_content.parse::<Token![,]>() {
                    Ok(_) => {}
                    Err(_) => abort!(paren_content.span(), "Expected comma (,)",),
                };
            }
        }
        Ok(Self {
            idents,
            idents_span: paren_content.span().into(),
        })
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

    // Validate custom iteration order
    if let Some(iteration_order) = &iteration_order {
        let expected_len = variants.len();
        let got_len = iteration_order.idents.len();
        if got_len != expected_len {
            abort!(
                iteration_order.idents_span,
                "Expected {} items in the iteration order but got {}",
                expected_len, got_len;
                note = "Enum `{}` has {} variants", input.ident, expected_len;
                note = "Each variant should appear exactly once in the iteration order";
            );
        }

        if let Some(invalid) = iteration_order
            .idents
            .iter()
            .filter(|ident| !variants.iter().any(|var| var.ident == **ident))
            .next()
        {
            abort!(
                iteration_order.idents_span,
                "Invalid variant for enum `{}`: {}",
                input.ident, invalid;
                note = "The iteration order can only contain variants of `{}`",
                input.ident;
            );
        }

        if let Some(missing) = variants
            .iter()
            .filter(|var| !iteration_order.idents.contains(&var.ident))
            .next()
        {
            abort!(
                iteration_order.idents_span,
                "Variant {} not covered",
                missing.ident;
                note = "Each variant of `{}` should appear exactly once in the iteration order",
                input.ident;
            );
        }
    }

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
    let tokens = if variants.is_empty() {
        // Special case for empty enums
        quote! {
            impl ::enum_rotate::EnumRotate for #name {
                fn next(&self) -> Self {
                    unsafe {
                        ::std::hint::unreachable_unchecked()
                    }
                }

                fn prev(&self) -> Self {
                    unsafe {
                        ::std::hint::unreachable_unchecked()
                    }
                }

                fn iter() -> impl Iterator<Item=Self> {
                    ::std::iter::empty()
                }

                fn iter_from(&self) -> impl Iterator<Item=Self> {
                    unsafe {
                        ::std::hint::unreachable_unchecked();
                    }
                    // This is necessary because "() is not an iterator"
                    #[allow(unreachable_code)]
                    ::std::iter::empty()
                }
            }
        }
    } else {
        // Base case for non-empty enums
        let map_from = iteration_order
            .map(|io| io.idents)
            .unwrap_or_else(|| variants.iter().map(|var| var.ident.clone()).collect());
        let map_to = {
            let mut vec = map_from.clone();
            vec.rotate_left(1);
            vec
        };

        quote! {
            impl ::enum_rotate::EnumRotate for #name {
                fn next(&self) -> Self {
                    match self {
                        #( Self::#map_from => Self::#map_to, )*
                    }
                }

                fn prev(&self) -> Self {
                    match self {
                        #( Self::#map_to => Self::#map_from, )*
                    }
                }

                fn iter() -> impl Iterator<Item=Self> {
                    vec![ #( Self::#map_from ),* ].into_iter()
                }

                fn iter_from(&self) -> impl Iterator<Item=Self> {
                    let mut vars = vec![ #( Self::#map_from ),* ];
                    let index = vars.iter().position(|var| {
                        ::std::mem::discriminant(var) == ::std::mem::discriminant(self)
                    }).unwrap();

                    vars.rotate_left(index);
                    vars.into_iter()
                }
            }
        }
    };

    tokens.into()
}
