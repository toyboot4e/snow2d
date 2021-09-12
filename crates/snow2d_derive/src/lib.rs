/*!
Procedural macros
*/

mod tyobj;
mod via_tyobj;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

/// Implements `TypeObject` trait
#[proc_macro_derive(TypeObject)]
pub fn tyobj(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    TokenStream::from(tyobj::impl_tyobj(ast))
}

/// Implements `SerdeViaTyObj` trait
#[proc_macro_derive(SerdeViaTyObj, attributes(via_tyobj))]
pub fn serde_via_tyobj(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    TokenStream::from(via_tyobj::impl_via_tyobj(ast))
}

// /// Implements `AssetBundle` trait
// #[proc_macro_derive(Assets, asset(via_tyobj))]
// pub fn assets(input: TokenStream) -> TokenStream {
//     todo!()
// }
