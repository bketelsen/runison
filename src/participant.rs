use crate::synchronizer::Synchronizer;
use crate::{client::Client, synchronizer::Status};
use crate::{client::Event, common::Message, common::Transfer};
use crate::{config::Config, node::Node};

use message_io::network::{Endpoint, NetEvent};

use std::collections::HashMap;
use std::net::SocketAddr;

use console::Term;
use console::{style, Emoji};
use indicatif::{HumanDuration, MultiProgress, ProgressBar, ProgressStyle};

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

static LOOKING_GLASS: Emoji<'_, '_> = Emoji("🔍  ", "");
static TRUCK: Emoji<'_, '_> = Emoji("🚚  ", "");
static CLIP: Emoji<'_, '_> = Emoji("🔗  ", "");
static PAPER: Emoji<'_, '_> = Emoji("📃  ", "");
static SPARKLE: Emoji<'_, '_> = Emoji("✨ ", ":-)");

pub struct Participant {
    client: Client,
    synchronizer: Synchronizer,
    name: String,
    debug: bool,
    status: Status,
    discovery_endpoint: Endpoint,
    public_addr: SocketAddr,
    known_participants: HashMap<String, Endpoint>, // Used only for free resources later
    term: Term,
}

impl Participant {
    pub fn new(
        config: Config,
        name: &str,
        target: &str,
        debug: bool,
        verbosity: u64,
    ) -> Option<Participant> {
        let listen = "0.0.0.0";
        let port = "0";
        // create network client
        if let Some(mut client) = Client::new(&listen, &port) {
            match client.gossip() {
                Ok((_, public_addr)) => {
                    // create synchronization service
                    if let Some(synchronizer) = Synchronizer::new(config) {
                        match client.connect(target) {
                            Ok(endpoint) => Some(Participant {
                                client: client,
                                synchronizer,
                                name: name.to_string(),
                                debug,
                                status: Status::Starting,
                                discovery_endpoint: endpoint,
                                public_addr,
                                known_participants: HashMap::new(),
                                term: Term::stdout(),
                            }),
                            Err(_) => None,
                        }
                    } else {
                        None
                    }
                }
                Err(_) => return None,
            }
        } else {
            None
        }
    }

    pub fn run(mut self) {
        const CHUNK_SIZE: usize = 65536;

        // transfers are files being received from the other network peer
        let mut transfers: HashMap<Endpoint, Transfer> = HashMap::new();
        // Update Status
        self.status = Status::Indexing;

        // create index
        self.synchronizer.index();

        self.status = Status::Running;

        // need logic/state here to determine what to do.

        if let Some(changes) = self.synchronizer.local_changes() {
            println!("Got {:?} local changes", changes.len());
            for change in changes {
                println!("{:#?} {:#?}", change.change_type, change.node.path);
            }
        }

        let message = Message::RegisterParticipant(self.name.clone(), self.public_addr);
        self.client.network.send(self.discovery_endpoint, message);

        if self.synchronizer.first_run {
            let message = Message::GetChangeset(self.synchronizer.entries.clone());
            self.client.network.send(self.discovery_endpoint, message);
        }

        loop {
            match self.client.event_queue.receive() {
                // Waiting events
                Event::Network(net_event) => match net_event {
                    NetEvent::Message(_, message) => match message {
                        Message::ParticipantList(participants) => {
                            println!(
                                "Participant list received ({} participants)",
                                participants.len()
                            );
                            for (name, addr) in participants {
                                self.discovered_participant(
                                    &name,
                                    addr,
                                    "I see you in the participant list",
                                );
                            }
                        }
                        Message::ParticipantNotificationAdded(name, addr) => {
                            println!("New participant '{}' in the network", name);
                            self.discovered_participant(&name, addr, "welcome to the network!");
                        }
                        Message::ParticipantNotificationRemoved(name) => {
                            println!("Removed participant '{}' from the network", name);

                            // Free related network resources to the endpoint.
                            // It is only necessary because the connections among participants
                            // are done by UDP,
                            // UDP is not connection-oriented protocol, and the
                            // AddedEndpoint/RemoveEndpoint events are not generated by UDP.
                            if let Some(endpoint) = self.known_participants.remove(&name) {
                                self.client
                                    .network
                                    .remove_resource(endpoint.resource_id())
                                    .unwrap();
                            }
                        }
                        Message::Greetings(name, gretings) => {
                            println!("'{}' says: {}", name, gretings);
                        }
                        Message::ServerStatus(status) => {
                            println!("status: {:?}", status);
                            if status == Status::Running {
                                let message = Message::GetNodes();
                                self.client.network.send(self.discovery_endpoint, message);
                            } else {
                                println!("Server not ready. Current status: {:?}", status);
                            }
                        }

                        Message::NodeList(list) => {
                            self.reconcile(list);
                        }

                        Message::Changeset(changes) => {
                            for change in changes {
                                println!(
                                    "Requesting {:#?}:{:#?}:{:#?}",
                                    change.change_type,
                                    change.node.is_dir,
                                    change.node.relative_path
                                );
                                let message = Message::SendMe(String::from(
                                    change.node.relative_path.to_str().unwrap(),
                                ));
                                self.client.network.send(self.discovery_endpoint, message);
                            }
                        }
                        _ => unreachable!(),
                    },
                    NetEvent::AddedEndpoint(_) => (),
                    NetEvent::RemovedEndpoint(endpoint) => {
                        if endpoint == self.discovery_endpoint {
                            return println!("Discovery server disconnected, closing");
                        }
                    }
                    NetEvent::DeserializationError(_) => (),
                },
                Event::SendChunk(_, _) => {}
            }
        }
    }
    fn reconcile(&mut self, other: BTreeMap<String, Node>) {
        for (key, _) in other {
            println!("{}: ", key);
            let mine = self.synchronizer.entries.get(&key);
            match mine {
                Some(node) => println!("{:?}", node.name),
                None => println!("Missing Locally: {:?}", &key),
            }
        }
    }

    // this is leftover cruft from the demo app in message-io.
    fn discovered_participant(&mut self, name: &str, addr: SocketAddr, message: &str) {
        if let Ok(endpoint) = self.client.network.connect_udp(addr) {
            let greetings = format!("Hi '{}', {}", name, message);
            let message = Message::Greetings(self.name.clone(), greetings);
            self.client.network.send(endpoint, message);
            self.known_participants.insert(name.to_string(), endpoint);
        }
    }
}
