use crate::config::{Config, Path};
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
#[derive(Clone, PartialEq, Serialize, Deserialize)]

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
impl Node {
    pub fn new(
        is_dir: bool,
        is_file: bool,
        is_symlink: bool,
        name: OsString,
        path: PathBuf,
        relative_path: PathBuf,
        len: u64,
        modified: std::time::SystemTime,
        inode: u64,
        root_path: PathBuf,
    ) -> Option<Node> {
        Some(Node {
            is_dir,
            is_file,
            is_symlink,
            name,
            path,
            relative_path,
            len,
            modified,
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
            is_dir: filetype.is_dir(),
            is_file: filetype.is_file(),
            is_symlink: filetype.is_symlink(),
            name: path.clone().into_os_string(),
            path: joined,
            relative_path: path,
            len: metadata.len(),
            modified: metadata.modified().unwrap(),
            inode,
            root_path,
        };
        Some(root)
    }
}
