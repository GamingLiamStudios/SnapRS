use log::error;
use slotmap::{DefaultKey, DenseSlotMap};

use crate::config::CONFIG;

use crate::packets::*;

use super::connection::*;
use std::{
    io::Read,
    net::TcpListener,
    sync::{atomic::AtomicBool, mpsc, Arc, Mutex},
    thread::{self, JoinHandle},
};

pub struct NetworkManager {
    connected: Arc<AtomicBool>,
    listener_thread: Option<JoinHandle<()>>,

    // TODO: Invesigate Lock-free alternatives
    // Locks when a new connection is added/removed
    pub connections: Arc<Mutex<DenseSlotMap<DefaultKey, ServerConnection>>>,
}

impl NetworkManager {
    pub fn new() -> Self {
        Self {
            connected: Arc::new(AtomicBool::new(false)),
            listener_thread: None,

            connections: Arc::new(Mutex::new(DenseSlotMap::with_key())),
        }
    }

    fn destroy(&mut self) {
        self.connected
            .store(false, std::sync::atomic::Ordering::Relaxed);
        if let Some(listener_thread) = self.listener_thread.take() {
            listener_thread.join().unwrap();
        }
    }

    pub fn start(&mut self) {
        self.connected
            .store(true, std::sync::atomic::Ordering::Relaxed);

        let connected = self.connected.clone();
        let sever_connections = self.connections.clone();

        self.listener_thread = Some(thread::spawn(move || {
            let listener = TcpListener::bind(format!("127.0.0.1:{}", CONFIG.network.port)).unwrap();
            listener.set_nonblocking(true).unwrap();

            let mut connections = DenseSlotMap::with_capacity(CONFIG.general.max_players);

            while connected.load(std::sync::atomic::Ordering::Relaxed) {
                // Handle all incoming connections(non-blocking)
                for stream in listener.incoming() {
                    match stream {
                        Ok(stream) => {
                            // Configure TCP Stream
                            stream.set_nonblocking(true).unwrap();
                            stream.set_nodelay(true).unwrap();

                            // Add connection to list
                            let (net_inbound, con_inbound) = mpsc::channel();
                            let (con_outbound, net_outbound) = mpsc::channel();

                            {
                                let mut conn = sever_connections.lock().unwrap();
                                (*conn).insert(ServerConnection::new(con_inbound, con_outbound));
                            }

                            connections.insert(NetworkConnection::new(
                                stream,
                                net_inbound,
                                net_outbound,
                            ));
                        }
                        Err(e) => {
                            if e.kind() != std::io::ErrorKind::WouldBlock {
                                error!("Error accepting connection: {}", e);
                            }
                        }
                    }
                }

                // Handle incoming/outgoing data from all connections
                let mut bytes = Vec::with_capacity(4096);
                for (key, connection) in connections.iter_mut() {
                    let read = connection.socket.read(&mut bytes).unwrap();
                    unsafe {
                        // SAFETY: read() returns the number of bytes read
                        // into bytes, so we can safely assume that those
                        // bytes are valid.
                        bytes.set_len(read);
                    }

                    while !bytes.is_empty() {
                        // TODO: packet reading stuff
                        serverbound::decode_handshaking(0x00, vec![0, 0, 0]);
                    }
                }
            }
        }));
    }
}

impl Drop for NetworkManager {
    fn drop(&mut self) {
        self.destroy();
    }
}
