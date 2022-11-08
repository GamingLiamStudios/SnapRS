use std::net::TcpStream;
use std::sync::mpsc;

use crate::packets::Packets;

use crate::packets::types::ConnectionState;

// Just a public interface for the connection
pub struct ServerConnection {
    pub inbound: mpsc::Receiver<Packets>, // Data from NetworkManager
    pub outbound: mpsc::Sender<Packets>,  // Data to NetworkManager

    pub state: ConnectionState,
}

pub(crate) struct NetworkConnection {
    pub(crate) socket: TcpStream,

    pub(crate) inbound: mpsc::Sender<Packets>, // Data to Server
    pub(crate) outbound: mpsc::Receiver<Packets>, // Data from Server

    pub(crate) state: ConnectionState,
}

impl ServerConnection {
    pub fn new(inbound: mpsc::Receiver<Packets>, outbound: mpsc::Sender<Packets>) -> Self {
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
        inbound: mpsc::Sender<Packets>,
        outbound: mpsc::Receiver<Packets>,
    ) -> Self {
        Self {
            socket,
            inbound,
            outbound,
            state: ConnectionState::Handshake,
        }
    }
}
