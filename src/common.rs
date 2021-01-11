use figment::{
    providers::{Format, Toml},
    Error, Figment,
};
use glob::Pattern;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::{BufRead, BufReader};
use std::time::{Instant, SystemTime};
use walkdir::{DirEntry, WalkDir};

use indicatif::{HumanDuration, ProgressBar, ProgressStyle};
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
    pub path: String,
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
        path: String,
        len: u64,
        modified: std::time::SystemTime,
        inode: u64,
    ) -> Option<Node> {
        Some(Node {
            is_dir,
            is_file,
            is_symlink,
            name,
            path,
            len,
            modified,
            inode,
        })
    }
    pub fn from_path(path: &str, config: &Config) -> Option<Node> {
        // add the root path back to the given path to get the full file path
        let config = config.clone();
        let rp = String::from(config.root.path.clone());
        let mut pathbuf = PathBuf::from(&rp);
        pathbuf.push(path);

        // get the metadata of the file
        // TODO: handle error for bad symlinks
        let metadata = std::fs::metadata(&pathbuf).unwrap();
        let inode = metadata.ino();
        let filetype = metadata.file_type();

        let root = Node {
            is_dir: filetype.is_dir(),
            is_file: filetype.is_file(),
            is_symlink: filetype.is_symlink(),
            name: PathBuf::from(path).file_name().unwrap().to_os_string(),
            path: pathbuf.to_str().unwrap().to_string(),
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
#[derive(Debug, Serialize, Deserialize, PartialEq, Copy, Clone)]

pub enum ChangeType {
    Added,
    Modified,
    Deleted,
}
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]

pub struct Change {
    pub change_type: ChangeType,
    pub node: Node,
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
    for pat in config.ignore.path.clone() {
        if Pattern::new(&pat)
            .unwrap()
            .matches(entry.path().to_str().unwrap())
        {
            return true;
        }
    }
    if Pattern::new(".runison-*")
        .unwrap()
        .matches(entry.path().file_name().unwrap().to_str().unwrap())
    {
        return true;
    }
    for pat in config.ignore.name.clone() {
        if Pattern::new(&pat)
            .unwrap()
            .matches(entry.path().file_name().unwrap().to_str().unwrap())
        {
            return true;
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
    first_run: bool,
}
impl Synchronizer {
    pub fn new(config: Config) -> Option<Synchronizer> {
        Some(Synchronizer {
            entries: BTreeMap::new(),
            config,
            first_run: false,
        })
    }
    fn move_index(&mut self) -> io::Result<()> {
        // skip the move if the file won't be there
        if self.first_run {
            return Ok(());
        }
        let config = self.config.clone();
        let rp = String::from(config.root.path.clone());

        let mut archive = PathBuf::from(&rp);
        archive.push(".runison-current");

        let mut newarchive = PathBuf::from(&rp);
        newarchive.push(".runison-previous");
        println!("Moving index to {:?}", newarchive.display());
        fs::rename(archive, newarchive) // Rename a.txt to b.txt
    }
    pub fn index(&mut self) {
        let started = Instant::now();
        match self.move_index() {
            Ok(_) => {}
            Err(_) => {
                self.first_run = true;
            }
        }
        println!("Indexing files...");
        let pb = ProgressBar::new_spinner();
        pb.enable_steady_tick(200);
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("/|\\- ")
                .template("{spinner:.dim.bold} indexing: {wide_msg}"),
        );
        let config = self.config.clone();
        let rp = String::from(config.root.path.clone());
        //    self.entries
        //       .insert(".".to_string(), Node::from_path(&rp, &config).unwrap());
        for entry in WalkDir::new(&rp)
            .into_iter()
            .filter_entry(|e| !ignored(e, &config.clone()))
        {
            let root_path = PathBuf::from(&rp);
            match entry {
                Ok(ent) => {
                    let config = self.config.clone();
                    let fp: String;

                    // if the path of the entry is the same as
                    // the root path, the entry key will be "" unless
                    // we specify it manually
                    if ent.path().to_str().unwrap().to_string().len()
                        == root_path.to_str().unwrap().to_string().len()
                    {
                        fp = root_path.to_str().unwrap().to_string();
                    } else {
                        fp = ent
                            .path()
                            .strip_prefix(&rp)
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .to_string();
                    }
                    pb.set_message(&fp.clone());
                    self.entries
                        .insert(fp.clone(), Node::from_path(&fp, &config).unwrap());
                    pb.tick();
                }
                Err(_) => {}
            }
        }
        pb.finish_and_clear();
        println!("Done indexing in {}", HumanDuration(started.elapsed()));

        let mut archive = PathBuf::from(&rp);
        archive.push(".runison-current");
        println!("{:?}", archive.display());
        {
            let f = std::fs::File::create(archive).unwrap();
            bincode::serialize_into(f, &self.entries).unwrap();
        }
    }
    pub fn local_changes(&mut self) -> Option<Vec<Change>> {
        println!("Detecting changed files...");

        let started = Instant::now();
        let pb = ProgressBar::new_spinner();
        pb.enable_steady_tick(200);
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("/|\\- ")
                .template("{spinner:.dim.bold} found: {wide_msg}"),
        );
        let config = self.config.clone();
        let rp = String::from(config.root.path.clone());

        let mut archive = PathBuf::from(&rp);
        archive.push(".runison-current");

        let mut oldarchive = PathBuf::from(&rp);
        oldarchive.push(".runison-previous");

        // TODO: If there is no current, then there is no local changeset.
        let current = std::fs::File::open(archive).unwrap();
        let reader = BufReader::new(current);

        let ca: std::result::Result<BTreeMap<String, Node>, Box<bincode::ErrorKind>> =
            bincode::deserialize_from(reader);
        match ca {
            Ok(current_archive) => {
                let previous = std::fs::File::open(oldarchive).unwrap();
                let preader = BufReader::new(previous);
                let pa: std::result::Result<BTreeMap<String, Node>, Box<bincode::ErrorKind>> =
                    bincode::deserialize_from(preader);
                match pa {
                    Ok(previous_archive) => {
                        let mut changes = Vec::new();
                        for (path, node) in &current_archive {
                            if let Some(prev) = previous_archive.get(path) {
                                // exists in both, check for change
                                if !node.is_dir && node.modified != prev.modified {
                                    // changed file
                                    pb.set_message(node.name.clone().to_str().unwrap());
                                    pb.tick();
                                    changes.push(Change {
                                        change_type: ChangeType::Modified,
                                        node: node.clone(),
                                    })
                                }
                            } else {
                                // doesn't exist in previous, is new file
                                pb.set_message(node.name.clone().to_str().unwrap());
                                pb.tick();
                                changes.push(Change {
                                    change_type: ChangeType::Added,
                                    node: node.clone(),
                                })
                            }
                        }
                        for (path, prev) in previous_archive {
                            match &current_archive.get(&path) {
                                Some(_) => {}
                                None => {
                                    // current file is deleted

                                    pb.set_message(prev.name.clone().to_str().unwrap());
                                    pb.tick();
                                    changes.push(Change {
                                        change_type: ChangeType::Deleted,
                                        node: prev.clone(),
                                    })
                                }
                            }
                        }

                        pb.finish_and_clear();
                        println!(
                            "Done scanning local changes in {}",
                            HumanDuration(started.elapsed())
                        );
                        return Some(changes);
                    }
                    Err(e) => {
                        println!("Error: {:?}", e);
                        return None;
                    }
                }
            }
            Err(e) => {
                println!("Error: {:?}", e);
                return None;
            }
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
