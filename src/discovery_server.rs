use crate::common::Node;

use super::common::{Config, Message, Status};

use message_io::events::EventQueue;
use message_io::network::{Endpoint, NetEvent, Network};

use std::net::SocketAddr;
use std::{collections::HashMap, path::PathBuf};

use walkdir::{DirEntryExt, WalkDir};

enum Event {
    Network(NetEvent<Message>),
}

struct ParticipantInfo {
    addr: SocketAddr,
    endpoint: Endpoint,
}

pub struct DiscoveryServer {
    event_queue: EventQueue<Event>,
    network: Network,
    participants: HashMap<String, ParticipantInfo>,
    status: Status,
    verbosity: u64,
    debug: bool,
    config: Config,
    entries: HashMap<String, Node>,
}

impl DiscoveryServer {
    pub fn new(
        config: Config,
        verbosity: u64,
        debug: bool,
        listen: &str,
        port: &str,
    ) -> Option<DiscoveryServer> {
        let mut event_queue = EventQueue::new();

        let network_sender = event_queue.sender().clone();
        let mut network =
            Network::new(move |net_event| network_sender.send(Event::Network(net_event)));

        let listen_addr = format!("{}:{}", &listen, &port);
        match network.listen_tcp(&listen_addr) {
            Ok(_) => {
                println!("Discovery server running at {}", &listen_addr);
                Some(DiscoveryServer {
                    event_queue,
                    network,
                    participants: HashMap::new(),
                    status: Status::Starting,
                    verbosity: verbosity,
                    debug: debug,
                    config: config,
                    entries: HashMap::new(),
                })
            }
            Err(_) => {
                println!("Can not listen on {}", listen_addr);
                None
            }
        }
    }

    pub fn run(mut self) {
        // Startup Preparation Things
        if self.debug {
            println!("[Preparing server...]");
        };

        if self.debug {
            println!("[Root: {:?}]", self.config.root.path);
        };

        let config = self.config.clone();
        let rp = String::from(config.root.path.clone());
        //  self.entries
        //     .insert(".".to_string(), Node::from_path(&rp, &config).unwrap());
        for entry in WalkDir::new(&rp)
            .into_iter()
            .filter_entry(|e| !crate::common::ignored(e, &config.clone()))
        {
            let config = self.config.clone();
            let e = entry.unwrap().clone();
            //println!("inode: {:?}", e.ino());
            //println!("mtime: {:?}", e.metadata().unwrap().modified().unwrap());
            let entry_path = e.path().clone().to_str().unwrap();
            //println!("path: {:?}", entry_path);
            self.entries.insert(
                e.clone()
                    .path()
                    .strip_prefix(&rp)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string(),
                Node::from_path(entry_path, &config).unwrap(),
            );
        }
        let mut archive = PathBuf::from(&rp);
        archive.push(".runison-before");
        println!("{:?}", archive.display());
        {
            let f = std::fs::File::create(archive).unwrap();
            bincode::serialize_into(f, &self.entries).unwrap();
        }
        println!("{:?}", self.entries.len());
        // debug iterate over everything.
        for (key, _) in &self.entries {
            println!("{}: ", key);
        }
        loop {
            match self.event_queue.receive() {
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
                            self.network
                                .send(endpoint, Message::ServerStatus(self.status));
                        }
                        Message::GetNodes() => {
                            if self.debug {
                                println!("[GetNodes: {:?}]", endpoint)
                            };

                            self.network
                                .send(endpoint, Message::NodeList(self.entries.clone()));
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

            self.network.send(endpoint, Message::ParticipantList(list));

            // Notify other participants about this new participant
            let endpoints = self.participants.values().map(|info| &info.endpoint);
            let message = Message::ParticipantNotificationAdded(name.to_string(), addr);
            self.network.send_all(endpoints, message);

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
            self.network.send_all(endpoints, message);
            println!("Removed participant '{}' with ip {}", name, info.addr);
        } else {
            println!(
                "Can not unregister a non-existent participant with name '{}'",
                name
            );
        }
    }
}
