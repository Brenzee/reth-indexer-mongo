use ::mongodb::Database;
use alloy::primitives::{keccak256, Address, Bloom, B256};
use alloy::rpc::types::{FilterSet, FilteredParams};
use config::{ABIItem, IndexerConfig, IndexerContractMapping};
use decoder::decode_logs;
use log::info;
use mongodb::{init_mongodb, insert_logs};
use reth_chainspec::ChainSpecBuilder;
use reth_db::mdbx::{DatabaseArguments, MaxReadTransactionDuration};
use reth_db::{open_db_read_only, DatabaseEnv};
use reth_node_ethereum::EthereumNode;
use reth_node_types::NodeTypesWithDBAdapter;
use reth_primitives::{Header, Log};
use reth_provider::{
    providers::StaticFileProvider, BlockReader, HeaderProvider, ProviderFactory, ReceiptProvider,
    TransactionsProvider,
};
use std::fs::File;
use std::io::Read;
use std::time::Instant;
use std::{path::Path, sync::Arc};
use tokio::time::Duration;

mod config;
mod decoder;
mod mongodb;

// Univ2 factory 10000835

/// Loads the indexer configuration from the "reth-indexer-config.json" file.
/// Returns the loaded `IndexerConfig` if successful.
/// Panics if the file does not exist or if there is an error reading or parsing the file.
fn load_indexer_config(file_path: &Path) -> IndexerConfig {
    let mut file = File::open(file_path)
        .unwrap_or_else(|_| panic!("Failed to find config file at path - {:?}", file_path));

    let mut content = String::new();
    file.read_to_string(&mut content)
        .expect("Failed to config.json file");

    let config: IndexerConfig =
        serde_json::from_str(&content).expect("Failed to parse config.json JSON");

    config
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let config: String = std::env::var("CONFIG").unwrap_or("./config.json".to_string());
    println!("Config: {}", config);

    let log_config: IndexerConfig = load_indexer_config(Path::new(&config));
    println!("log_config: {:#?}", log_config);

    sync(&log_config).await?;

    Ok(())
}
async fn sync(config: &IndexerConfig) -> eyre::Result<()> {
    info!("Starting indexer...");

    let from_block = config.from_block;
    let to_block = config.to_block;

    let db_path = Path::new(&config.reth_db_location);
    let read_tx_duration = MaxReadTransactionDuration::Set(Duration::from_secs(30));
    let database_args =
        DatabaseArguments::default().with_max_read_transaction_duration(Some(read_tx_duration));
    let db = open_db_read_only(db_path.join("db").as_path(), database_args)?;
    info!("Opened db");

    println!("Initializing MongoDB");
    let mongodb = init_mongodb(&config.mongodb, &config.event_mappings).await?;
    info!("Initialized MongoDB");
    let spec = ChainSpecBuilder::mainnet().build();
    let factory = ProviderFactory::<NodeTypesWithDBAdapter<EthereumNode, Arc<DatabaseEnv>>>::new(
        db.into(),
        spec.into(),
        StaticFileProvider::read_only(db_path.join("static_files"), true)?,
    );

    let provider = factory.provider()?;

    info!("MongoDB Syncing...");
    let start = Instant::now();

    for block_number in from_block..to_block {
        info!("Checking block {}", block_number);
        match provider.header_by_number(block_number).unwrap() {
            None => {
                log::warn!("Block {} not found", block_number);
                continue;
            }
            Some(block_header) => {
                for mapping in &config.event_mappings {
                    // If the event needs to be filtered by a specific contract address
                    if let Some(contract_addr) = &mapping.filter_by_contract_addresses {
                        if !contract_addr
                            .iter()
                            .any(|address| contract_in_bloom(*address, block_header.logs_bloom))
                        {
                            continue;
                        }
                    }

                    if !mapping.decode_abi_items.iter().any(|abi_item| {
                        topic_in_bloom(abi_item_topic_id(abi_item), block_header.logs_bloom)
                    }) {
                        continue;
                    }

                    process_block(&provider, &mongodb, &mapping, &block_header, block_number).await;
                }
            }
        }
    }

    info!("MongoDB sync is done");
    let duration = start.elapsed();
    println!("Time taken: {:.2}", duration.as_secs_f32());

    Ok(())
}

fn contract_in_bloom(contract_address: Address, logs_bloom: Bloom) -> bool {
    let filter_set = FilterSet::from(contract_address);
    let address_filter = FilteredParams::address_filter(&filter_set);

    FilteredParams::matches_address(logs_bloom, &address_filter)
}

fn topic_in_bloom(topic: B256, logs_bloom: Bloom) -> bool {
    let filter_set = FilterSet::from(topic);
    let topic_filter = FilteredParams::topics_filter(&[filter_set]);

    FilteredParams::matches_topics(logs_bloom, &topic_filter)
}

