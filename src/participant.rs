use crate::common::{Config, Message, Node};

use message_io::events::EventQueue;
use message_io::network::{Endpoint, NetEvent, Network};

use bincode;
use std::net::SocketAddr;
use std::{collections::HashMap, path::PathBuf};
use walkdir::WalkDir;
enum Event {
    Network(NetEvent<Message>),
}

pub struct Participant {
    event_queue: EventQueue<Event>,
    network: Network,
    name: String,
    debug: bool,
    discovery_endpoint: Endpoint,
    public_addr: SocketAddr,
    known_participants: HashMap<String, Endpoint>, // Used only for free resources later
    entries: HashMap<String, Node>,
    config: Config,
}

impl Participant {
    pub fn new(
        config: Config,
        name: &str,
        target: &str,
        debug: bool,
        verbosity: u64,
    ) -> Option<Participant> {
        let mut event_queue = EventQueue::new();

        let network_sender = event_queue.sender().clone();
        let mut network =
            Network::new(move |net_event| network_sender.send(Event::Network(net_event)));

        // A listener for any other participant that want to establish connection.
        let listen_addr = "127.0.0.1:0";
        if let Ok((_, addr)) = network.listen_udp(listen_addr) {
            // 'addr' contains the port that the OS gives for us when we put a 0.

            // Connection to the discovery server.
            if let Ok(endpoint) = network.connect_tcp(target) {
                Some(Participant {
                    config,
                    event_queue,
                    debug,
                    network,
                    name: name.to_string(),
                    discovery_endpoint: endpoint,
                    public_addr: addr,
                    known_participants: HashMap::new(),
                    entries: HashMap::new(),
                })
            } else {
                println!("Can not connect to the discovery server at {}", target);
                None
            }
        } else {
            println!("Can not listen on {}", listen_addr);
            None
        }
    }

    pub fn run(mut self) {
        // Register this participant into the discovery server
        let message = Message::GetStatus();
        self.network.send(self.discovery_endpoint, message);
        // Startup Preparation Things
        if self.debug {
            println!("[Preparing server...]");
        };

        if self.debug {
            println!("[Root: {:?}]", self.config.root.path);
        };

        let config = self.config.clone();
        let rp = String::from(config.root.path.clone());
        //    self.entries
        //       .insert(".".to_string(), Node::from_path(&rp, &config).unwrap());
        for entry in WalkDir::new(&rp)
            .into_iter()
            .filter_entry(|e| !crate::common::ignored(e, &config.clone()))
        {
            let config = self.config.clone();
            self.entries.insert(
                entry
                    .unwrap()
                    .path()
                    .strip_prefix(&rp)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string(),
                Node::from_path(&config.root.path, &config).unwrap(),
            );
        }
        // todo: put this in the root
        let mut archive = PathBuf::from(&rp);
        archive.push(".runison-before");
        println!("{:?}", archive.display());
        {
            let f = std::fs::File::create(archive).unwrap();
            bincode::serialize_into(f, &self.entries).unwrap();
        }
        let message = Message::RegisterParticipant(self.name.clone(), self.public_addr);
        self.network.send(self.discovery_endpoint, message);
        loop {
            match self.event_queue.receive() {
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
                                self.network
                                    .remove_resource(endpoint.resource_id())
                                    .unwrap();
                            }
                        }
                        Message::Greetings(name, gretings) => {
                            println!("'{}' says: {}", name, gretings);
                        }
                        Message::ServerStatus(status) => {
                            println!("status: {:?}", status);

                            let message = Message::GetNodes();
                            self.network.send(self.discovery_endpoint, message);
                        }

                        Message::NodeList(list) => {
                            self.reconcile(list);
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
            }
        }
    }
    fn reconcile(&mut self, other: HashMap<String, Node>) {
        for (key, _) in other {
            println!("{}: ", key);
            let mine = self.entries.get(&key);
            match mine {
                Some(node) => println!("{:?}", node.name),
                None => println!("Missing Locally: {:?}", &key),
            }
        }
    }
    fn discovered_participant(&mut self, name: &str, addr: SocketAddr, message: &str) {
        if let Ok(endpoint) = self.network.connect_udp(addr) {
            let greetings = format!("Hi '{}', {}", name, message);
            let message = Message::Greetings(self.name.clone(), greetings);
            self.network.send(endpoint, message);
            self.known_participants.insert(name.to_string(), endpoint);
        }
    }
}
