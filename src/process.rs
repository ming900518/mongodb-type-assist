use std::{collections::BTreeMap, sync::Mutex};

use bson::Document;
use mongodb::sync::Database;
use rayon::prelude::*;
use tracing::{error, info, warn};

use crate::{
    error_exit,
    types::structure::{CollectionName, CollectionStruct, DataStruct, FieldStruct, FromStruct},
};

pub fn parse_collections(db: &Database, collections: Vec<String>) -> CollectionStruct {
    let set = collections.into_par_iter().filter_map(|collection| {
        info!("Processing: {collection}");
        let collection_fields = Mutex::new(DataStruct(BTreeMap::new()));
        db.collection(&collection).find(None, None).map_or_else(
            |error| error!("Error when fetching documents in collecton {collection}: {error}"),
            |cursor| {
                cursor.for_each(|result| {
                    result.map_or_else(
                        |error| {
                            warn!("Document in {collection} contains error. Cause: {error}");
                        },
                        |document| process_document(&collection_fields, document),
                    );
                });
            },
        );
        info!("Done processing: {collection}");
        collection_fields.into_inner().map_or_else(|error| {
            error!("Error when getting the value stored in mutex, resulting collection {collection} could not be processed: {error}");
            None
        }, |data| Some((CollectionName(collection.clone()), data)))
    }).collect();
    CollectionStruct(set)
}

fn process_document(collection_fields: &Mutex<DataStruct>, document: Document) {
    let mut collection_fields = collection_fields
        .lock()
        .unwrap_or_else(|error| error_exit!("Unable to lock the mutex", error));
    document.into_iter().for_each(|field| {
        let (field_name, mut new_types) = FieldStruct::convert(field);
        if let Some(orig_types) = collection_fields.0.get(&field_name) {
            new_types = orig_types.merge(&new_types);
        }
        collection_fields.0.insert(field_name, new_types);
    });
}
