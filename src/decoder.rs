use alloy::{
    dyn_abi::SolType,
    primitives::{Address, B256},
    sol_types::sol_data::{self, IntBitCount, SupportedInt},
};
use mongodb::bson::{Bson, Decimal128};
use reth_primitives::Log;
use std::str::FromStr;

use crate::config::{ABIInput, ABIItem};

#[derive(Debug)]
pub struct DecodedTopic {
    pub name: String,
    pub value: Bson,
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

    // TODO: Regex should go here

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

fn decode_numeric_string(topic: &[u8], is_signed: bool) -> Bson {
    // Implementation that converts to String
    // This doesn't need to be generic since it handles all larger sizes
    if is_signed {
        sol_data::Int::<256>::abi_decode(topic, true)
            .unwrap()
            .to_string()
            .into()
    } else {
        sol_data::Uint::<256>::abi_decode(topic, true)
            .unwrap()
            .to_string()
            .into()
    }
}

fn decode_numeric_long<const BITS: usize>(topic: &[u8], is_signed: bool) -> Bson
where
    IntBitCount<BITS>: SupportedInt,
{
    if BITS > 64 {
        panic!("Bits size {} is not supported for Bson::Int64", BITS);
    }

    let value = if is_signed {
        sol_data::Int::<BITS>::abi_decode(topic, true)
            .unwrap()
            .to_string()
            .parse::<i64>()
            .unwrap()
    } else {
        sol_data::Uint::<BITS>::abi_decode(topic, true)
            .unwrap()
            .to_string()
            .parse::<i64>()
            .unwrap()
    };

    Bson::Int64(value)
}

fn decode_numeric_128<const BITS: usize>(topic: &[u8], is_signed: bool) -> Bson
where
    IntBitCount<BITS>: SupportedInt,
{
    if is_signed {
        sol_data::Int::<BITS>::abi_decode(topic, true)
            .unwrap()
            .to_string()
            .parse::<Decimal128>()
            .unwrap()
            .into()
    } else {
        let value = sol_data::Uint::<BITS>::abi_decode(topic, true)
            .unwrap()
            .to_string();

        match Decimal128::from_str(&value) {
            Ok(decimal) => decimal.into(),
            Err(_) => {
                println!("Error parsing decimal: {}. Bits: {}", value, BITS);
                value.into()
            }
        }
    }
}

fn decode_topic_value(topic: &[u8], abi: &ABIInput) -> Bson {
    match abi.type_.as_str() {
        "address" => sol_data::Address::abi_decode(topic, true)
            .unwrap()
            .to_checksum(None)
            .into(),
        "bool" => sol_data::Bool::abi_decode(topic, true).unwrap().into(),
        "bytes" => sol_data::Bytes::abi_decode(topic, true)
            .unwrap()
            .to_string()
            .into(),
        "string" => sol_data::String::abi_decode(topic, true).unwrap().into(),
        t if t.starts_with("uint") || t.starts_with("int") => {
            let (bits, is_signed) = if t.starts_with("uint") {
                (t[4..].parse::<usize>().unwrap(), false)
            } else {
                (t[3..].parse::<usize>().unwrap(), true)
            };

            if bits % 8 != 0 || bits == 0 || bits > 256 {
                panic!("Invalid bit size: {}", bits);
            }

            match bits {
                // Use Long for bits <= 64
                8 | 16 | 24 | 32 | 40 | 48 | 56 | 64 => decode_numeric_long::<64>(topic, is_signed),
                // Use Decimal128 for bits > 64 but <= 128
                72..=128 => decode_numeric_128::<128>(topic, is_signed),
                // Use String for bits > 128
                136..=256 => decode_numeric_string(topic, is_signed),
                _ => panic!("Unsupported bit size: {}", bits),
            }

            //match bits {
            //    // Use Decimal128 for bits <= 128
            //    8 => decode_numeric_128::<8>(topic, is_signed),
            //    16 => decode_numeric_128::<16>(topic, is_signed),
            //    24 => decode_numeric_128::<24>(topic, is_signed),
            //    32 => decode_numeric_128::<32>(topic, is_signed),
            //    40 => decode_numeric_128::<40>(topic, is_signed),
            //    48 => decode_numeric_128::<48>(topic, is_signed),
            //    56 => decode_numeric_128::<56>(topic, is_signed),
            //    64 => decode_numeric_128::<64>(topic, is_signed),
            //    72 => decode_numeric_128::<72>(topic, is_signed),
            //    80 => decode_numeric_128::<80>(topic, is_signed),
            //    88 => decode_numeric_128::<88>(topic, is_signed),
            //    96 => decode_numeric_128::<96>(topic, is_signed),
            //    104 => decode_numeric_128::<104>(topic, is_signed),
            //    112 => decode_numeric_128::<112>(topic, is_signed),
            //    120 => decode_numeric_128::<120>(topic, is_signed),
            //    128 => decode_numeric_128::<128>(topic, is_signed),
            //    // Use String for bits > 128
            //    // TODO: Perfect scenario would be if there was a way to make these numbers
            //    // as Decimal128 as well
            //    136..=256 => decode_numeric_string(topic, is_signed),
            //    _ => panic!("Unsupported bit size: {}", bits),
            //}
        }
        _ => panic!("Unknown type: {}", abi.type_),
    }
}
