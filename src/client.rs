use crate::config::Config;
use crate::node::Node;
use message_io::events::EventQueue;
use message_io::network::{Endpoint, NetEvent, Network};
use serde::{Deserialize, Serialize};
use std::io;
use std::net::SocketAddr;

use crate::common::Message;

pub enum Event {
    Network(NetEvent<Message>),
}

// Client encapsulates the network activity
pub struct Client {
    pub event_queue: EventQueue<Event>,
    pub network: Network,
    listen: String,
    port: String,
    listen_addr: String,
}
impl Client {
    pub fn new(listen: &str, port: &str) -> Option<Client> {
        let mut event_queue = EventQueue::new();

        let network_sender = event_queue.sender().clone();
        let network = Network::new(move |net_event| network_sender.send(Event::Network(net_event)));

        let listen_addr = format!("{}:{}", &listen, &port);
        Some(Client {
            event_queue,
            network,
            listen: listen.to_string(),
            port: port.to_string(),
            listen_addr,
        })
    }
    // Server
    pub fn start(&mut self) -> io::Result<(usize, SocketAddr)> {
        self.network.listen_tcp(&self.listen_addr)
    }
    // Client
    pub fn connect(&mut self, target: &str) -> io::Result<Endpoint> {
        self.network.connect_tcp(target)
    }
    // Client Gossip
    pub fn gossip(&mut self) -> io::Result<(usize, SocketAddr)> {
        self.network.listen_udp(&self.listen_addr)
    }
}
