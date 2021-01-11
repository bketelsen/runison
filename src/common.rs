use figment::{
    providers::{Format, Toml},
    Error, Figment,
};
use glob::Pattern;
use std::collections::{BTreeMap, HashMap};
use walkdir::{DirEntry, WalkDir};

use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::fs::File;
use std::io;
use std::net::SocketAddr;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

use message_io::events::EventQueue;
use message_io::network::{Endpoint, NetEvent, Network};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]

pub struct Node {
    pub is_dir: bool,
    pub is_file: bool,
    pub is_symlink: bool,
    pub name: OsString,
    pub len: u64,
    pub modified: std::time::SystemTime,
    pub inode: u64,
}
impl Node {
    pub fn new(
        is_dir: bool,
        is_file: bool,
        is_symlink: bool,
        name: OsString,
        len: u64,
        modified: std::time::SystemTime,
        inode: u64,
    ) -> Option<Node> {
        Some(Node {
            is_dir,
            is_file,
            is_symlink,
            name,
            len,
            modified,
            inode,
        })
    }
    pub fn from_path(path: &str, config: &Config) -> Option<Node> {
        let root_buf = PathBuf::from(path);

        let metadata = std::fs::metadata(&root_buf).unwrap();
        let inode = metadata.ino();
        let filetype = metadata.file_type();

        let root = Node {
            is_dir: filetype.is_dir(),
            is_file: filetype.is_file(),
            is_symlink: filetype.is_symlink(),
            name: PathBuf::from(path).file_name().unwrap().to_os_string(),
            len: metadata.len(),
            modified: metadata.modified().unwrap(),
            inode,
        };
        Some(root)
    }
}
#[derive(Clone, PartialEq, Deserialize)]
pub struct Config {
    pub root: Root,
    pub path: Path,
    pub ignore: Ignore,
}

#[derive(Clone, PartialEq, Deserialize)]
pub struct Root {
    pub path: String,
}
#[derive(Clone, PartialEq, Deserialize)]

pub struct Path {
    pub directories: Vec<String>,
}

#[derive(Clone, PartialEq, Deserialize)]
pub struct Ignore {
    pub name: Vec<String>,
    pub path: Vec<String>,
}

pub fn get_config(path: &str) -> Result<Config, figment::Error> {
    Figment::new().merge(Toml::file(path)).extract()
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Copy, Clone)]
// Operational status of the process
pub enum Status {
    Starting,
    Indexing,
    Running,
    Stopping,
}

#[derive(Serialize, Deserialize)]
pub enum Message {
    // To DiscoveryServer
    RegisterParticipant(String, SocketAddr),
    UnregisterParticipant(String),
    GetStatus(),
    GetNodes(),

    // From DiscoveryServer
    ParticipantList(Vec<(String, SocketAddr)>),
    ParticipantNotificationAdded(String, SocketAddr),
    ParticipantNotificationRemoved(String),
    ServerStatus(Status),
    NodeList(BTreeMap<String, Node>),

    // From Participant to Participant
    Greetings(String, String), //name and grettings
}

// check ignored files and directories, returning true if
// the current entry should be ignored
pub fn ignored(entry: &DirEntry, config: &Config) -> bool {
    if entry.metadata().unwrap().is_dir() {
        for pat in config.ignore.path.clone() {
            if Pattern::new(&pat)
                .unwrap()
                .matches(entry.path().to_str().unwrap())
            {
                println!(
                    "Matched skip rule {:?} for {:?}",
                    &pat,
                    entry.path().to_str().unwrap()
                );
                return true;
            }
        }
    } else {
        for pat in config.ignore.name.clone() {
            if Pattern::new(&pat)
                .unwrap()
                .matches(entry.path().file_name().unwrap().to_str().unwrap())
            {
                println!(
                    "Matched skip rule {:?} for {:?}",
                    &pat,
                    entry.path().file_name().unwrap().to_str().unwrap()
                );
                return true;
            }
        }
    }
    false
}
/*
pub struct DiscoveryServer {
   X event_queue: EventQueue<Event>,
   X network: Network,
    participants: HashMap<String, ParticipantInfo>,
    status: Status,
    verbosity: u64,
    debug: bool,
  X  config: Config,
  X  entries: BTreeMap<String, Node>,
}
pub struct Participant {
  X  event_queue: EventQueue<Event>,
  X  network: Network,
    name: String,
    debug: bool,
    status: Status,
    discovery_endpoint: Endpoint,
    public_addr: SocketAddr,
    known_participants: HashMap<String, Endpoint>, // Used only for free resources later
  X  entries: BTreeMap<String, Node>,
  X  config: Config,
}
*/

pub struct Synchronizer {
    pub entries: BTreeMap<String, Node>,
    config: Config,
}
impl Synchronizer {
    pub fn new(config: Config) -> Option<Synchronizer> {
        Some(Synchronizer {
            entries: BTreeMap::new(),
            config,
        })
    }
    pub fn index(&mut self) {
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
        let mut archive = PathBuf::from(&rp);
        archive.push(".runison-current");
        println!("{:?}", archive.display());
        {
            let f = std::fs::File::create(archive).unwrap();
            bincode::serialize_into(f, &self.entries).unwrap();
        }
    }
}
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
