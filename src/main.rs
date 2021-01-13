mod client;
mod common;
mod config;
mod discovery_server;
mod node;
mod participant;
mod synchronizer;

extern crate clap;
use clap::{App, Arg, SubCommand};

fn main() {
    let matches = App::new("runison")
        .version("0.1.0")
        .author("Brian Ketelsen <mail@bjk.fyi>")
        .about("Synchronize local and remote filesystems")
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .default_value("runison.toml")
                .help("Sets a custom config file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("debug")
                .short("d")
                .help("debug level logging"),
        )
        .arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity"),
        )
        .subcommand(
            SubCommand::with_name("server")
                .about("start synchronization server")
                .arg(
                    Arg::with_name("listen")
                        .short("l")
                        .value_name("LISTEN")
                        .default_value("127.0.0.1")
                        .help("listener interface"),
                )
                .arg(
                    Arg::with_name("port")
                        .short("p")
                        .value_name("PORT")
                        .default_value("5150")
                        .help("listener port"),
                ),
        )
        .subcommand(
            SubCommand::with_name("client")
                .about("start synchronization client")
                .arg(
                    Arg::with_name("name")
                        .required(true)
                        .short("n")
                        .value_name("HOSTNAME")
                        .help("client name"),
                )
                .arg(
                    Arg::with_name("target")
                        .short("t")
                        .value_name("SERVER:PORT")
                        .default_value("127.0.0.1:5150")
                        .help("target server host:port"),
                ),
        )
        .get_matches();

    let config = matches.value_of("config").unwrap_or("default.conf");
    println!("Using config: {}", config);
    let debug = matches.is_present("debug");

    let verbosity = matches.occurrences_of("v");

    // You can check if a subcommand was used like normal
    if let Some(ref matches) = matches.subcommand_matches("server") {
        let listen = matches.value_of("listen").unwrap();
        let port = matches.value_of("port").unwrap();
        let result = config::get_config(config);
        match result {
            Ok(config) => {
                match discovery_server::DiscoveryServer::new(config, verbosity, debug, listen, port)
                {
                    Some(discovery_server) => discovery_server.run(),
                    None => println!("Can not run the discovery server"),
                }
            }
            Err(error) => println!("Error: {:?}", error),
        };
    } else {
        if let Some(ref matches) = matches.subcommand_matches("client") {
            let target = matches.value_of("target").unwrap();
            let name = matches.value_of("name").unwrap();

            let result = config::get_config(config);
            match result {
                Ok(config) => {
                    match participant::Participant::new(config, name, target, debug, verbosity) {
                        Some(participant) => participant.run(),
                        None => println!("Can not run the client"),
                    }
                }
                Err(error) => println!("Error: {:?}", error),
            };
        } else {
            println!("{}", matches.usage());
        }
    }
}
