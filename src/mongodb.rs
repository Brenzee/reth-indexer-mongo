use mongodb::{
    bson::{self, doc, DateTime, Document},
    options::{ClientOptions, ResolverConfig},
    Client, Collection, Database, IndexModel,
};
use reth_primitives::{Header, TransactionSigned, TransactionSignedNoHash};

use crate::{
    config::{IndexerContractMapping, IndexerMongoDBConfig},
    decoder::DecodedLog,
};

pub async fn init_mongodb(
    config: &IndexerMongoDBConfig,
    event_mappings: &[IndexerContractMapping],
) -> eyre::Result<Database> {
    let options = ClientOptions::parse(&config.connection_string).await?;
    let client = Client::with_options(options)?;
    let db = client.database(&config.database);
    // Need to create tables
    create_collections(&db, config, event_mappings).await?;
    Ok(db)
}

async fn create_collections(
    db: &Database,
    config: &IndexerMongoDBConfig,
    event_mappings: &[IndexerContractMapping],
) -> eyre::Result<()> {
    for mapping in event_mappings {
        for abi_item in &mapping.decode_abi_items {
            let collection_name = &abi_item.collection_name;

            if config.drop_tables {
                println!("Dropping collection: {}", collection_name);
                db.collection::<Document>(collection_name).drop().await?;
            }

            db.create_collection(collection_name).await?;
            println!("Created collection: {}", collection_name);

            if let Some(custom_db_indexes) = &abi_item.custom_db_indexes {
                for index in custom_db_indexes {
                    let index = IndexModel::builder()
                        .keys(Document::from_iter(index.iter().map(|i| {
                            (i.index_field.clone(), bson::Bson::Int32(i.sort_asc as i32))
                        })))
                        .build();
                    db.collection::<Document>(collection_name)
                        .create_index(index)
                        .await?;
                }
            }
        }
    }
    println!("Created all collections");
    Ok(())
}

pub async fn insert_logs(
    db: &Database,
    collection_name: &str,
    header: &Header,
    tx: &TransactionSignedNoHash,
    logs: &[DecodedLog],
) -> eyre::Result<()> {
    let collection: Collection<Document> = db.collection(collection_name);

    let block_hash = header.hash_slow().to_string();
    let block_number = match bson::to_bson(&(header.number as i64)) {
        Ok(block_number) => block_number,
        Err(_) => bson::to_bson(&header.number.to_string()).unwrap(),
    };

    let docs: Vec<Document> = logs
        .iter()
        .map(|log| {
            let timestamp = DateTime::from_millis((header.timestamp as i64) * 1000);
            let mut doc = doc! {
                "block_number": block_number.clone(),
                "contract_address": log.address.to_string(),
                "tx_hash": tx.hash().to_string(),
                "block_hash": block_hash.clone(),
                "timestamp": timestamp,
            };

            for topic in &log.topics {
                doc.insert(&topic.name, &topic.value);
            }

            doc
        })
        .collect();

    collection.insert_many(docs).await?;
    Ok(())
}
