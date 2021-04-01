#[macro_use]
extern crate syn;
extern crate darling;
use proc_macro::TokenStream;
use syn::DeriveInput;
use darling::FromDeriveInput;

mod value_object;
use value_object::{ValueObjectAttributes, ValueObject};
#[proc_macro_derive(ValueObject, attributes(value_object))]
pub fn derive_value_object(input: TokenStream) -> TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);
    let attributes = match ValueObjectAttributes::from_derive_input(&derive_input) {
        Ok(attributes) => attributes,
        Err(error) => {
            return TokenStream::from(error.write_errors());
        },
    };
    let value_object = ValueObject::new(attributes, derive_input);
    if let Err(error) = value_object.validate() {
        return TokenStream::from(error.write_errors());
    }
    let output = match value_object.generate() {
        Ok(output) => output,
        Err(error) => {
            return TokenStream::from(error.write_errors());
        },
    };
    return output.into();
}