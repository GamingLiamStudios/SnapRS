use proc_macro::TokenStream;

extern crate proc_macro;
mod packets;

#[proc_macro]
pub fn packets(input: TokenStream) -> TokenStream {
    packets::packets(input)
}
