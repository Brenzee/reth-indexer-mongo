use alloy::primitives::Address;
use serde::Deserialize;
use std::path::PathBuf;

/// Represents an input parameter in the ABI.
#[derive(Debug, Deserialize, Clone)]
pub struct ABIInput {
    /// Indicates if the input parameter is indexed.
    pub indexed: bool,

    /// The name of the input parameter.
    pub name: String,

    /// The type of the input parameter.
    #[serde(rename = "type")]
    pub type_: String,
    // NOTE: Not sure if necessary
    //#[serde(
    //    // deserialize_with = "deserialize_regex_option",
    //    rename = "rethRegexMatch"
    //)]
    //pub regex: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CustomDbIndex {
    /// True = 1, False = -1
    #[serde(rename = "sortAsc")]
    pub sort_asc: bool,
    #[serde(rename = "indexField")]
    pub index_field: String,
}

/// Represents an item in the ABI.
#[derive(Debug, Deserialize, Clone)]
pub struct ABIItem {
    /// The list of input parameters for the ABI item.
    pub inputs: Vec<ABIInput>,

    /// The name of the ABI item.
    pub name: String,

    /// The name of the collection to store the ABI item in.
    #[serde(rename = "collectionName")]
    pub collection_name: String,

    /// Apply custom indexes to the database
    #[serde(rename = "customDbIndexes")]
    pub custom_db_indexes: Option<Vec<Vec<CustomDbIndex>>>,
}

/// Represents a contract mapping in the Indexer.
#[derive(Debug, Deserialize, Clone)]
pub struct IndexerContractMapping {
    /// The contract address.
    #[serde(rename = "filterByContractAddress")]
    // pub contract_address: Option<Address>,
    pub filter_by_contract_addresses: Option<Vec<Address>>,

    /// The list of ABI items to decode.
    #[serde(rename = "decodeAbiItems")]
    pub decode_abi_items: Vec<ABIItem>,
}

// For drop_tables
fn default_false() -> bool {
    false
}

/// Represents a contract mapping in the Indexer.
#[derive(Debug, Deserialize)]
pub struct IndexerMongoDBConfig {
    /// The MongoDB connection string.
    #[serde(rename = "connectionString")]
    pub connection_string: String,

    /// The database name.
    pub database: String,

    /// If true, the tables will be dropped and recreated before syncing.
    #[serde(rename = "dropTableBeforeSync")]
    #[serde(default = "default_false")]
    pub drop_tables: bool,
}

#[derive(Debug, Deserialize)]
pub struct IndexerConfig {
    /// The location of the Reth DB.
    #[serde(rename = "rethDBLocation")]
    pub reth_db_location: PathBuf,

    /// The block number from which the script should start from.
    #[serde(rename = "fromBlockNumber")]
    pub from_block: u64,

    // TODO: Make the to_block optional. Optional -> follow head block
    /// The starting block number.
    /// For now to_block is required
    #[serde(rename = "toBlockNumber")]
    pub to_block: u64,

    /// The mongodb configuration.
    pub mongodb: IndexerMongoDBConfig,

    /// The list of contract mappings.
    #[serde(rename = "eventMappings")]
    pub event_mappings: Vec<IndexerContractMapping>,
}
