use std::net::TcpStream;
use std::sync::mpsc;

use crate::packets::Packet;

// Just a public interface for the connection
pub struct ServerConnection {
    pub inbound: mpsc::Receiver<Box<dyn Packet>>, // Data from NetworkManager
    pub outbound: mpsc::Sender<Box<dyn Packet>>,  // Data to NetworkManager
}

pub(crate) struct NetworkConnection {
    pub(crate) socket: TcpStream,

    pub(crate) inbound: mpsc::Sender<Box<dyn Packet>>, // Data to Server
    pub(crate) outbound: mpsc::Receiver<Box<dyn Packet>>, // Data from Server
}

impl ServerConnection {
    pub fn new(
        inbound: mpsc::Receiver<Box<dyn Packet>>,
        outbound: mpsc::Sender<Box<dyn Packet>>,
    ) -> Self {
        Self { inbound, outbound }
    }
}

impl NetworkConnection {
    pub(crate) fn new(
        socket: TcpStream,
        inbound: mpsc::Sender<Box<dyn Packet>>,
        outbound: mpsc::Receiver<Box<dyn Packet>>,
    ) -> Self {
        Self {
            socket,
            inbound,
            outbound,
        }
    }
}
