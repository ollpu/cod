
use quote::{quote};

#[proc_macro_derive(Node)]
pub fn node(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    match &input.data {
        syn::Data::Struct(data) => derive_struct(&input, data).into(),
        _=> unimplemented!(),
    }
}

fn derive_struct(derive_input: &syn::DeriveInput, _data: &syn::DataStruct) -> proc_macro2::TokenStream {
    let name = &derive_input.ident;

    quote! {
        impl cod::Node for #name {
            
            fn header(&self) -> &cod::Header {
                &self.header
            }

            fn header_mut(&mut self) -> &mut cod::Header {
                &mut self.header
            }
        }
    }
}