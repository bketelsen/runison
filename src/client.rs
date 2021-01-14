mod config;
mod node;
mod synchronizer;

use synchronizer::Synchronizer;
use tonic::Request;

use runison::synchronizer_client::SynchronizerClient;
use runison::{ChangeSetResponse, Entries, Feature, Point, Rectangle, RouteNote, RouteSummary};

pub mod runison {
    tonic::include_proto!("runison");
}

use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "runison", about = "A modern file synchronization tool.")]
struct Opt {
    /// Activate debug mode
    // short and long flags (-d, --debug) will be deduced from the field's name
    #[structopt(short, long)]
    debug: bool,

    /// Synchronization server
    // we don't want to name it "speed", need to look smart
    #[structopt(short = "s", long = "server")]
    server: String,

    /// Configuration file
    #[structopt(short = "c", long = "config", parse(from_os_str))]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();
    println!("{:?}", opt);
    let result = config::get_config(opt.config);
    match result {
        Ok(config) => {
            // create a synchronizer
            let mut synchronizer = Synchronizer::new(config).unwrap();
            // index local files
            synchronizer.index();

            // create a client
            let mut client = SynchronizerClient::connect("http://[::1]:10000").await?;

            println!("*** Get ChangeSet ***");
            let response = client
                .get_change_set(Request::new(Entries {
                    nodes: synchronizer.entries.nodes,
                }))
                .await?;
            println!("RESPONSE = {:?}", response);
        }
        Err(error) => {
            println!("Error: {:?}", error);
        }
    };
    Ok(())
}
