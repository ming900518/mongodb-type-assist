use std::{collections::BTreeSet, fmt::Debug, path::PathBuf};

use bson::Bson;
use tracing::error;

use crate::{error_exit, CONFIG};

use super::structure::{FromStruct, InnerDataStruct, InnerFieldStruct};

pub trait TypeScriptProducer {
    fn format_type(&self, path: Option<PathBuf>);
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone)]
pub enum TypeScriptType {
    Array(Box<TypeScriptType>),
    Object(InnerDataStruct),
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
            Self::BigInt => "bigint".into(),
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
            (Bson::Document(document), _) => Self::Object(InnerDataStruct(
                document
                    .into_iter()
                    .map(InnerFieldStruct::convert)
                    .collect(),
            )),
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
