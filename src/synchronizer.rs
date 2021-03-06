use std::{
    collections::HashMap,
    fs,
    io::{self, BufReader},
    path::PathBuf,
    time::Instant,
};

use crate::config::{Config, Path};
use glob::Pattern;
use indicatif::{HumanDuration, ProgressBar, ProgressStyle};

use crate::runison::*;
use serde::{Deserialize, Serialize};
use walkdir::{DirEntry, WalkDir};
#[derive(Debug, Serialize, Deserialize, PartialEq, Copy, Clone)]
// Operational status of the process
pub enum Status {
    Starting,
    Indexing,
    Running,
    Stopping,
}

pub struct Synchronizer {
    pub entries: Entries,
    pub config: Config,
    pub first_run: bool,
}
impl Synchronizer {
    pub fn new(config: Config) -> Option<Synchronizer> {
        Some(Synchronizer {
            entries: Entries {
                nodes: HashMap::new(),
            },
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
                        fp = String::from(".");
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
                    self.entries.nodes.insert(
                        fp.clone(),
                        Node::from_path(PathBuf::from(&rp), PathBuf::from(&fp), &config).unwrap(),
                    );
                    pb.tick();
                }
                Err(_) => {}
            }
        }
        pb.finish_and_clear();
        println!("Done indexing in {}", HumanDuration(started.elapsed()));
        /*
        let mut archive = PathBuf::from(&rp);
        archive.push(".runison-current");
        {
            let f = std::fs::File::create(archive).unwrap();
            bincode::serialize_into(f, &self.entries).unwrap();
        }
        */
    }
    pub fn remote_changes(self, remote_tree: Entries) -> Option<Vec<Change>> {
        println!("Detecting file changeset...");

        let started = Instant::now();
        let mut changes = Vec::new();
        for (path, node) in &self.entries.nodes {
            if let Some(remote) = remote_tree.nodes.get(path) {
                // exists in both, check for change
                if !node.dir && node.mod_seconds != remote.mod_seconds {
                    // changed file
                    if node.mod_seconds > remote.mod_seconds {
                        changes.push(Change {
                            change_type: ChangeType::Clientmodify as i32,
                            node: Some(node.clone()),
                        })
                    }
                }
            } else {
                // doesn't exist locally, is new file
                changes.push(Change {
                    change_type: ChangeType::Clientadd as i32,
                    node: Some(node.clone()),
                })
            }
        }
        for (path, remote) in remote_tree.nodes {
            match self.entries.nodes.get(&path) {
                Some(_) => {}
                None => {
                    // remote file doesn't exist locally
                    changes.push(Change {
                        change_type: ChangeType::Serveradd as i32,
                        node: Some(remote.clone()),
                    })
                }
            }
        }

        println!(
            "Done creating changeset in {}",
            HumanDuration(started.elapsed())
        );
        if changes.len() > 0 {
            return Some(changes);
        }

        None
    }
    pub fn local_changes(&mut self) -> Option<Vec<Change>> {
        println!("Detecting changed files...");
        if self.first_run {
            return None;
        }
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

        /*
        let ca: std::result::Result<HashMap<String, Node>, Box<bincode::ErrorKind>> =
            bincode::deserialize_from(reader);
        match ca {
            Ok(current_archive) => {
                let previous = std::fs::File::open(oldarchive).unwrap();
                let preader = BufReader::new(previous);
                let pa: std::result::Result<HashMap<String, Node>, Box<bincode::ErrorKind>> =
                    bincode::deserialize_from(preader);
                match pa {
                    Ok(previous_archive) => {
                        let mut changes = Vec::new();
                        for (path, node) in &current_archive {
                            if let Some(prev) = previous_archive.get(path) {
                                // exists in both, check for change
                                if !node.dir && node.modified != prev.modified {
                                    // changed file
                                    pb.set_message(node.name.clone().to_str().unwrap());
                                    pb.tick();
                                    changes.push(Change {
                                        change_type: ChangeType::Clientmodify as i32,
                                        node: Some(node.clone()),
                                    })
                                }
                            } else {
                                // doesn't exist in previous, is new file
                                pb.set_message(node.name.clone().as_str());
                                pb.tick();
                                changes.push(Change {
                                    change_type: ChangeType::Clientadd as i32,
                                    node: Some(node.clone()),
                                })
                            }
                        }
                        for (path, prev) in previous_archive {
                            match &current_archive.get(&path) {
                                Some(_) => {}
                                None => {
                                    // current file is deleted

                                    pb.set_message(prev.name.clone().as_str());
                                    pb.tick();

                                    changes.push(Change {
                                        change_type: ChangeType::Clientdelete as i32,
                                        node: Some(prev.clone()),
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
         */
        None
    }
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