fn abi_item_topic_id(item: &ABIItem) -> B256 {
    let input_types: Vec<String> = item
        .inputs
        .iter()
        .map(|input| input.type_.clone())
        .collect();

    keccak256(format!("{}({})", item.name, input_types.join(",")))
}

async fn process_block<T: ReceiptProvider + HeaderProvider + BlockReader + TransactionsProvider>(
    provider: &T,
    mongodb: &Database,
    mapping: &IndexerContractMapping,
    header: &Header,
    block_number: u64,
) {
    let block_indecies = provider
        .block_body_indices(block_number)
        .unwrap_or_else(|e| {
            eprintln!("Error fetching block {}: {}", block_number, e);
            // panic!("Failed to fetch block indices");
            return None;
        });

    if let Some(block_indecies) = block_indecies {
        for tx_id in
            block_indecies.first_tx_num..block_indecies.first_tx_num + block_indecies.tx_count
        {
            let receipt = match provider.receipt(tx_id) {
                Ok(Some(receipt)) => receipt,
                _ => continue,
            };

            let logs: Vec<Log> =
                if let Some(contract_addresses) = &mapping.filter_by_contract_addresses {
                    receipt
                        .logs
                        .iter()
                        .filter(|log| {
                            contract_addresses
                                .iter()
                                .any(|address| address == &log.address)
                        })
                        .cloned()
                        .collect()
                } else {
                    receipt.logs
                };

            if logs.is_empty() {
                continue;
            }

            process_tx(provider, mongodb, mapping, header, tx_id, &logs).await;
        }
    }
}

async fn process_tx<T: ReceiptProvider + HeaderProvider + BlockReader + TransactionsProvider>(
    provider: &T,
    mongodb: &Database,
    mapping: &IndexerContractMapping,
    header: &Header,
    tx_id: u64,
    logs: &[Log],
) {
    let tx = match provider.transaction_by_id_no_hash(tx_id) {
        Ok(Some(tx)) => tx,
        _ => return,
    };

    for abi_item in &mapping.decode_abi_items {
        let topic_id = abi_item_topic_id(abi_item);
        if !topic_in_bloom(topic_id, header.logs_bloom) {
            continue;
        }

        let decoded_logs = decode_logs(topic_id, logs, abi_item);
        if decoded_logs.is_empty() {
            continue;
        }

        match insert_logs(
            mongodb,
            &abi_item.collection_name,
            header,
            &tx,
            &decoded_logs,
        )
        .await
        {
            Ok(_) => (),
            Err(e) => log::error!("Error inserting logs: {}", e),
        }
    }
}

// fn sync_events<T: ReceiptProvider + HeaderProvider + BlockReader + TransactionsProvider>(
//     config: &IndexerConfig,
//     provider: &T,
// ) -> eyre::Result<()> {
//     // TODO: The goal is as follows:
//     // 1. fromBlockNumber and toBlockNumber - this is the range of blocks to check
//     // 2. in config needs to add mongodb support
//     let block_number = 21116342;
//     const USDC: Address = address!("a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48");

//     let data = provider
//         .header_by_number(block_number)?
//         .ok_or(eyre::eyre!("block not found"))?;
//     println!("block header: {:#?}", data);
//     let start = Instant::now();

//     let filter_set = FilterSet::from(USDC);
//     let address_filter = FilteredParams::address_filter(&filter_set);
//     let has_usdc = FilteredParams::matches_address(data.logs_bloom, &address_filter);

//     if has_usdc == false {
//         println!("Block doesn't have USDC logs");
//         return Ok(());
//     } else {
//         println!("Block has USDC logs");
//     }

//     let block_indecies = provider.block_body_indices(block_number)?.unwrap();
//     for tx_id in block_indecies.first_tx_num..block_indecies.first_tx_num + block_indecies.tx_count
//     {
//         // let tx = provider.transaction_by_id_no_hash(tx_id)?.unwrap();
//         // println!("tx: {:?}", tx);
//         let receipt = provider.receipt(tx_id)?.unwrap();

//         let logs: Vec<Log> = receipt
//             .logs
//             .iter()
//             .filter(|log| USDC == log.address)
//             .cloned()
//             .collect();

//         if logs.is_empty() {
//             continue;
//         }

//         let tx = provider.transaction_by_id(tx_id)?.unwrap();

//         println!("Transaction {} had USDC logs {:#?}", tx.hash, logs);

//         // if logs.is_emp
//     }
//     let duration = start.elapsed();
//     println!("Time taken: {} ms", duration.as_millis());

//     // data.logs
//     // let rpc_bloom: Bloom = Bloom::from_str(&format!("{:?}", header_tx_info.logs_bloom)).unwrap();

//     // let tx_hash: FixedBytes<32> =
//     //     "0xd39613e81e6f3976770a038980ce271a1223c41c823257a535def515dda76559".parse()?;
//     // let tx = provider
//     //     .transaction_by_hash(tx_hash)?
//     //     .ok_or(eyre::eyre!("did not get tx"))?;
//     // println!("tx: {:#?}", tx);

//     Ok(())
// }
