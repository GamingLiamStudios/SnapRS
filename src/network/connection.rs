use std::net::TcpStream;

use crate::packets::Packets;

use crate::packets::types::ConnectionState;

use crossbeam_channel::{Receiver, Sender};

// Just a public interface for the connection
pub struct ServerConnection {
    pub inbound: Receiver<Packets>, // Data from NetworkManager
    pub outbound: Sender<Packets>,  // Data to NetworkManager

    pub state: ConnectionState,
}

pub(crate) struct NetworkConnection {
    pub(crate) socket: TcpStream,

    pub(crate) inbound: Sender<Packets>,    // Data to Server
    pub(crate) outbound: Receiver<Packets>, // Data from Server

    pub(crate) state: ConnectionState,
}

impl ServerConnection {
    pub fn new(inbound: Receiver<Packets>, outbound: Sender<Packets>) -> Self {
        Self {
            inbound,
            outbound,
            state: ConnectionState::Handshake,
        }
    }
}

impl NetworkConnection {
    pub(crate) fn new(
        socket: TcpStream,
        inbound: Sender<Packets>,
        outbound: Receiver<Packets>,
    ) -> Self {
        Self {
            socket,
            inbound,
            outbound,
            state: ConnectionState::Handshake,
        }
    }
}
