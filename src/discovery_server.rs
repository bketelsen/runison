use crate::config::Config;
use crate::synchronizer::Synchronizer;
use crate::{client::Client, synchronizer::Status};
use crate::{client::Event, common::Message, common::Transfer};

use message_io::network::{Endpoint, NetEvent};

use std::io::Read;
use std::{collections::HashMap, fs::File};
use std::{net::SocketAddr, path::PathBuf};
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
        const CHUNK_SIZE: usize = 65536;

        // Update Status
        self.status = Status::Indexing;

        // create index
        self.synchronizer.index();

        self.status = Status::Running;

        // inbound_transfers are files being received from the other network peer
        let mut inbound_transfers: HashMap<Endpoint, Transfer> = HashMap::new();

        //        let mut ob_tx_map: HashMap<String, Transfer> = HashMap::new();
        // outbound_transfers are files being sent to the other network peer
        let mut outbound_transfers: HashMap<Endpoint, Vec<Transfer>> = HashMap::new();

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

                        Message::GetChangeset(remote_tree) => {
                            if self.debug {
                                println!("[GetChangeset: {:?}]", endpoint)
                            };

                            if let Some(changes) = &self.synchronizer.remote_changes(remote_tree) {
                                self.client
                                    .network
                                    .send(endpoint, Message::Changeset(changes.to_vec()));
                            }
                        }
                        Message::SendMe(file) => {
                            let mut pb = PathBuf::from(&self.synchronizer.config.root.path);
                            pb.push(&file);
                            println!("Received SendMe {:?}", &pb.as_path());
                            if let Ok(f) = File::open(&pb) {
                                println!("opened file {:?}", &pb.as_path());
                                if let Ok(m) = f.metadata() {
                                    println!("opened got metadata{:?}", &pb.as_path());
                                    let mut transfer = Transfer {
                                        endpoint: endpoint,
                                        file: f,
                                        name: file.clone(),
                                        current_size: 0,
                                        expected_size: m.len() as usize,
                                    };

                                    println!("created transfer{:?}", &pb.as_path());
                                    // check to see if this endpoint is already active
                                    if let Some(transfers) = outbound_transfers.get_mut(&endpoint) {
                                        // Yes it's active
                                        /*  Some(transfers) => {
                                            // endpoint isn't active. Let's activate it!
                                            transfers.push(&mut transfer);
                                        }
                                        None => {
                                            let mut v = Vec::new();
                                            v.push(&mut transfer);
                                            outbound_transfers.insert(endpoint, v);
                                        }
                                        */
                                        println!("found transfer{:?}", &pb.as_path());
                                        transfers.push(transfer);

                                        self.client
                                            .event_queue
                                            .sender()
                                            .send(Event::SendChunk(endpoint, file.clone()));

                                        println!("sent SendChunk{:?}", &pb.as_path());
                                    } else {
                                        let mut v = Vec::new();
                                        v.push(transfer);
                                        outbound_transfers.insert(endpoint, v);

                                        self.client
                                            .event_queue
                                            .sender()
                                            .send(Event::SendChunk(endpoint, file.clone()));

                                        println!("sent SendChunk{:?}", &pb.as_path());
                                    }
                                }
                            }
                        }

                        Message::ParticipantList(_) => {}
                        Message::ParticipantNotificationAdded(_, _) => {}
                        Message::ParticipantNotificationRemoved(_) => {}
                        Message::ServerStatus(_) => {}
                        Message::NodeList(_) => {}
                        Message::Changeset(_) => {}
                        Message::FileRequest(_, _) => {}
                        Message::Chunk(_) => {}
                        Message::CanReceive(_) => {}
                        Message::Greetings(_, _) => {}
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
                            self.unregister(&name);
                        }
                    }
                    NetEvent::DeserializationError(_) => (),
                },
                Event::SendChunk(endpoint, fname) => {
                    println!("Received request to send file chunk {:?}", fname.clone());
                    // there should already be a transfer for this endpoint:file
                    if let Some(transfers) = outbound_transfers.get_mut(&endpoint) {
                        for v in transfers {
                            if !v.file.metadata().unwrap().is_dir() {
                                if v.endpoint == endpoint {
                                    println!("Matching endpoint");

                                    let mut data = [0; CHUNK_SIZE];
                                    let bytes_read = v.file.read(&mut data).unwrap();
                                    if bytes_read > 0 {
                                        let chunk = Message::Chunk(Vec::from(&data[0..bytes_read]));
                                        self.client.network.send(endpoint, chunk);
                                        self.client
                                            .event_queue
                                            .sender()
                                            .send(Event::SendChunk(endpoint, fname.clone()));
                                    } else {
                                        return println!("\nFile sent!");
                                    }
                                }
                            }
                        }
                    } else {
                        println!("no transfers found for endpoint {:#}", endpoint);
                    }
                    /*
                    let mut data = [0; CHUNK_SIZE];
                    let bytes_read = file.read(&mut data).unwrap();
                    if bytes_read > 0 {
                        let chunk = SenderMsg::Chunk(Vec::from(&data[0..bytes_read]));
                        network.send(server_id, chunk);
                        file_bytes_sent += bytes_read;
                        event_queue.sender().send(Event::SendChunk);

                        let percentage =
                            ((file_bytes_sent as f32 / file_size as f32) * 100.0) as usize;
                        print!("\rSending '{}': {}%", file_name, percentage);
                    } else {
                        return println!("\nFile sent!");
                    }
                    */
                }
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
