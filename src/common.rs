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
use std::net::SocketAddr;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

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

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
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
