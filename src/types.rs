use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

use bson::Bson;
use clap::Parser;
use serde::Deserialize;
use tracing::error;

use crate::{error_exit, CONFIG};

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
    pub collections: Vec<String>,
    pub mongodb_types: bool,
}

pub type DataStructure = BTreeMap<String, TypeScriptType>;

#[derive(Eq, PartialEq, Ord, PartialOrd, Debug, Clone)]
pub enum TypeScriptType {
    Array(Box<TypeScriptType>),
    Object(DataStructure),
    Number,
    BigInt,
    Null,
    String,
    Buffer,
    Boolean,
    Any,
    ObjectId,
    Timestamp,
    DateTime,
    MaxKey,
    MinKey,
    Undefined,
    Union(BTreeSet<TypeScriptType>),
}

impl TypeScriptType {
    pub fn merge(&self, other: &Self) -> Self {
        let set = match (&self, &other) {
            (Self::Union(set_a), Self::Union(set_b)) => {
                let mut new_set = BTreeSet::new();
                set_a.union(set_b).for_each(|item| {
                    new_set.insert(item.clone());
                });
                new_set
            }
            (Self::Union(set_a), _) => {
                let mut new_set = set_a.clone();
                new_set.insert(other.clone());
                new_set
            }
            (_, Self::Union(set_b)) => {
                let mut new_set = set_b.clone();
                new_set.insert(self.clone());
                new_set
            }
            _ => BTreeSet::from([self.clone(), other.clone()]),
        };
        match set.len() {
            0 => Self::Undefined,
            1 => set.iter().next().unwrap_or(&Self::Undefined).clone(),
            _ => Self::Union(set),
        }
    }
}

impl FromIterator<Self> for TypeScriptType {
    fn from_iter<T: IntoIterator<Item = Self>>(iter: T) -> Self {
        let set = iter.into_iter().collect::<BTreeSet<_>>();

        match set.len() {
            0 => Self::Undefined,
            1 => set.iter().next().unwrap_or(&Self::Undefined).clone(),
            _ => Self::Union(set),
        }
    }
}

pub trait FromStructure<T> {
    fn convert(value: T) -> Self;
}

pub type FieldStructure = (String, TypeScriptType);

impl FromStructure<(String, Bson)> for FieldStructure {
    fn convert(value: (String, Bson)) -> Self {
        let (field_name, bson) = value;
        (field_name, TypeScriptType::from(bson))
    }
}

impl From<Bson> for TypeScriptType {
    fn from(value: Bson) -> Self {
        let mongodb_types = CONFIG
            .get()
            .unwrap_or_else(|| error_exit!("Unable to fetch the config", ""))
            .mongodb_types;

        match (value, mongodb_types) {
            (Bson::Array(array), _) => Self::Array(Box::from(
                array.into_iter().map(Self::from).collect::<Self>(),
            )),
            (Bson::Document(document), _) => {
                Self::Object(document.into_iter().map(FieldStructure::convert).collect())
            }
            (Bson::Double(_) | Bson::Int32(_), _) => Self::Number,
            (Bson::Int64(_) | Bson::Decimal128(_), _) => Self::BigInt,
            (Bson::String(_) | Bson::RegularExpression(_) | Bson::JavaScriptCode(_), _) => {
                Self::String
            }
            (Bson::Binary(_), _) => Self::Buffer,
            (Bson::Boolean(_), _) => Self::Boolean,
            (Bson::Null, _) => Self::Null,
            (Bson::Timestamp(_), true) => Self::Timestamp,
            (Bson::DateTime(_), true) => Self::DateTime,
            (Bson::MaxKey, true) => Self::MaxKey,
            (Bson::MinKey, true) => Self::MinKey,
            (Bson::ObjectId(_), true) => Self::ObjectId,
            (Bson::ObjectId(_), false) => Self::String,
            _ => Self::Any,
        }
    }
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
