use figment::{
    providers::{Format, Toml},
    Error, Figment,
};
use serde::{Deserialize, Serialize};

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
