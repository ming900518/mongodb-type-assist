use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Mutex,
};

use bson::Document;
use mongodb::sync::Database;
use rayon::prelude::*;
use tracing::{error, info, warn};

use crate::{
    error_exit,
    types::{
        structure::{
            CollectionName, CollectionStruct, FieldName, FieldStruct, FromStruct, ObjectStruct,
        },
        typescript::TypeScriptType,
        ParseAsMap,
    },
    CONFIG,
};

pub fn parse_collections(db: &Database, collections: Vec<String>) -> CollectionStruct {
    let set = collections.into_par_iter().filter_map(|collection| {
        info!("Processing: {collection}");
        let collection_fields = Mutex::new(ObjectStruct(BTreeMap::new()));
        db.collection(&collection).find(None, None).map_or_else(
            |error| error!("Error when fetching documents in collecton {collection}: {error}"),
            |cursor| {
                let mut documents = cursor.filter_map(|result|{
                    result.map_or_else(
                        |error| {warn!("Document in {collection} contains error. Cause: {error}"); None},
                        Some,
                    )
                }).collect::<Vec<Document>>();

                documents.sort_by_key(|b| std::cmp::Reverse(std::mem::size_of_val(b)));

                documents.into_iter().for_each(|document| process_document(&collection, &collection_fields, document));
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

fn process_document(
    collection_name: &str,
    collection_fields: &Mutex<ObjectStruct>,
    document: Document,
) {
    let parse_field_as_map = CONFIG
        .get()
        .and_then(|config| config.parse_field_as_map.clone())
        .unwrap_or_default();

    let mut orig_field_names = collection_fields
        .lock()
        .unwrap_or_else(|error| error_exit!("Unable to lock the mutex", error))
        .0
        .keys()
        .map(|field_name| field_name.0.clone())
        .collect::<BTreeSet<String>>();

    document.into_iter().for_each(|field| {
        let (field_name, mut new_types) =
            if parse_field_as_map.contains(&ParseAsMap::new(collection_name, &field.0)) {
                (FieldName(field.0), TypeScriptType::Map)
            } else {
                FieldStruct::convert(field)
            };

        if let Some(orig_types) = collection_fields
            .lock()
            .unwrap_or_else(|error| error_exit!("Unable to lock the mutex", error))
            .0
            .get(&field_name)
        {
            new_types = orig_types.merge(&new_types);
        }

        collection_fields
            .lock()
            .unwrap_or_else(|error| error_exit!("Unable to lock the mutex", error))
            .0
            .insert(field_name.clone(), new_types);
        orig_field_names.remove(&field_name.0);
    });

    for field_name in orig_field_names {
        let mut new_types = TypeScriptType::Undefined;

        if let Some(orig_types) = collection_fields
            .lock()
            .unwrap_or_else(|error| error_exit!("Unable to lock the mutex", error))
            .0
            .get(&FieldName(field_name.clone()))
        {
            new_types = orig_types.merge(&new_types);
        }

        collection_fields
            .lock()
            .unwrap_or_else(|error| error_exit!("Unable to lock the mutex", error))
            .0
            .insert(FieldName(field_name), new_types);
    }
}
