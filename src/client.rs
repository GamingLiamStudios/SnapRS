use nom::{
    Parser,
    bytes::tag,
    number::be_u16,
};
use smol::net::TcpStream;
use tracing::{
    debug,
    info,
    trace,
    warn,
};
use uuid::Uuid;

use crate::{
    encode::{
        self,
        Generate,
    },
    packets::{
        self,
        COMPRESSION_MIN_SIZE,
        ClientConnection,
        DummyError,
    },
    parser::{
        construct_varint,
        parse_string,
        parse_varint,
    },
    text::TextComponent,
};

fn handle_plugin_message(
    channel: &str,
    data: &[u8],
) -> Vec<u8> {
    let mut buffer = Vec::new();
    match channel {
        "minecraft:brand" => {
            // Parse client data
            let (_, client_brand) =
                parse_string::<32767>(data).expect("Failed to parse brand channel");
            info!("Client has brand {client_brand}");

            _ = encode::bounded_string::<32767, DummyError>("SnapRS")
                .generate_in_place(&mut buffer);
        },
        channel => {
            debug!("Recieved data from unknown channel {channel}");
        },
    }

    buffer
}

async fn handle_login(
    client: &mut ClientConnection,
    buffer: &mut Vec<u8>,
) -> std::io::Result<(String, Uuid)> {
    use packets::login::{
        LoginPacket,
        client,
    };

    packets::recv_packet(client, buffer).await?;
    let Ok((_, LoginPacket::Start { name, uuid })) = LoginPacket::parse(buffer) else {
        return Err(std::io::ErrorKind::InvalidData.into());
    };
    let name = name.to_owned(); // Own it, since buffer isn't constant
    info!("{name} ({uuid}) Attempting connection");

    let packet = client::Compression {
        max_size: COMPRESSION_MIN_SIZE,
    };
    if let Err(error) = client.write_packet(packet).await {
        warn!(?error, "Failed to enable compression for packet");
        return Err(std::io::ErrorKind::InvalidData.into());
    }
    client.compression = true;

    // TODO: Encryption flow

    let packet = client::Success {
        username: &name,
        uuid,
        properties: Vec::new(),
    };
    if let Err(error) = client.write_packet(packet).await {
        warn!(?error, "Failed to complete Login process");
        return Err(std::io::ErrorKind::InvalidData.into());
    }

    packets::recv_packet(client, buffer).await?;
    let Ok((_, LoginPacket::Ack)) = LoginPacket::parse(buffer) else {
        return Err(std::io::ErrorKind::InvalidData.into());
    };
    trace!("Got Login Ack");

    Ok((name, uuid))
}

#[allow(clippy::too_many_lines)]
pub async fn spawn(stream: TcpStream) {
    let mut client = ClientConnection::new(stream);

    // Read handshake packet
    // TODO: Custom logic for Legacy Ping
    let mut buffer = vec![0; 2048];
    if packets::recv_packet(&mut client, &mut buffer)
        .await
        .is_err()
    {
        warn!("Attemped connection from invalid client");
        return;
    }

    let Ok((_, (_, _, _server_address, _server_port, next_state))) = (
        tag(&construct_varint::<0x00>()[..]),
        tag(&construct_varint::<765>()[..]), // 1.20.4
        parse_string::<256>,
        be_u16(),
        parse_varint,
    )
        .parse(&buffer[..])
    else {
        return;
    };

    match next_state {
        1 => {
            // Status
            use packets::status::{
                StatusPacket,
                client,
            };

            loop {
                if packets::recv_packet(&mut client, &mut buffer)
                    .await
                    .is_err()
                {
                    return;
                }

                match StatusPacket::parse(&buffer) {
                    Ok((_, StatusPacket::Request)) => {
                        // TODO: Fetch server info
                        let response = client::StatusResponse {};
                        if client.write_packet(response).await.is_err() {
                            return;
                        }
                    },
                    Ok((_, StatusPacket::Ping(timestamp))) => {
                        let response = client::StatusPong { timestamp };
                        _ = client.write_packet(response).await;
                        return;
                    },
                    Err(_) => return,
                }
            }
        },
        2 => {
            // Login, handled outside of match
        },
        _ => return,
    }

    // Do Login
    let Ok((name, _uuid)) = handle_login(&mut client, &mut buffer).await else {
        return;
    };

    // Do configuration flow
    {
        use packets::configure::{
            ConfigurePacket,
            client,
        };

        loop {
            if packets::recv_packet(&mut client, &mut buffer)
                .await
                .is_err()
            {
                warn!("Client left suddenly");
                return;
            }
            match ConfigurePacket::parse(&buffer) {
                Ok((_, ConfigurePacket::PluginMessage { channel, data })) => {
                    // Respond with channel data
                    let response = handle_plugin_message(channel, data);

                    let packet = client::PluginMessage {
                        channel,
                        data: &response,
                    };
                    _ = client.write_packet(packet).await;
                },
                Ok((
                    _,
                    ConfigurePacket::Information {
                        locale: _,
                        view_distance,
                        chat_mode: _,
                        chat_colors: _,
                        enabled_skin: _,
                        main_hand: _,
                        text_filtering: _,
                        server_listings: _,
                    },
                )) => {
                    info!("Sending disconnect");
                    let reason = TextComponent::new_text(format!("Only {view_distance} chunks?"));
                    let response = client::Disconnect { reason: &reason };
                    _ = client.write_packet(response).await;
                    return;
                },
                Ok((_, packet)) => {
                    info!(?packet, "{name} sent config packet");
                },
                Err(nom::Err::Incomplete(_)) => {
                    warn!("Client sent incomplete packet");
                },
                Err(nom::Err::Error(error) | nom::Err::Failure(error)) => {
                    warn!(?error, "Unknown Packet?");
                },
            }
        }
    }
}
