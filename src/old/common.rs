use std::collections::BTreeMap;

use message_io::network::Endpoint;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

use crate::node::Node;
use std::fs::File;

use crate::synchronizer::{Change, Status};

#[derive(Serialize, Deserialize)]
pub enum Message {
    // To DiscoveryServer
    RegisterParticipant(String, SocketAddr),
    UnregisterParticipant(String),
    GetStatus(),
    GetNodes(),
    GetChangeset(BTreeMap<String, Node>),

    // From DiscoveryServer
    ParticipantList(Vec<(String, SocketAddr)>),
    ParticipantNotificationAdded(String, SocketAddr),
    ParticipantNotificationRemoved(String),
    ServerStatus(Status),
    NodeList(BTreeMap<String, Node>),

    Changeset(Vec<Change>),

    // File Transfer
    FileRequest(String, usize), // name, size

    //From sender to receiver
    Chunk(Vec<u8>), // data

    //From receiver to sender
    CanReceive(bool),
    SendMe(String), // name, size

    // From Participant to Participant
    Greetings(String, String), //name and grettings
}
pub struct Transfer {
    pub endpoint: Endpoint,
    pub file: File,
    pub name: String,
    pub current_size: usize,
    pub expected_size: usize,
}
