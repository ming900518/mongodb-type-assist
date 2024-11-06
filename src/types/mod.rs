use std::path::PathBuf;

use clap::Parser;
use serde::{Deserialize, Serialize};

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

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub uri: String,
    pub database: String,
    pub pool_size: Option<u32>,
    #[serde(default)]
    pub collection_filter: FilterConfig,
    pub mongodb_types: bool,
    pub parse_field_as_map: Option<Vec<ParseAsMap>>,
}

impl Config {
    pub fn example() -> Self {
        Self {
            uri: "mongodb://username:password@ip:port/?replicaSet=rs0&directConnection=true"
                .to_owned(),
            database: "database_name".to_owned(),
            pool_size: Some(10),
            collection_filter: FilterConfig::Exclude {
                collections: vec!["excluded_collection".to_owned()],
            },
            mongodb_types: false,
            parse_field_as_map: Some(vec![ParseAsMap {
                collection: "collection_name".to_owned(),
                field: "kv_store".to_owned(),
            }]),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(tag = "type")]
pub enum FilterConfig {
    Include {
        collections: Vec<String>,
    },
    Exclude {
        collections: Vec<String>,
    },
    #[default]
    All,
}

#[derive(Eq, PartialEq, Serialize, Deserialize, Debug, Default, Clone)]
pub struct ParseAsMap {
    pub collection: String,
    pub field: String,
}

impl ParseAsMap {
    pub fn new<T: Into<String>>(collection: T, field: T) -> Self {
        Self {
            collection: collection.into(),
            field: field.into(),
        }
    }
}
