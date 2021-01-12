use crate::config::Config;
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
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
