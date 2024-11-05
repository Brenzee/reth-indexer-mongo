use alloy::primitives::{address, Address};
// use alloy::primitives::{Address, Bloom, FixedBytes, Sealable, B256};
// use alloy::providers::Provider;
use alloy::rpc::types::{Filter, FilterSet, FilteredParams, ValueOrArray};
use reth_chainspec::ChainSpecBuilder;
use reth_db::{open_db_read_only, DatabaseEnv};
use reth_node_ethereum::EthereumNode;
use reth_node_types::NodeTypesWithDBAdapter;
use reth_primitives::Log;
// use reth_primitives::SealedHeader;
use reth_provider::{
    providers::StaticFileProvider, AccountReader, BlockReader, BlockSource,
    BlockchainTreePendingStateProvider, HeaderProvider, ProviderFactory, ReceiptProvider,
    StateProvider, TransactionsProvider,
};
use reth_provider::{ChainSpecProvider, ChainStateBlockReader, FullRpcProvider};
use std::time::Instant;
use std::{path::Path, sync::Arc};

mod config;

// Providers are zero cost abstractions on top of an opened MDBX Transaction
// exposing a familiar API to query the chain's information without requiring knowledge
// of the inner tables.
//
// These abstractions do not include any caching and the user is responsible for doing that.
// Other parts of the code which include caching are parts of the `EthApi` abstraction.
fn main() -> eyre::Result<()> {
    // Opens a RO handle to the database file.
    // This is /root/.local/share/reth/mainnet
    let db_path = std::env::var("RETH_DB_PATH")?;
    let db_path = Path::new(&db_path);
    let db = open_db_read_only(db_path.join("db").as_path(), Default::default())?;

    // Instantiate a provider factory for Ethereum mainnet using the provided DB.
    // TODO: Should the DB version include the spec so that you do not need to specify it here?
    let spec = ChainSpecBuilder::mainnet().build();
    let factory = ProviderFactory::<NodeTypesWithDBAdapter<EthereumNode, Arc<DatabaseEnv>>>::new(
        db.into(),
        spec.into(),
        StaticFileProvider::read_only(db_path.join("static_files"), true)?,
    );

    // This call opens a RO transaction on the database. To write to the DB you'd need to call
    // the `provider_rw` function and look for the `Writer` variants of the traits.
    let provider = factory.provider()?;

    // const range =

    sync_events(&provider)?;

    // Closes the RO transaction opened in the `factory.provider()` call. This is optional and
    // would happen anyway at the end of the function scope.
    drop(provider);

    Ok(())
}

fn sync_events<T: ReceiptProvider + HeaderProvider + BlockReader + TransactionsProvider>(
    provider: &T,
) -> eyre::Result<()> {
    let block_number = 21116342;
    const USDC: Address = address!("a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48");

    let data = provider
        .header_by_number(block_number)?
        .ok_or(eyre::eyre!("block not found"))?;
    println!("block header: {:#?}", data);
    let start = Instant::now();

    let filter_set = FilterSet::from(USDC);
    let address_filter = FilteredParams::address_filter(&filter_set);
    let has_usdc = FilteredParams::matches_address(data.logs_bloom, &address_filter);

    if has_usdc == false {
        println!("Block doesn't have USDC logs");
        return Ok(());
    } else {
        println!("Block has USDC logs");
    }

    let block_indecies = provider.block_body_indices(block_number)?.unwrap();
    for tx_id in block_indecies.first_tx_num..block_indecies.first_tx_num + block_indecies.tx_count
    {
        // let tx = provider.transaction_by_id_no_hash(tx_id)?.unwrap();
        // println!("tx: {:?}", tx);
        let receipt = provider.receipt(tx_id)?.unwrap();

        let logs: Vec<Log> = receipt
            .logs
            .iter()
            .filter(|log| USDC == log.address)
            .cloned()
            .collect();

        if logs.is_empty() {
            continue;
        }

        let tx = provider.transaction_by_id(tx_id)?.unwrap();

        println!("Transaction {} had USDC logs {:#?}", tx.hash, logs);

        // if logs.is_emp
    }
    let duration = start.elapsed();
    println!("Time taken: {} ms", duration.as_millis());

    // data.logs
    // let rpc_bloom: Bloom = Bloom::from_str(&format!("{:?}", header_tx_info.logs_bloom)).unwrap();

    // let tx_hash: FixedBytes<32> =
    //     "0xd39613e81e6f3976770a038980ce271a1223c41c823257a535def515dda76559".parse()?;
    // let tx = provider
    //     .transaction_by_hash(tx_hash)?
    //     .ok_or(eyre::eyre!("did not get tx"))?;
    // println!("tx: {:#?}", tx);

    Ok(())
}

