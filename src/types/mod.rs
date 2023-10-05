use std::path::PathBuf;

use clap::Parser;
use serde::Deserialize;

pub mod structure;
pub mod typescript;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[arg(value_name = "CONFIG JSON FILE")]
    pub config_file: Option<PathBuf>,

    #[arg(short, long, value_name = "DIRECTORY")]
    pub output: Option<PathBuf>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub uri: String,
    pub database: String,
    pub pool_size: Option<u32>,
    pub collections: Option<Vec<String>>,
    pub mongodb_types: bool,
}
