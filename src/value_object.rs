use syn;
use quote::quote;
use syn::DeriveInput;
use darling::FromDeriveInput;
use proc_macro2::TokenStream as TokenStream2;
use darling::Error;
use quote::ToTokens;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(value_object))]
pub struct ValueObjectAttributes {
    #[darling(rename = "error_type")]
    error_type: syn::Path,
    #[darling(rename = "load_fn")]
    load_fn: syn::Path,
    #[darling(rename = "serde_derive", default)]
    serde_derive: Option<bool>,
    #[darling(rename = "serde_crate", default)]
    serde_crate: Option<String>,
    #[darling(rename = "display_derive", default)]
    display_derive: Option<bool>,
    #[darling(rename = "try_from_derive", default)]
    try_from_derive: Option<bool>,
    #[darling(rename = "from_str_derive", default)]
    from_str_derive: Option<bool>,
}

#[derive(Debug)]
pub struct ValueObject {
    ident: syn::Ident,
    attributes: ValueObjectAttributes,
    generics: syn::Generics,
    struct_internals: syn::Data,
}

impl ValueObject {
    pub fn new(attributes: ValueObjectAttributes, input: DeriveInput) -> ValueObject {
        return ValueObject {
            attributes,
            ident: input.ident,
            generics: input.generics,
            struct_internals: input.data,
        };
    }

    pub fn validate(&self) -> Result<(), Error> {
        if !self.generics.params.is_empty() {
            return Err(Error::custom(format!("Generics not allowed in value-object `{}`", self.ident)));
        }
        let _ = self.get_internal_type()?;
        return Ok(());
    }

    fn get_internal_type(&self) -> Result<syn::Type, Error> {
        let struct_internals = match &self.struct_internals {
            &syn::Data::Struct(ref struct_internals) => {
                struct_internals
            },
            &syn::Data::Enum(..) => {
                return Err(Error::custom("Enum struct not supported"));
            },
            &syn::Data::Union(..) => {
                return Err(Error::custom("Union struct not supported"));
            },
        };
        let field = match struct_internals.fields {
            syn::Fields::Named(ref fields) => {
                if fields.named.len() != 1 {
                    return Err(Error::custom("Object value must contain only one field"));    
                }
                fields.named[0].clone()
            },
            syn::Fields::Unnamed(ref fields) => {
                if fields.unnamed.len() != 1 {
                    return Err(Error::custom("Object value must contain only one field"));    
                }
                fields.unnamed[0].clone()
            },
            syn::Fields::Unit => {
                return Err(Error::custom("Empty structs does not supported"));
            },
        };
        return Ok(field.ty);
    }

    fn generate_serde(&self) -> Result<TokenStream2, Error> {
        let serde_enable = self.attributes.serde_derive.unwrap_or(true);
        if !serde_enable {
            return Ok(TokenStream2::new());
        }
        let serde_crate = self
            .attributes
            .serde_crate
            .as_ref()
            .map(String::as_str)
            .unwrap_or("serde");
        let serde_crate = syn::Ident::new(serde_crate, proc_macro2::Span::call_site());
        let ident = &self.ident;
        let load_fn = &self.attributes.load_fn;
        let internal_type = self.get_internal_type()?;
        return Ok(quote! {
            impl <'de>#serde_crate::de::Deserialize<'de> for #ident {
                fn deserialize<D>(deserializer: D) -> Result<#ident, D::Error> where
                    D: #serde_crate::de::Deserializer<'de> {
                    let value = #internal_type::deserialize(deserializer)?;
                    let value = #load_fn(value).map_err(#serde_crate::de::Error::custom)?;
                    return Ok(value);
                }
            }
            
            impl #serde_crate::Serialize for #ident {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where
                    S: #serde_crate::Serializer {
                    self.0.serialize(serializer)
                }
            }            
        });
    }

    fn generate_display(&self) -> Result<TokenStream2, Error> {
        let display_derive_enabled = self
            .attributes
            .display_derive
            .unwrap_or(true);
        if !display_derive_enabled {
            return Ok(TokenStream2::new());
        }
        let ident = &self.ident;
        return Ok(quote! {
            impl std::fmt::Display for #ident {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
                    write!(f, "{}", self.0)
                }
            }
        });
    }

    fn generate_try_from(&self) -> Result<TokenStream2, Error> {
        let try_from_derive_enabled = self
            .attributes
            .try_from_derive
            .unwrap_or(true);
        if !try_from_derive_enabled {
            return Ok(TokenStream2::new());
        }
        let internal_type = self.get_internal_type()?;
        let error_type = &self.attributes.error_type;
        let ident = &self.ident;
        let load_fn = &self.attributes.load_fn;
        return Ok(quote! {
            impl std::convert::TryFrom<#internal_type> for #ident {
                type Error = #error_type;
                fn try_from(value: #internal_type) -> Result<Self, Self::Error> {
                    let value = #load_fn(value)?;
                    return Ok(value);
                }
            }
        });
    }

    fn generate_from_str(&self) -> Result<TokenStream2, Error> {
        const FROM_STR_DEFAULT_TYPES: [&'static str; 17] = [
            "bool", "char", 
            "f32", "f64", 
            "i8", "i16", "i32", "i64", "i128", "isize",
            "u8", "u16", "u32", "u64", "u128", "usize",
            "String",
        ];
        let internal_type = self.get_internal_type()?;
        let internal_type_str = format!("{}", internal_type.to_token_stream());
        let internal_type_str = internal_type_str.as_str();
        let is_default_type = FROM_STR_DEFAULT_TYPES.contains(&internal_type_str);
        let from_str_derive_enable = self
            .attributes
            .from_str_derive
            .unwrap_or(is_default_type);
        if !from_str_derive_enable {
            return Ok(TokenStream2::new());
        }
        let ident = &self.ident;
        let load_fn = &self.attributes.load_fn;
        let error_type = &self.attributes.error_type;
        return Ok(quote! {
            impl std::str::FromStr for #ident {
                type Err = #error_type;
                fn from_str(s: &str) -> Result<Self, Self::Err> {
                    let value = #internal_type::from_str(s)?;
                    let value = #load_fn(value)?;
                    return Ok(value);
                }
            }
        });
    }

    pub fn generate(&self) -> Result<TokenStream2, Error> {
        let serde_token = self.generate_serde()?;
        let display_token = self.generate_display()?;
        let try_from_token = self.generate_try_from()?;
        let from_str_token = self.generate_from_str()?;
        let result = quote! {
            #serde_token
            #display_token
            #try_from_token
            #from_str_token
        };
        return Ok(result);
    }
}