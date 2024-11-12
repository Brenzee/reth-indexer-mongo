use alloy::{
    dyn_abi::SolType,
    primitives::{Address, B256},
    sol_types::sol_data::{self, IntBitCount, SupportedInt},
};
use mongodb::bson::{Bson, Decimal128};
use reth_primitives::Log;

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
        sol_data::Uint::<BITS>::abi_decode(topic, true)
            .unwrap()
            .to_string()
            .parse::<Decimal128>()
            .unwrap()
            .into()
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
                // Use Decimal128 for bits <= 128
                8 => decode_numeric_128::<8>(topic, is_signed),
                16 => decode_numeric_128::<16>(topic, is_signed),
                24 => decode_numeric_128::<24>(topic, is_signed),
                32 => decode_numeric_128::<32>(topic, is_signed),
                40 => decode_numeric_128::<40>(topic, is_signed),
                48 => decode_numeric_128::<48>(topic, is_signed),
                56 => decode_numeric_128::<56>(topic, is_signed),
                64 => decode_numeric_128::<64>(topic, is_signed),
                72 => decode_numeric_128::<72>(topic, is_signed),
                80 => decode_numeric_128::<80>(topic, is_signed),
                88 => decode_numeric_128::<88>(topic, is_signed),
                96 => decode_numeric_128::<96>(topic, is_signed),
                104 => decode_numeric_128::<104>(topic, is_signed),
                112 => decode_numeric_128::<112>(topic, is_signed),
                120 => decode_numeric_128::<120>(topic, is_signed),
                128 => decode_numeric_128::<128>(topic, is_signed),
                // Use String for bits > 128
                136..=256 => decode_numeric_string(topic, is_signed),
                // 136 => decode_numeric::<136>(topic, is_signed),
                // 144 => decode_numeric::<144>(topic, is_signed),
                // 152 => decode_numeric::<152>(topic, is_signed),
                // 160 => decode_numeric::<160>(topic, is_signed),
                // 168 => decode_numeric::<168>(topic, is_signed),
                // 176 => decode_numeric::<176>(topic, is_signed),
                // 184 => decode_numeric::<184>(topic, is_signed),
                // 192 => decode_numeric::<192>(topic, is_signed),
                // 200 => decode_numeric::<200>(topic, is_signed),
                // 208 => decode_numeric::<208>(topic, is_signed),
                // 216 => decode_numeric::<216>(topic, is_signed),
                // 224 => decode_numeric::<224>(topic, is_signed),
                // 232 => decode_numeric::<232>(topic, is_signed),
                // 240 => decode_numeric::<240>(topic, is_signed),
                // 248 => decode_numeric::<248>(topic, is_signed),
                // 256 => decode_numeric::<256>(topic, is_signed),
                _ => panic!("Unsupported bit size: {}", bits),
            }
        }
        // "uint8" => sol_data::Uint::<8>::abi_decode(topic, true)
        //     .unwrap()
        //     .to_string(),
        // "uint16" => sol_data::Uint::<16>::abi_decode(topic, true)
        //     .unwrap()
        //     .to_string(),
        // "uint24" => sol_data::Uint::<24>::abi_decode(topic, true)
        //     .unwrap()
        //     .to_string(),
        // "uint32" => sol_data::Uint::<32>::abi_decode(topic, true)
        //     .unwrap()
        //     .to_string(),
        // "uint64" => sol_data::Uint::<64>::abi_decode(topic, true)
        //     .unwrap()
        //     .to_string(),
        // "uint112" => sol_data::Uint::<112>::abi_decode(topic, true)
        //     .unwrap()
        //     .to_string(),
        // "uint128" => sol_data::Uint::<128>::abi_decode(topic, true)
        //     .unwrap()
        //     .to_string(),
        // "uint256" => sol_data::Uint::<256>::abi_decode(topic, true)
        //     .unwrap()
        //     .to_string(),
        // "int8" => sol_data::Int::<8>::abi_decode(topic, true)
        //     .unwrap()
        //     .to_string(),
        // "int16" => sol_data::Int::<16>::abi_decode(topic, true)
        //     .unwrap()
        //     .to_string(),
        // "int24" => sol_data::Int::<24>::abi_decode(topic, true)
        //     .unwrap()
        //     .to_string(),
        // "int32" => sol_data::Int::<32>::abi_decode(topic, true)
        //     .unwrap()
        //     .to_string(),
        // "int64" => sol_data::Int::<64>::abi_decode(topic, true)
        //     .unwrap()
        //     .to_string(),
        // "int128" => sol_data::Int::<128>::abi_decode(topic, true)
        //     .unwrap()
        //     .to_string(),
        // "int256" => sol_data::Int::<256>::abi_decode(topic, true)
        //     .unwrap()
        //     .to_string(),
        _ => panic!("Unknown type: {}", abi.type_),
    }
}
