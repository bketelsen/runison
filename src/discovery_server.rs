use crate::common::{Client, Event, Synchronizer};
use crate::common::{Config, Message, Status};

use message_io::network::{Endpoint, NetEvent};

use std::collections::HashMap;
use std::net::SocketAddr;

struct ParticipantInfo {
    addr: SocketAddr,
    endpoint: Endpoint,
}

pub struct DiscoveryServer {
    client: Client,
    synchronizer: Synchronizer,
    participants: HashMap<String, ParticipantInfo>,
    status: Status,
    verbosity: u64,
    debug: bool,
}

impl DiscoveryServer {
    pub fn new(
        config: Config,
        verbosity: u64,
        debug: bool,
        listen: &str,
        port: &str,
    ) -> Option<DiscoveryServer> {
        // create network client
        if let Some(mut client) = Client::new(&listen, &port) {
            match client.start() {
                Ok(_) => println!("Started listener"),
                Err(_) => return None,
            };
            // create synchronization service
            if let Some(synchronizer) = Synchronizer::new(config) {
                Some(DiscoveryServer {
                    client,
                    participants: HashMap::new(),
                    status: Status::Starting,
                    verbosity,
                    debug,
                    synchronizer,
                })
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn run(mut self) {
        // Update Status
        self.status = Status::Indexing;

        // create index
        self.synchronizer.index();

        self.status = Status::Running;
        loop {
            match self.client.event_queue.receive() {
                Event::Network(net_event) => match net_event {
                    NetEvent::Message(endpoint, message) => match message {
                        Message::RegisterParticipant(name, addr) => {
                            if self.debug {
                                println!("[Registering participant {:?}]", endpoint)
                            };
                            self.register(&name, addr, endpoint);
                        }
                        Message::UnregisterParticipant(name) => {
                            if self.debug {
                                println!("[Unegistering participant {:?}]", &name)
                            };
                            self.unregister(&name);
                        }

                        Message::GetStatus() => {
                            if self.debug {
                                println!("[Get Status: {:?}]", endpoint)
                            };
                            self.client
                                .network
                                .send(endpoint, Message::ServerStatus(self.status));
                        }
                        Message::GetNodes() => {
                            if self.debug {
                                println!("[GetNodes: {:?}]", endpoint)
                            };

                            self.client.network.send(
                                endpoint,
                                Message::NodeList(self.synchronizer.entries.clone()),
                            );
                        }
                        _ => unreachable!(),
                    },
                    NetEvent::AddedEndpoint(_) => (),
                    NetEvent::RemovedEndpoint(endpoint) => {
                        // Participant disconection without explict unregistration.
                        // We must remove from the registry too.
                        let participant_name = self.participants.iter().find_map(|(name, info)| {
                            if info.endpoint == endpoint {
                                Some(name.clone())
                            } else {
                                None
                            }
                        });

                        if let Some(name) = participant_name {
                            self.unregister(&name)
                        }
                    }
                    NetEvent::DeserializationError(_) => (),
                },
            }
        }
    }

    fn register(&mut self, name: &str, addr: SocketAddr, endpoint: Endpoint) {
        if !self.participants.contains_key(name) {
            // Update the new participant with the whole participants information
            let list = self
                .participants
                .iter()
                .map(|(name, info)| (name.clone(), info.addr))
                .collect();

            self.client
                .network
                .send(endpoint, Message::ParticipantList(list));

            // Notify other participants about this new participant
            let endpoints = self.participants.values().map(|info| &info.endpoint);
            let message = Message::ParticipantNotificationAdded(name.to_string(), addr);
            self.client.network.send_all(endpoints, message);

            // Register participant
            self.participants
                .insert(name.to_string(), ParticipantInfo { addr, endpoint });
            println!("Added participant '{}' with ip {}", name, addr);
        } else {
            println!(
                "Participant with name '{}' already exists, please register with another name",
                name
            );
        }
    }

    fn unregister(&mut self, name: &str) {
        if let Some(info) = self.participants.remove(name) {
            // Notify other participants about this removed participant
            let endpoints = self.participants.values().map(|info| &info.endpoint);
            let message = Message::ParticipantNotificationRemoved(name.to_string());
            self.client.network.send_all(endpoints, message);
            println!("Removed participant '{}' with ip {}", name, info.addr);
        } else {
            println!(
                "Can not unregister a non-existent participant with name '{}'",
                name
            );
        }
    }
}
