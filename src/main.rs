#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(dead_code)]

mod process;
mod types;

use std::{error::Error, fs::File, io::BufReader, panic, path::PathBuf, sync::OnceLock};

use clap::Parser;
use mongodb::{
    options::{ClientOptions, ConnectionString},
    sync::Client,
};
use serde_json::from_reader;
use tracing::{debug, error, warn};
use types::{typescript::TypeScriptProducer, Cli, Config, FilterConfig};

use crate::process::parse_collections;

static CONFIG: OnceLock<Config> = OnceLock::new();

fn main() {
    panic::set_hook(Box::new(|panic_info| {
        debug!("{:#?}", panic_info);
        error!("Operation has been canceled.");
    }));

    tracing_subscriber::fmt::init();

    let params = Cli::parse();

    let config = CONFIG.get_or_init(|| {
        File::open(
            params
                .config_file
                .unwrap_or_else(|| PathBuf::from("./config.json")),
        )
        .map_err(Box::from)
        .and_then(|file| from_reader(BufReader::new(file)).map_err(Box::from))
        .unwrap_or_else(|error: Box<dyn Error>| error_exit!("Error when processing config", error))
    });

    let db = Client::with_options({
        let mut options = ClientOptions::parse_connection_string_sync(
            ConnectionString::parse(&config.uri).unwrap_or_else(|error| {
                error_exit!("Unable to parse MongoDB connection string", error)
            }),
        )
        .unwrap_or_else(|error| error_exit!("Error when processing MongoDB options", error));
        options.max_pool_size = config.pool_size;
        options
    })
    .unwrap_or_else(|error| error_exit!("Error when processing config", error))
    .database(&config.database);

    let collections = db.list_collections(None, None).map_or_else(
        |error| error_exit!("Error when fetching collections", error),
        |collection| {
            collection
                .into_iter()
                .filter_map(|data| {
                    data.ok().and_then(|value| match config.collection_filter {
                        FilterConfig::Include { ref collections } => {
                            if collections.contains(&value.name) {
                                Some(value.name)
                            } else {
                                None
                            }
                        }
                        FilterConfig::Exclude { ref collections } => {
                            if collections.contains(&value.name) {
                                None
                            } else {
                                Some(value.name)
                            }
                        }
                        FilterConfig::All => Some(value.name),
                    })
                })
                .collect()
        },
    );

    parse_collections(&db, collections).format_type(params.output);
}

#[macro_export]
macro_rules! error_exit {
    ($message: expr, $error: expr) => {{
        let error = $error;
        let message = $message;
        error!("{message}: {error}");
        panic!("{error}");
    }};
}
