extern crate proc_macro;
use either::Either;
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenTree, Literal};
use quote::quote;
use syn::{
    braced,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Expr, Ident, Result, Token,
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
    traits: Vec<Ident>,
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
            traits: if input.peek(Token![:]) {
                input.parse::<Token![:]>()?;

                // Punctuation<...> doesn't work properly here for some reason
                let mut traits = Vec::new();
                traits.push(input.parse()?);
                while input.peek(Token![+]) {
                    input.parse::<Token![+]>()?;
                    traits.push(input.parse()?);
                }

                traits
            } else {
                Vec::new()
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
    ty: proc_macro2::TokenStream,

    length: Option<Either<Ident, Literal>>,
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
                    if input.peek(Ident) {
                        length = Some(Either::Left(input.parse()?));
                    } else {
                        length = Some(Either::Right(input.parse()?));
                    }
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
            ty: ty.into(),
            length,
        })
    }
}

fn ident_lower(ident: &Ident) -> Ident {
    Ident::new(&ident.to_string().to_lowercase(), ident.span())
}

fn ident_camel(ident: &Ident) -> Ident {
    let mut ident_str = ident.to_string();
    ident_str[0..0].make_ascii_uppercase();
    Ident::new(&ident_str, ident.span())
}

fn ident_strcat(ident: &Ident, suffix: &str) -> Ident {
    Ident::new(&format!("{}{}", ident, suffix), ident.span())
}

fn ident_cat(ident: &Ident, suffix: &Ident) -> Ident {
    Ident::new(&format!("{}{}", ident, suffix), ident.span())
}

struct PacketInfo {
    id: Expr,
    full_ident: Ident,
    path: Vec<Ident>,
    traits: Vec<Ident>,
}

#[proc_macro]
pub fn packets(items: TokenStream) -> TokenStream {
    let directions = syn::parse_macro_input!(items as Packets).directions;

    let mut packet_info = Vec::new();
    let mut dirs = Vec::new();

    for direction in &directions {
        let mut states = Vec::new();
        for state in &direction.states {
            let mut packets = Vec::new();
            for packet in &state.packets {
                let mut fields = Vec::new();

                let mut exclude = Vec::new();
                for field in &packet.fields {
                    if let Some(length) = &field.length {
                        if let either::Left(ident) = &length {
                            // Find field with the same name as the length
                            let length_field = packet
                                .fields
                                .iter()
                                .find(|f| &f.ident == ident)
                                .expect("length field not found");

                            if !length_field.vis {
                                exclude.push(ident);
                            }
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

                let packet_ident = packet.ident.clone();

                packets.push(quote! {
                    pub struct #packet_ident {
                        #(#fields)*
                    }
                });

                packet_info.push(PacketInfo {
                    id: packet.id.clone(),
                    full_ident: ident_cat(
                        &ident_camel(&direction.ident),
                        &ident_cat(&ident_camel(&state.ident), &ident_camel(&packet.ident)),
                    ),
                    path: vec![
                        ident_lower(&direction.ident),
                        ident_strcat(&ident_lower(&state.ident), "_packets"),
                        packet.ident.clone(),
                    ],
                    traits: packet.traits.clone(),
                });

                if packet.traits.contains(&Ident::new("Ignore", Span::call_site())) {
                    continue;
                }

                let mut decode = Vec::new();
                let mut decode_param = Vec::new();
                let mut encode = Vec::new();

                for field in &packet.fields {
                    let ident = &field.ident;
                    let ty = &field.ty;

                    // Exclude length fields from the struct if not public
                    if !exclude.iter().any(|e| *e == ident) {
                        decode_param.push(quote! {
                            #ident,
                        });
                        encode.push(quote! {
                            serial::encode(&self.#ident, encoder)?;
                        });
                    }

                    decode.push(quote! {
                        let #ident = <#ty as serial::Decode>::decode(decoder)?;
                    });
                }

                packets.push(quote! {
                    impl serial::Decode for #packet_ident {
                        fn decode(decoder: &mut serial::Decoder) -> Result<Self, serial::DecodeError> {
                            #(#decode)*
            
                            Ok(Self {
                                #(#decode_param)*
                            })
                        }
                    }
                    impl serial::Encode for #packet_ident {
                        fn encode(&self, encoder: &mut serial::Encoder) -> Result<(), serial::EncodeError> {
                            #(#encode)*
            
                            Ok(())
                        }
                    }
                });
            }

            let state_ident = ident_strcat(&ident_lower(&state.ident), "_packets");
            states.push(quote! {
                pub mod #state_ident {
                    use super::*;
                    #(#packets)*
                }
            });
        }
        let dir_ident = ident_lower(&direction.ident);

        dirs.push(quote! {
            pub mod #dir_ident {
                use super::*;
                #(#states)*
            }
        });
    }

    let mut packets = Vec::new();
    let mut packet_conv = Vec::new();

    let mut packet_impl_id = Vec::new();
    let mut packet_impl_data = Vec::new();

    let mut packet_debug = Vec::new();

    for packet in packet_info {
        let full_ident = &packet.full_ident;
        let path = &packet.path;
        let id = &packet.id;

        packets.push(quote! {
            #full_ident(Box<#(#path)::*>)
        });
        packet_conv.push(quote! {
            impl From<#(#path)::*> for Packets {
                fn from(packet: #(#path)::*) -> Self {
                    Self::#full_ident(Box::new(packet))
                }
            }
        });

        packet_impl_id.push(quote! {
            #full_ident(..) => #id,
        });
        if packet
            .traits
            .contains(&Ident::new("Ignore", Span::call_site()))
        {
            packet_impl_data.push(quote! {
                #full_ident(packet) => Vec::new(),
            });
        } else {
            packet_impl_data.push(quote! {
                #full_ident(packet) => serial::encode_to_vec(packet.as_ref()).unwrap(),
            });
        }

        packet_debug.push(quote! {
            #full_ident(packet) => write!(f, stringify!(#full_ident))?,
        });
    }

    quote! {
        pub enum Packets {
            #(#packets,)*
        }
        impl Packets {
            pub fn get_id(&self) -> u8 {
                match self {
                    #(Self::#packet_impl_id)*
                }
            }
            pub fn get_data(&self) -> Vec<u8> {
                match self {
                    #(Self::#packet_impl_data)*
                }
            }
        }
        impl std::fmt::Debug for Packets {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    #(Self::#packet_debug)*
                }
                Ok(())
            }
        }
        #(#packet_conv)*
        #(#dirs)*
    }
    .into()
}
