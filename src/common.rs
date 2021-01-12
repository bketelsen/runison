use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

use crate::config::Config;
use crate::node::Node;

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

    // From Participant to Participant
    Greetings(String, String), //name and grettings
}
