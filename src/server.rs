use std::sync::Arc;

use prost_types::compiler::code_generator_response::File;
use tonic::transport::Server;
use tonic::{Request, Response, Status};

use runison::synchronizer_server::{Synchronizer, SynchronizerServer};

use runison::{ChangeSetResponse, Entries};
use std::path::PathBuf;
use structopt::StructOpt;

pub mod runison {
    tonic::include_proto!("runison");
}

mod config;
mod node;
mod synchronizer;
use synchronizer::Synchronizer as FileSynchronizer;

#[derive(Debug, StructOpt)]
#[structopt(name = "runison-server", about = "A modern file synchronization tool.")]
struct Opt {
    /// Activate debug mode
    // short and long flags (-d, --debug) will be deduced from the field's name
    #[structopt(short, long)]
    debug: bool,

    /// Configuration file
    #[structopt(short = "c", long = "config", parse(from_os_str))]
    config: PathBuf,
}
#[derive(Copy, Clone)]
struct SyncWrapper<'a> {
    sync_ref: &'a FileSynchronizer,
}
pub struct SynchronizerService<'a> {
    synchronizer: SyncWrapper<'a>,
}
#[tonic::async_trait]
impl Synchronizer for SynchronizerService {
    async fn get_change_set(
        &self,
        request: Request<Entries>,
    ) -> Result<Response<ChangeSetResponse>, Status> {
        println!("GetChangeSet = {:?}", request);
        &self
            .synchronizer
            .sync_ref
            .remote_changes(request.into_inner());
        Ok(Response::new(ChangeSetResponse::default()))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();
    println!("{:?}", opt);
    let result = config::get_config(opt.config);

    match result {
        Ok(config) => {
            let addr = "[::1]:10000".parse().unwrap();

            println!("Synchronizer listening on: {}", addr);

            let synchronizer = SynchronizerService {
                synchronizer: SyncWrapper { sync_ref: &synch },
            };

            let svc = SynchronizerServer::new(synchronizer);

            Server::builder().add_service(svc).serve(addr).await;
        }

        Err(_) => {}
    }
    Ok(())
}
