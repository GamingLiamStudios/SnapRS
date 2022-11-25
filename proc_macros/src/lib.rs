extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro2::TokenTree;
use quote::quote;
use syn::{
    braced,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Expr, Ident, Result, Token, Type,
};

struct Packets {
    directions: Punctuated<Direction, Token![,]>,
}

impl Parse for Packets {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Packets {
            directions: input.parse_terminated(Direction::parse)?,
        })
    }
}

struct Direction {
    ident: Ident,
    states: Punctuated<State, Token![,]>,
}

impl Parse for Direction {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Direction {
            ident: input.parse()?,
            states: {
                input.parse::<Token![=>]>()?;
                let content;
                braced!(content in input);
                content.parse_terminated(State::parse)?
            },
        })
    }
}

struct State {
    ident: Ident,
    packets: Punctuated<Packet, Token![,]>,
}

impl Parse for State {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(State {
            ident: input.parse()?,
            packets: {
                input.parse::<Token![=>]>()?;
                let content;
                braced!(content in input);
                content.parse_terminated(Packet::parse)?
            },
        })
    }
}

struct Packet {
    id: Expr,
    ident: Ident,
    traits: Option<Vec<Ident>>,
    fields: Punctuated<Field, Token![,]>,
}

impl Parse for Packet {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Packet {
            id: input.parse()?,
            ident: {
                input.parse::<Token![=>]>()?;
                input.parse()?
            },
            traits: {
                if input.peek(Token![:]) {
                    input.parse::<Token![:]>()?;

                    // Punctuation doesn't work properly here for some reason
                    let mut traits = Vec::new();
                    traits.push(input.parse()?);
                    while input.peek(Token![+]) {
                        input.parse::<Token![+]>()?;
                        traits.push(input.parse()?);
                    }

                    Some(traits)
                } else {
                    None
                }
            },
            fields: {
                let content;
                braced!(content in input);
                content.parse_terminated(Field::parse)?
            },
        })
    }
}

// TODO: Fixed-length arrays
struct Field {
    vis: bool,
    ident: Ident,
    ty: Type,
    length: Option<Ident>,
}

impl Parse for Field {
    fn parse(input: ParseStream) -> Result<Self> {
        let vis = if input.peek(Token![pub]) {
            input.parse::<Token![pub]>()?;
            true
        } else {
            false
        };
        let ident = input.parse()?;

        input.parse::<Token![:]>()?;
        let mut length = None;
        let ty = {
            let ty_ident = input.parse::<Ident>()?;
            if input.peek(Token![<]) {
                input.parse::<Token![<]>()?;

                let mut innerty = Vec::new();
                let contains_length = input
                    .step(|cursor| {
                        let mut rest = *cursor;
                        while let Some((tt, next)) = rest.token_tree() {
                            match &tt {
                                TokenTree::Punct(punct)
                                    if (punct.as_char() == ',' || punct.as_char() == '>') =>
                                {
                                    return Ok(((punct.as_char() == ','), next));
                                }
                                _ => {
                                    rest = next;
                                    innerty.push(tt);
                                }
                            }
                        }
                        Err(cursor.error("no `,` or `>` was found after this point"))
                    })
                    .unwrap();

                if contains_length {
                    length = Some(input.parse()?);
                    input.parse::<Token![>]>()?;
                }
                quote! { #ty_ident<#(#innerty)*> }
            } else {
                quote! { #ty_ident }
            }
        };

        Ok(Field {
            vis,
            ident,
            ty: syn::parse2(ty)?,
            length,
        })
    }
}

fn ident_lower(ident: &Ident) -> Ident {
    Ident::new(&ident.to_string().to_lowercase(), ident.span())
}

fn ident_cat(ident: &Ident, suffix: &str) -> Ident {
    Ident::new(&format!("{}{}", ident, suffix), ident.span())
}

#[proc_macro]
pub fn packets(items: TokenStream) -> TokenStream {
    let directions = syn::parse_macro_input!(items as Packets).directions;

    let mut stream = Vec::new();

    for direction in directions {
        let mut dirs = Vec::new();
        for state in direction.states {
            let mut packets = Vec::new();
            for packet in state.packets {
                let mut fields = Vec::new();

                let mut exclude = Vec::new();
                for field in &packet.fields {
                    if let Some(length) = &field.length {
                        // FIXME: what
                        println!("length: {:?} {}", length, field.vis);
                        if !field.vis {
                            exclude.push(length);
                        }
                    }
                }

                for field in &packet.fields {
                    let ident = &field.ident;
                    let ty = &field.ty;

                    // Exclude length fields from the struct if not public
                    if exclude.iter().any(|e| *e == ident) {
                        continue;
                    }

                    fields.push(quote! {
                        pub #ident: #ty,
                    });
                }

                let packet_ident = packet.ident;

                packets.push(quote! {
                    pub struct #packet_ident {
                        #(#fields)*
                    }
                });
            }

            let state_ident = ident_cat(&ident_lower(&state.ident), "_packets");
            dirs.push(quote! {
                pub mod #state_ident {
                    use super::*;
                    #(#packets)*
                }
            });
        }
        let dir_ident = ident_lower(&direction.ident);

        stream.push(quote! {
            pub mod #dir_ident {
                use super::*;
                #(#dirs)*
            }
        });
    }

    quote! {
        #(#stream)*
    }
    .into()
}
