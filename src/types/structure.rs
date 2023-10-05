use std::{
    collections::BTreeMap,
    fmt::{Debug, Display},
    fs::create_dir_all,
    path::PathBuf,
};

use bson::Bson;
use tracing::{error, info};

use super::typescript::{TypeScriptProducer, TypeScriptType};

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone)]
pub struct CollectionStruct(pub BTreeMap<CollectionName, DataStruct>);

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone)]
pub struct DataStruct(pub BTreeMap<FieldName, TypeScriptType>);

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone)]
pub struct InnerDataStruct(pub BTreeMap<InnerFieldName, TypeScriptType>);

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone)]
pub struct CollectionName(pub String);

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone)]
pub struct FieldName(pub String);

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone)]
pub struct InnerFieldName(pub String);

impl Debug for DataStruct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (field_name, structure) in &self.0 {
            writeln!(f, "  {field_name:?}!: {structure:#?};").ok();
        }
        Ok(())
    }
}

impl Debug for InnerDataStruct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#?}", self.0)
    }
}

impl Debug for CollectionName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut collection_name = self.0.clone().chars().collect::<Vec<_>>();
        let first_letter = collection_name.remove(0);
        collection_name.insert(0, first_letter.to_ascii_uppercase());
        writeln!(
            f,
            "export class {} {{",
            collection_name.into_iter().collect::<String>()
        )
    }
}

impl Display for CollectionName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Debug for FieldName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "  {}", self.0)
    }
}

impl Debug for InnerFieldName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "    {}", self.0)
    }
}

impl TypeScriptProducer for CollectionStruct {
    fn format_type(&self, path_option: Option<PathBuf>) {
        for (collection_name, structure) in &self.0 {
            let print_result = format!("{collection_name:?}{structure:#?}}}");
            match path_option {
                Some(ref path) => {
                    let mut path = path.clone();
                    if !path.exists() {
                        create_dir_all(&path).unwrap_or_else(|error| {
                            error!(
                                "Unable to create the directories required by operation: {error}"
                            );
                        });
                    }

                    path.push(format!("{collection_name}.ts"));

                    let path_str = path.to_str().unwrap_or("unknown path");

                    std::fs::write(&path, print_result).map_or_else(
                        |error| {
                            error!(
                                "Unable to produce collection {collection_name}'s type definition to {path_str}: {error}"
                            );
                        },
                        |()| info!("Collection {collection_name}'s type definition has been saved to {path_str}."));
                }
                None => {
                    info!(
                        "TypeScript type for collection {}\n{print_result}",
                        collection_name
                    );
                }
            }
        }
    }
}

pub trait FromStruct<T> {
    fn convert(value: T) -> Self;
}

pub type FieldStruct = (FieldName, TypeScriptType);

impl FromStruct<(String, Bson)> for FieldStruct {
    fn convert(value: (String, Bson)) -> Self {
        let (field_name, bson) = value;
        (FieldName(field_name), TypeScriptType::from(bson))
    }
}

pub type InnerFieldStruct = (InnerFieldName, TypeScriptType);

impl FromStruct<(String, Bson)> for InnerFieldStruct {
    fn convert(value: (String, Bson)) -> Self {
        let (field_name, bson) = value;
        (InnerFieldName(field_name), TypeScriptType::from(bson))
    }
}
