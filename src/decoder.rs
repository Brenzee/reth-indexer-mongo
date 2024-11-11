use alloy::{
    dyn_abi::SolType,
    primitives::{Address, B256},
    sol_types::sol_data,
};
use reth_primitives::Log;

use crate::config::{ABIInput, ABIItem};

#[derive(Debug)]
pub struct DecodedTopic {
    pub name: String,
    pub value: String,
}

/// Represents a decoded structure with a name and a corresponding value.
#[derive(Debug)]
pub struct DecodedLog {
    pub address: Address,
    pub topics: Vec<DecodedTopic>,
}

pub fn decode_logs(topic_id: B256, logs: &[Log], abi_item: &ABIItem) -> Vec<DecodedLog> {
    logs.iter()
        .filter_map(|log| {
            let topic = log.topics();
            if topic.len() > 0 && topic[0] == topic_id {
                decode_log(log, abi_item).ok()
            } else {
                None
            }
        })
        .collect()
}

fn decode_log(log: &Log, abi_item: &ABIItem) -> Result<DecodedLog, ()> {
    let decoded_indexed_topics = decode_log_topics(log, abi_item)?;
    let decoded_non_indexed_data = decode_log_data(log, abi_item)?;

    let mut topics: Vec<DecodedTopic> = decoded_indexed_topics
        .into_iter()
        .chain(decoded_non_indexed_data)
        .collect();

    topics.sort_by_key(|item| {
        abi_item
            .inputs
            .iter()
            .position(|input| input.name == item.name)
    });

    Ok(DecodedLog {
        address: log.address,
        topics,
    })
}

fn decode_log_topics(log: &Log, abi: &ABIItem) -> Result<Vec<DecodedTopic>, ()> {
    let indexed_inputs: Vec<&ABIInput> = abi
        .inputs
        .iter()
        .filter(|input| input.indexed)
        .collect::<Vec<_>>();

    if indexed_inputs.len() != log.topics().len() - 1 {
        // -1 because the first topic is the event signature
        return Err(());
    }

    let mut results: Vec<DecodedTopic> = Vec::<DecodedTopic>::new();

    for (i, topic) in log.topics().iter().enumerate().skip(1) {
        let abi_input = indexed_inputs[i - 1];
        results.push(decode_topic_log(topic.as_slice(), abi_input)?);
    }

    Ok(results)
}

fn decode_topic_log(topic: &[u8], abi_input: &ABIInput) -> Result<DecodedTopic, ()> {
    let value = decode_topic_value(topic, abi_input);

    // TODO: Do we need regex check here?

    Ok(DecodedTopic {
        name: abi_input.name.clone(),
        value,
    })
}

fn decode_log_data(log: &Log, abi: &ABIItem) -> Result<Vec<DecodedTopic>, ()> {
    let non_indexed_inputs: Vec<&ABIInput> = abi
        .inputs
        .iter()
        .filter(|input| !input.indexed)
        .collect::<Vec<_>>();

    // let topics = log.data.as_slice().chunks_exact(32);
    let topics = log.data.data.chunks_exact(32);
    if non_indexed_inputs.len() != topics.len() {
        return Err(());
    }

    let mut results = Vec::<DecodedTopic>::new();

    for (i, topic) in topics.enumerate() {
        let abi_input = non_indexed_inputs[i];
        results.push(decode_topic_log(topic, abi_input)?);
    }

    Ok(results)
}

fn decode_topic_value(topic: &[u8], abi: &ABIInput) -> String {
    // TODO probably a nicer way to do this with sol_data directly PR welcome
    match abi.type_.as_str() {
        "address" => sol_data::Address::abi_decode(topic, true)
            .unwrap()
            .to_checksum(None),
        "bool" => sol_data::Bool::abi_decode(topic, true).unwrap().to_string(),
        "bytes" => sol_data::Bytes::abi_decode(topic, true)
            .unwrap()
            .to_string(),
        "string" => sol_data::String::abi_decode(topic, true).unwrap(),
        "uint8" => sol_data::Uint::<8>::abi_decode(topic, true)
            .unwrap()
            .to_string(),
        "uint16" => sol_data::Uint::<16>::abi_decode(topic, true)
            .unwrap()
            .to_string(),
        "uint24" => sol_data::Uint::<24>::abi_decode(topic, true)
            .unwrap()
            .to_string(),
        "uint32" => sol_data::Uint::<32>::abi_decode(topic, true)
            .unwrap()
            .to_string(),
        "uint64" => sol_data::Uint::<64>::abi_decode(topic, true)
            .unwrap()
            .to_string(),
        "uint112" => sol_data::Uint::<112>::abi_decode(topic, true)
            .unwrap()
            .to_string(),
        "uint128" => sol_data::Uint::<128>::abi_decode(topic, true)
            .unwrap()
            .to_string(),
        "uint256" => sol_data::Uint::<256>::abi_decode(topic, true)
            .unwrap()
            .to_string(),
        "int8" => sol_data::Int::<8>::abi_decode(topic, true)
            .unwrap()
            .to_string(),
        "int16" => sol_data::Int::<16>::abi_decode(topic, true)
            .unwrap()
            .to_string(),
        "int24" => sol_data::Int::<24>::abi_decode(topic, true)
            .unwrap()
            .to_string(),
        "int32" => sol_data::Int::<32>::abi_decode(topic, true)
            .unwrap()
            .to_string(),
        "int64" => sol_data::Int::<64>::abi_decode(topic, true)
            .unwrap()
            .to_string(),
        "int128" => sol_data::Int::<128>::abi_decode(topic, true)
            .unwrap()
            .to_string(),
        "int256" => sol_data::Int::<256>::abi_decode(topic, true)
            .unwrap()
            .to_string(),
        _ => panic!("Unknown type: {}", abi.type_),
    }
}
