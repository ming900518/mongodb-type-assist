use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Debug,
    path::PathBuf,
};

use bson::Bson;
use clap::Parser;
use serde::Deserialize;
use tracing::{error, info};

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
    pub pool_size: Option<u32>,
    pub collections: Option<Vec<String>>,
    pub mongodb_types: bool,
}

pub trait TypeScriptProducer {
    fn format_type(&self, path: Option<PathBuf>);
}

pub type CollectionStructure = BTreeMap<CollectionName, DataStructure>;

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone)]
pub struct CollectionName(pub String);

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone)]
pub struct FieldName(pub String);

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone)]
pub struct InnerFieldName(pub String);

impl Debug for CollectionName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "export class {} ", self.0)
    }
}

impl Debug for FieldName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}!", self.0)
    }
}

impl Debug for InnerFieldName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TypeScriptProducer for CollectionStructure {
    fn format_type(&self, _path: Option<PathBuf>) {
        for (field_name, structure) in self {
            info!("{field_name:?}{structure:#?}");
        }
    }
}

pub type DataStructure = BTreeMap<FieldName, TypeScriptType>;
pub type InnerDataStructure = BTreeMap<InnerFieldName, TypeScriptType>;

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone)]
pub enum TypeScriptType {
    Array(Box<TypeScriptType>),
    Object(InnerDataStructure),
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

impl Debug for TypeScriptType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.print_typescript())
    }
}

impl TypeScriptType {
    fn print_typescript(&self) -> String {
        match self {
            Self::Array(inner_type) => format!("{}[]", inner_type.print_typescript()),
            Self::Object(data_structure) => format!("{data_structure:#?}"),
            Self::Number => "number".into(),
            Self::BigInt => "BigInt".into(),
            Self::Null => "null".into(),
            Self::String => "string".into(),
            Self::Buffer => "Buffer".into(),
            Self::Boolean => "boolean".into(),
            Self::Any => "any".into(),
            Self::ObjectId => "ObjectId".into(),
            Self::Timestamp => "Timestamp".into(),
            Self::DateTime => "DateTime".into(),
            Self::MaxKey => "MaxKey".into(),
            Self::MinKey => "MinKey".into(),
            Self::Undefined => "undefined".into(),
            Self::Union(types) => types
                .iter()
                .map(Self::print_typescript)
                .collect::<Vec<_>>()
                .join(" | "),
        }
    }

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

pub type FieldStructure = (FieldName, TypeScriptType);

impl FromStructure<(String, Bson)> for FieldStructure {
    fn convert(value: (String, Bson)) -> Self {
        let (field_name, bson) = value;
        (FieldName(field_name), TypeScriptType::from(bson))
    }
}

pub type InnerFieldStructure = (InnerFieldName, TypeScriptType);

impl FromStructure<(String, Bson)> for InnerFieldStructure {
    fn convert(value: (String, Bson)) -> Self {
        let (field_name, bson) = value;
        (InnerFieldName(field_name), TypeScriptType::from(bson))
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
            (Bson::Document(document), _) => Self::Object(
                document
                    .into_iter()
                    .map(InnerFieldStructure::convert)
                    .collect(),
            ),
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
