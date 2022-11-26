
use either::Either;
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenTree, Literal};
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

            // innerty contains everything in a <> block
            let mut innerty = Vec::new();
            if input.peek(Token![<]) {
                input.parse::<Token![<]>()?;

                // Read everything until the closing brackets
                let mut depth = 1;

                while depth > 0 {
                    let token = input.parse::<TokenTree>()?;
                    match token {
                        TokenTree::Punct(punct) => {
                            if punct.as_char() == '<' {
                                depth += 1;
                            }
                            if punct.as_char() == '>' {
                                depth -= 1;
                            }
                            innerty.push(TokenTree::Punct(punct));
                        }
                        _ => innerty.push(token),
                    }
                }

                innerty.pop(); // Last token is guaranteed to be a '>'
            }

            if ty_ident == "Vec" {
                // Vec format: Vec<ty> or Vec<ty, length>
                // First, let's split innerty at each comma
                let mut innerty = innerty
                    .into_iter()
                    .fold(vec![Vec::new()], |mut acc, token| {
                        if let TokenTree::Punct(punct) = token {
                            if punct.as_char() == ',' {
                                acc.push(Vec::new());
                                return acc;
                            }

                            acc.last_mut().unwrap().push(TokenTree::Punct(punct));
                        }
                        else {
                            acc.last_mut().unwrap().push(token);
                        }

                        acc
                    })
                    .into_iter()
                    .map(|tokens| tokens.into_iter().collect())
                    .collect::<Vec<proc_macro2::TokenStream>>();
                
                // Now, let's check if we got 1 or 2 TokenStreams
                match innerty.len() {
                    1 => {
                        // We got 1 TokenStream, so we have Vec<ty>
                        let ty = innerty.pop().unwrap();
                        quote! { Vec<#ty> }
                    }
                    2 => {
                        // We got 2 TokenStreams, so we have Vec<ty, length>
                        let length_ts = innerty.pop().unwrap();
                        let ty = innerty.pop().unwrap();

                        // We need to parse the length to see if it's a literal or an ident
                        let length_content = match syn::parse2::<Ident>(length_ts.clone()) {
                            Ok(ident) => Either::Left(ident),
                            Err(_) => Either::Right(syn::parse2::<Literal>(length_ts).unwrap()),
                        };
                        length = Some(length_content);

                        quote! { Vec<#ty> }
                    }
                    _ => panic!("Invalid Vec format"),
                }
            } else {
                if innerty.is_empty() {
                    quote! { #ty_ident }
                } else {
                    quote! { #ty_ident<#(#innerty)*> }
                }
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

struct VecParse {
    ty: Type,
}

impl Parse for VecParse {
    fn parse(input: ParseStream) -> Result<Self> {
        let ident = input.parse::<Ident>()?;
        if ident != "Vec" {
            panic!("Invalid Vec format");
        }

        input.parse::<Token![<]>()?;
        let ty = input.parse::<Type>()?;
        input.parse::<Token![>]>()?;

        Ok(VecParse {
            ty,
        })
    }
}

/// Ident Modifiers
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

pub fn packets(items: TokenStream) -> TokenStream {
    let directions = syn::parse_macro_input!(items as Packets).directions;

    let mut packet_info = Vec::new();
    let mut dirs = Vec::new();

    for direction in &directions {
        let mut states = Vec::new();
        for state in &direction.states {
            let mut packets = Vec::new();
            let mut decode_packets = Vec::new();

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
                    let should_encode = !exclude.iter().any(|e| *e == ident);
                    if should_encode {
                        decode_param.push(quote! {
                            #ident,
                        });
                    }

                    if let Some(length) = &field.length {
                        // if Length exists, then field is a Vec
                        // We need to decode Vec<ty> from ty first
                        let ty: TokenStream = ty.clone().into();
                        let ty = syn::parse_macro_input!(ty as VecParse).ty;

                        match length {
                            either::Left(length_ident) => {
                                if length_ident == "remain" {
                                    decode.push(quote! {
                                        let mut #ident = Vec::new();
                                        while $decoder.remaining() > 0 {
                                            #ident.push(<#ty as serial::Decode>::decode(decoder)?);
                                        }
                                    });
                                }
                                else
                                {
                                    let length = ident_strcat(length_ident, "_usize");
                                    decode.push(quote! {
                                        let #length = u32::from(#length_ident) as usize;
                                        let mut #ident = Vec::with_capacity(#length);
                                        for _ in 0..#length {
                                            #ident.push(<#ty as serial::Decode>::decode(decoder)?);
                                        }
                                    });
                                }
                                
                            }
                            either::Right(literal) => {
                                decode.push(quote! {
                                    let mut #ident = Vec::with_capacity(#literal as usize);
                                    for _ in 0..#literal {
                                        #ident.push(<#ty as serial::Decode>::decode(decoder)?);
                                    }
                                });
                            }
                        }

                        encode.push(quote! {
                            for item in &self.#ident {
                                serial::Encode::encode(item, encoder)?;
                            }
                        });
                    } else {
                        decode.push(quote! {
                            let #ident = <#ty as serial::Decode>::decode(decoder)?;
                        });
                        if should_encode {
                            encode.push(quote! {
                                serial::Encode::encode(&self.#ident, encoder)?;
                            });
                        }
                    }
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

                if packet.traits.contains(&Ident::new("Ignore", Span::call_site())) {
                    let id = packet.id.clone();
                    decode_packets.push(quote! {
                        #id => {
                            error!("Packet with id {} known but undecodable", #id);
                            None
                        }
                    });
                }
                else {
                    let id = packet.id.clone();
                    let full_ident = ident_cat(
                        &ident_camel(&direction.ident),
                        &ident_cat(&ident_camel(&state.ident), &ident_camel(&packet.ident)),
                    );
                    let ident = packet.ident.clone();
                    decode_packets.push(quote! {
                        #id => {
                            let (packet, _) = serial::decode_from_slice::<#ident>(&data).unwrap();
                            Some(Packets::#full_ident(Box::new(packet)))
                        },
                    });
                }
            }

            let state_ident = ident_strcat(&ident_lower(&state.ident), "_packets");
            states.push(quote! {
                pub mod #state_ident {
                    use super::*;
                    use log::error;

                    pub fn decode_packet(id: u8, data: Vec<u8>) -> Option<Packets> {
                        match id {
                            #(#decode_packets)*
                            _ => {
                                error!("Unknown packet id: {}", id);
                                None
                            }
                        }
                    }

                    #(#packets)*
                }
            });
        }
        let dir_ident = ident_lower(&direction.ident);

        let state_idents = direction
            .states
            .iter()
            .map(|s| ident_strcat(&ident_lower(&s.ident), "_packets"))
            .collect::<Vec<_>>();
        let state_decode = direction.states.iter().map(|s| ident_cat(&Ident::new("decode_", Span::call_site()), &ident_lower(&s.ident))).collect::<Vec<_>>();
 
        dirs.push(quote! {
            pub mod #dir_ident {
                use super::*;
                #(pub use #state_idents::decode_packet as #state_decode;)*
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
