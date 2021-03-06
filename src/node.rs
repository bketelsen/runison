use crate::config::{Config, Path};
use crate::runison::*;

use serde::{Deserialize, Serialize};
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::{ffi::OsString, time::SystemTime};
/*
pub struct Node {
    pub is_dir: bool,
    pub is_file: bool,
    pub is_symlink: bool,
    pub name: OsString,
    pub path: PathBuf,
    pub relative_path: PathBuf,
    pub len: u64,
    pub modified: std::time::SystemTime,
    pub inode: u64,
    pub root_path: PathBuf,
}
*/
impl Node {
    pub fn new(
        dir: bool,
        file: bool,
        symlink: bool,
        name: String,
        path: String,
        relative_path: String,
        len: u64,
        mod_seconds: u64,
        mod_nano: u32,
        inode: u64,
        root_path: String,
    ) -> Option<Node> {
        Some(Node {
            dir,
            file,
            symlink,
            name,
            path,
            relative_path,
            len,
            mod_seconds,
            mod_nano,
            inode,
            root_path,
        })
    }
    pub fn from_path(root_path: PathBuf, path: PathBuf, config: &Config) -> Option<Node> {
        // add the root path back to the given path to get the full file path
        let config = config.clone();
        // get the metadata of the file
        // TODO: handle error for bad symlinks
        let mut joined = PathBuf::new();
        for p in root_path.iter() {
            joined.push(p);
        }

        if path.to_str() != Some(".") {
            for p in path.iter() {
                joined.push(p);
            }
        }
        let metadata = std::fs::metadata(&joined).unwrap();
        let inode = metadata.ino();
        let filetype = metadata.file_type();

        let root = Node {
            dir: filetype.is_dir(),
            file: filetype.is_file(),
            symlink: filetype.is_symlink(),
            name: String::from(path.clone().into_os_string().to_str().unwrap()),
            path: String::from(joined.into_os_string().to_str().unwrap()),
            relative_path: String::from(path.clone().into_os_string().to_str().unwrap()),
            len: metadata.len(),
            mod_seconds: match metadata
                .modified()
                .unwrap()
                .duration_since(SystemTime::UNIX_EPOCH)
            {
                Ok(n) => n.as_secs(),
                Err(_) => 0,
            },
            mod_nano: match metadata
                .modified()
                .unwrap()
                .duration_since(SystemTime::UNIX_EPOCH)
            {
                Ok(n) => n.subsec_nanos(),
                Err(_) => 0,
            },
            inode,
            root_path: String::from(root_path.into_os_string().to_str().unwrap()),
        };
        Some(root)
    }
}
