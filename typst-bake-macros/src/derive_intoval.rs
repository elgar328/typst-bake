// Based on derive_typst_intoval by KillTheMule
// https://github.com/KillTheMule/derive_typst_intoval
// Licensed under Apache-2.0 / MIT

use proc_macro2::TokenStream;
use quote::quote;
use syn::ext::IdentExt;
use syn::{Data, DeriveInput, Fields, Generics, Result, parse_quote};

/// Generate the [`IntoValue`] trait implementation for a struct.
pub fn derive_into_value(item: DeriveInput) -> Result<TokenStream> {
    let Input {
        ty,
        generics,
        dictentries,
    } = gather_input(&item)?;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics ::typst_bake::__internal::typst::foundations::IntoValue
            for #ty #ty_generics #where_clause
        {
            fn into_value(self) -> ::typst_bake::__internal::typst::foundations::Value {
                let d = ::typst_bake::__internal::typst::foundations::dict!(
                    #(#dictentries),*
                );
                ::typst_bake::__internal::typst::foundations::Value::Dict(d)
            }
        }
    })
}

/// Generate the [`IntoDict`] implementation and [`From`] conversion for a struct.
pub fn derive_into_dict(item: DeriveInput) -> Result<TokenStream> {
    let Input {
        ty,
        generics,
        dictentries,
    } = gather_input(&item)?;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics #ty #ty_generics #where_clause {
            #[inline]
            #[must_use]
            pub fn into_dict(self) -> ::typst_bake::__internal::typst::foundations::Dict {
                ::typst_bake::__internal::typst::foundations::dict!(
                    #(#dictentries),*
                )
            }
        }

        impl #impl_generics ::core::convert::From<#ty #ty_generics>
            for ::typst_bake::__internal::typst::foundations::Dict #where_clause
        {
            fn from(value: #ty #ty_generics) -> Self {
                value.into_dict()
            }
        }
    })
}

/// What both derives need from the struct.
struct Input<'a> {
    ty: &'a syn::Ident,
    /// The struct's generics, with an `IntoValue` bound added to every type parameter.
    generics: Generics,
    dictentries: Vec<TokenStream>,
}

fn gather_input(item: &DeriveInput) -> Result<Input<'_>> {
    let Data::Struct(data) = &item.data else {
        return Err(syn::Error::new_spanned(item, "only structs are supported"));
    };

    let Fields::Named(ref fields) = data.fields else {
        return Err(syn::Error::new_spanned(
            &data.fields,
            "only named fields are supported",
        ));
    };

    // Bound each type parameter, like the std derives: unlike bounds on the field types,
    // this also works for recursive structs (`children: Vec<Self>`).
    let mut generics = item.generics.clone();
    for param in generics.type_params_mut() {
        param.bounds.push(parse_quote!(
            ::typst_bake::__internal::typst::foundations::IntoValue
        ));
    }

    let dictentries = fields
        .named
        .iter()
        .map(|field| {
            let ident = field.ident.as_ref().unwrap();
            // `r#type` is the key `type`.
            let key = ident.unraw().to_string();

            quote! {
                #key => ::typst_bake::__internal::typst::foundations::IntoValue::into_value(self.#ident)
            }
        })
        .collect();

    Ok(Input {
        ty: &item.ident,
        generics,
        dictentries,
    })
}
