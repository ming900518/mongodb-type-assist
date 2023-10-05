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
use types::{Cli, Config, TypeScriptProducer};

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

    let collections = if config.collections.is_none()
        | config.collections.clone().is_some_and(|vec| vec.is_empty())
    {
        db.list_collections(None, None).map_or_else(
            |error| error_exit!("Error when fetching collections", error),
            |collection| {
                Some(
                    collection
                        .into_iter()
                        .filter_map(|data| data.ok().map(|value| value.name))
                        .collect(),
                )
            },
        )
    } else {
        config.collections.clone()
    }
    .unwrap_or_else(|| error_exit!("No collections avaliable.", ""));

    parse_collections(&db, collections).format_type(params.output);
}