// async fn process_block<T: ReceiptProvider + TransactionsProvider + HeaderProvider + BlockReader>(
//     provider: T,
//     csv_writers: &mut [CsvWriter],
//     db_writers: &Vec<Box<dyn DatasourceWritable>>,
//     mapping: &IndexerContractMapping,
//     rpc_bloom: Bloom,
//     block_number: u64,
//     header_tx_info: &Header,
// ) {
//     let block_body_indices = provider.block_body_indices(block_number).unwrap();
//     if let Some(block_body_indices) = block_body_indices {
//         for tx_id in block_body_indices.first_tx_num
//             ..block_body_indices.first_tx_num + block_body_indices.tx_count
//         {
//             if let Some(transaction) = provider.transaction_by_id_no_hash(tx_id).unwrap() {
//                 if let Some(receipt) = provider.receipt(tx_id).unwrap() {
//                     let logs: Vec<Log> =
//                         if let Some(contract_address) = &mapping.filter_by_contract_addresses {
//                             receipt
//                                 .logs
//                                 .iter()
//                                 .filter(|log| {
//                                     contract_address
//                                         .iter()
//                                         .any(|address| address == &log.address)
//                                 })
//                                 .cloned()
//                                 .collect()
//                         } else {
//                             receipt.logs
//                         };
//
//                     if logs.is_empty() {
//                         continue;
//                     }
//
//                     process_transaction(
//                         csv_writers,
//                         db_writers,
//                         mapping,
//                         rpc_bloom,
//                         &logs,
//                         transaction,
//                         header_tx_info,
//                     )
//                     .await;
//                 }
//             }
//         }
//     }
// }
//
// fn header_provider_example<T: HeaderProvider>(provider: T, number: u64) -> eyre::Result<()> {
//     // Can query the header by number
//     let header = provider
//         .header_by_number(number)?
//         .ok_or(eyre::eyre!("header not found"))?;
//
//     // We can convert a header to a sealed header which contains the hash w/o needing to re-compute
//     // it every time.
//     let sealed = header.seal_slow();
//     let (header, seal) = sealed.into_parts();
//     let sealed_header = SealedHeader::new(header, seal);
//
//     // Can also query the header by hash!
//     let header_by_hash = provider
//         .header(&sealed_header.hash())?
//         .ok_or(eyre::eyre!("header by hash not found"))?;
//     println!("header: {:#?}", header_by_hash);
//     assert_eq!(sealed_header.header(), &header_by_hash);
//
//     // The header's total difficulty is stored in a separate table, so we have a separate call for
//     // it. This is not needed for post PoS transition chains.
//     let td = provider
//         .header_td_by_number(number)?
//         .ok_or(eyre::eyre!("header td not found"))?;
//     assert!(!td.is_zero());
//
//     // Can query headers by range as well, already sealed!
//     let headers = provider.sealed_headers_range(100..200)?;
//     assert_eq!(headers.len(), 100);
//
//     Ok(())
// }
//
// fn txs_provider_example<T: TransactionsProvider>(provider: T) -> eyre::Result<()> {
//     // Try the 5th tx
//     let txid = 5;
//
//     // Query a transaction by its primary ordered key in the db
//     let tx = provider
//         .transaction_by_id(txid)?
//         .ok_or(eyre::eyre!("transaction not found"))?;
//
//     // Can query the tx by hash
//     let tx_by_hash = provider
//         .transaction_by_hash(tx.hash)?
//         .ok_or(eyre::eyre!("txhash not found"))?;
//     assert_eq!(tx, tx_by_hash);
//
//     // Can query the tx by hash with info about the block it was included in
//     let (tx, meta) = provider
//         .transaction_by_hash_with_meta(tx.hash)?
//         .ok_or(eyre::eyre!("txhash not found"))?;
//     assert_eq!(tx.hash, meta.tx_hash);
//
//     // Can reverse lookup the key too
//     let id = provider
//         .transaction_id(tx.hash)?
//         .ok_or(eyre::eyre!("txhash not found"))?;
//     assert_eq!(id, txid);
//
//     // Can find the block of a transaction given its key
//     let _block = provider.transaction_block(txid)?;
//
//     // Can query the txs in the range [100, 200)
//     let _txs_by_tx_range = provider.transactions_by_tx_range(100..200)?;
//     // Can query the txs in the _block_ range [100, 200)]
//     let _txs_by_block_range = provider.transactions_by_block_range(100..200)?;
//
//     Ok(())
// }
//
// fn block_provider_example<T: BlockReader>(provider: T, number: u64) -> eyre::Result<()> {
//     // Can query a block by number
//     let block = provider
//         .block(number.into())?
//         .ok_or(eyre::eyre!("block num not found"))?;
//     assert_eq!(block.number, number);
//
//     // Can query a block with its senders, this is useful when you'd want to execute a block and do
//     // not want to manually recover the senders for each transaction (as each transaction is
//     // stored on disk with its v,r,s but not its `from` field.).
//     let block = provider
//         .block(number.into())?
//         .ok_or(eyre::eyre!("block num not found"))?;
//
//     // Can seal the block to cache the hash, like the Header above.
//     let sealed_block = block.clone().seal_slow();
//
//     // Can also query the block by hash directly
//     let block_by_hash = provider
//         .block_by_hash(sealed_block.hash())?
//         .ok_or(eyre::eyre!("block by hash not found"))?;
//     assert_eq!(block, block_by_hash);
//
//     // Or by relying in the internal conversion
//     let block_by_hash2 = provider
//         .block(sealed_block.hash().into())?
//         .ok_or(eyre::eyre!("block by hash not found"))?;
//     assert_eq!(block, block_by_hash2);
//
//     // Or you can also specify the datasource. For this provider this always return `None`, but
//     // the blockchain tree is also able to access pending state not available in the db yet.
//     let block_by_hash3 = provider
//         .find_block_by_hash(sealed_block.hash(), BlockSource::Any)?
//         .ok_or(eyre::eyre!("block hash not found"))?;
//     assert_eq!(block, block_by_hash3);
//
//     // Can query the block's ommers/uncles
//     let _ommers = provider.ommers(number.into())?;
//
//     // Can query the block's withdrawals (via the `WithdrawalsProvider`)
//     let _withdrawals =
//         provider.withdrawals_by_block(sealed_block.hash().into(), sealed_block.timestamp)?;
//
//     Ok(())
// }
//
// fn receipts_provider_example<T: ReceiptProvider + TransactionsProvider + HeaderProvider>(
//     provider: T,
// ) -> eyre::Result<()> {
//     let txid = 5;
//     let header_num = 100;
//
//     // Query a receipt by txid
//     let receipt = provider
//         .receipt(txid)?
//         .ok_or(eyre::eyre!("tx receipt not found"))?;
//
//     // Can query receipt by txhash too
//     let tx = provider.transaction_by_id(txid)?.unwrap();
//     let receipt_by_hash = provider
//         .receipt_by_hash(tx.hash)?
//         .ok_or(eyre::eyre!("tx receipt by hash not found"))?;
//     assert_eq!(receipt, receipt_by_hash);
//
//     // Can query all the receipts in a block
//     let _receipts = provider
//         .receipts_by_block(100.into())?
//         .ok_or(eyre::eyre!("no receipts found for block"))?;
//
//     // Can check if a address/topic filter is present in a header, if it is we query the block and
//     // receipts and do something with the data
//     // 1. get the bloom from the header
//     let header = provider.header_by_number(header_num)?.unwrap();
//     let bloom = header.logs_bloom;
//
//     // 2. Construct the address/topics filters
//     // For a hypothetical address, we'll want to filter down for a specific indexed topic (e.g.
//     // `from`).
//     let addr = Address::random();
//     let topic = B256::random();
//
//     // TODO: Make it clearer how to choose between event_signature(topic0) (event name) and the
//     // other 3 indexed topics. This API is a bit clunky and not obvious to use at the moment.
//     let filter = Filter::new().address(addr).event_signature(topic);
//     let filter_params = FilteredParams::new(Some(filter));
//     let address_filter = FilteredParams::address_filter(&addr.into());
//     let topics_filter = FilteredParams::topics_filter(&[topic.into()]);
//
//     // 3. If the address & topics filters match do something. We use the outer check against the
//     // bloom filter stored in the header to avoid having to query the receipts table when there
//     // is no instance of any event that matches the filter in the header.
//     if FilteredParams::matches_address(bloom, &address_filter)
//         && FilteredParams::matches_topics(bloom, &topics_filter)
//     {
//         let receipts = provider
//             .receipt(header_num)?
//             .ok_or(eyre::eyre!("receipt not found"))?;
//         for log in &receipts.logs {
//             if filter_params.filter_address(&log.address)
//                 && filter_params.filter_topics(log.topics())
//             {
//                 // Do something with the log e.g. decode it.
//                 println!("Matching log found! {log:?}")
//             }
//         }
//     }
//
//     Ok(())
// }
//
// fn state_provider_example<T: StateProvider + AccountReader>(provider: T) -> eyre::Result<()> {
//     let address = Address::random();
//     let storage_key = B256::random();
//
//     // Can get account / storage state with simple point queries
//     let _account = provider.basic_account(address)?;
//     let _code = provider.account_code(address)?;
//     let _storage = provider.storage(address, storage_key)?;
//     // TODO: unimplemented.
//     // let _proof = provider.proof(address, &[])?;
//
//     Ok(())
// }
