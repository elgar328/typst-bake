// Based on derive_typst_intoval by KillTheMule
// https://github.com/KillTheMule/derive_typst_intoval
// Licensed under Apache-2.0 / MIT

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Result};

/// IntoValue derive macro implementation
pub fn derive_into_value(item: DeriveInput) -> Result<TokenStream> {
    let (ty, dictentries) = gather_input(&item)?;

    Ok(quote! {
        impl ::typst_bake::__internal::typst::foundations::IntoValue for #ty {
            fn into_value(self) -> ::typst_bake::__internal::typst::foundations::Value {
                let d = ::typst_bake::__internal::typst::foundations::dict!(
                    #(#dictentries),*
                );
                ::typst_bake::__internal::typst::foundations::Value::Dict(d)
            }
        }
    })
}

/// IntoDict derive macro implementation
pub fn derive_into_dict(item: DeriveInput) -> Result<TokenStream> {
    let (ty, dictentries) = gather_input(&item)?;

    Ok(quote! {
        impl #ty {
            #[inline]
            #[must_use]
            pub fn into_dict(self) -> ::typst_bake::__internal::typst::foundations::Dict {
                ::typst_bake::__internal::typst::foundations::dict!(
                    #(#dictentries),*
                )
            }
        }

        impl ::core::convert::From<#ty> for ::typst_bake::__internal::typst::foundations::Dict {
            fn from(value: #ty) -> Self {
                value.into_dict()
            }
        }
    })
}

fn gather_input(item: &DeriveInput) -> Result<(&syn::Ident, Vec<TokenStream>)> {
    let ty = &item.ident;

    let Data::Struct(ref data) = &item.data else {
        return Err(syn::Error::new_spanned(item, "only structs are supported"));
    };

    let Fields::Named(ref fields) = data.fields else {
        return Err(syn::Error::new_spanned(
            &data.fields,
            "only named fields are supported",
        ));
    };

    let dictentries: Vec<_> = fields
        .named
        .iter()
        .map(|field| {
            let ident = field.ident.as_ref().unwrap();
            let key = ident.to_string();

            quote! {
                #key => ::typst_bake::__internal::typst::foundations::IntoValue::into_value(self.#ident)
            }
        })
        .collect();

    Ok((ty, dictentries))
}
