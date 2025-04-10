use nom::{
    Parser,
    bytes::tag,
    number::be_u16,
};
use smol::net::TcpStream;
use tracing::{
    info,
    warn,
};

use crate::{
    packets::{
        self,
        COMPRESSION_MIN_SIZE,
        ClientConnection,
    },
    parser::{
        construct_varint,
        parse_string,
        parse_varint,
    },
    text::TextComponent,
};

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
    {
        use packets::login::{
            LoginPacket,
            client,
        };

        if packets::recv_packet(&mut client, &mut buffer)
            .await
            .is_err()
        {
            return;
        }
        let Ok((_, LoginPacket::Start { name, uuid })) = LoginPacket::parse(&buffer) else {
            return;
        };
        info!("{name} ({uuid}) Attempting connection");

        let packet = client::Compression {
            max_size: COMPRESSION_MIN_SIZE,
        };
        client
            .write_packet(packet)
            .await
            .expect("Failed to send packet");
        client.compression = true;

        let mut reason = TextComponent::new_text("Press ");
        reason.add_child(TextComponent::new_keybind("key.jump").bold().italic());
        reason.add_child(TextComponent::new_text(" to say \""));
        reason.add_child(TextComponent::new_text("Apple").bold());
        reason.add_child(TextComponent::new_text("\""));

        _ = client
            .write_packet(client::Disconnect { reason: &reason })
            .await;
    }
}
