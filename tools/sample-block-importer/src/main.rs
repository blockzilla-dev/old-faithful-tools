use anyhow::{Context, Result, ensure};
use blockzilla_archive_v2::*;
use blockzilla_compact::*;
use blockzilla_primitives::{CompactPubkey, wincode_leb128_config};
use blockzilla_registry::KeyIndex;
use blockzilla_sample_block_bench::{hex_bytes, sha256, summarize};
use of_car_reader::{
    CarBlockGroup, CarBlockReader,
    confirmed_block::Rewards,
    metadata_decoder::{ZstdReusableDecoder, decode_rewards_from_frame},
    versioned_transaction::{
        CompiledInstruction, MessageHeader, VersionedMessage, VersionedTransaction,
    },
};
use sha2::{Digest, Sha256};
use solana_pubkey::Pubkey;
use std::{
    io::{BufReader, Cursor},
    path::PathBuf,
    str::FromStr,
};

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let source = PathBuf::from(args.next().context("CAR input")?);
    let output = PathBuf::from(args.next().context("fixture output directory")?);
    ensure!(args.next().is_none(), "expected CAR and output directory");
    let source_bytes = std::fs::read(&source)?;
    let mut reader =
        CarBlockReader::with_capacity(BufReader::new(Cursor::new(&source_bytes)), 8 << 20);
    reader.skip_header()?;
    let mut group = CarBlockGroup::new();
    ensure!(
        reader.read_until_block_into(&mut group)?,
        "CAR has no complete block"
    );
    let slot = group.slot.context("missing slot")?;
    let mut frames = group.transaction_frames();
    let mut sources = Vec::new();
    let mut source_hash = Sha256::new();
    source_hash.update(b"BZ-SIGNED-TRANSACTIONS-V1");
    while let Some(frame) = frames.next_frame()? {
        let index = sources.len() as u32;
        ensure!(
            frame.index.is_none_or(|value| value == u64::from(index)),
            "source row order mismatch"
        );
        source_hash.update(index.to_le_bytes());
        source_hash.update((frame.transaction_data.len() as u64).to_le_bytes());
        source_hash.update(frame.transaction_data);
        sources.push((
            frame.transaction_data.len(),
            frame.metadata_data.map(<[u8]>::len),
        ));
    }
    let source_signed_sha256 = hex_bytes(&source_hash.finalize());
    let keys = KeyIndex::build(Vec::new());
    let mut txs = Vec::new();
    let mut signature_counts = Vec::new();
    let mut source_log_lines = 0usize;
    let mut source_metadata_rows = 0usize;
    let mut source_failed_transactions = 0usize;
    let mut source_inner_instructions = 0usize;
    let mut iter = group.transactions();
    while let Some((transaction, metadata)) = iter.next_tx()? {
        let ordinal = txs.len();
        signature_counts.push(transaction.signatures.len() as u32);
        source_log_lines += metadata.map_or(0, |meta| meta.log_messages.len());
        source_metadata_rows += usize::from(metadata.is_some());
        source_failed_transactions += usize::from(metadata.is_some_and(|meta| meta.err.is_some()));
        source_inner_instructions += metadata.map_or(0, |meta| {
            meta.inner_instructions
                .iter()
                .map(|group| group.instructions.len())
                .sum::<usize>()
        });
        txs.push(WincodeArchiveV2Transaction {
            tx_index: ordinal as u32,
            tx: WincodeArchiveV2Payload::Decoded {
                source_len: sources[ordinal].0 as u64,
                value: transaction_owned(transaction),
            },
            metadata: metadata
                .map(|meta| -> Result<_> {
                    Ok(WincodeArchiveV2Payload::Decoded {
                        source_len: sources[ordinal].1.context("metadata size")? as u64,
                        value: compact_meta_from_proto(meta, &keys)?,
                    })
                })
                .transpose()?,
        });
    }
    drop(iter);
    ensure!(txs.len() == sources.len(), "transaction rows lost");
    let mut cursor = 0usize;
    let mut poh_entries = Vec::new();
    for ((num_hashes, hash), count) in group
        .poh_num_hashes
        .iter()
        .zip(&group.poh_hashes)
        .zip(&group.entry_tx_counts)
    {
        let end = cursor + *count as usize;
        ensure!(
            end <= signature_counts.len(),
            "PoH transaction count overflow"
        );
        poh_entries.push(CompactPohEntry {
            num_hashes: *num_hashes,
            hash: *hash,
            tx_count: *count,
            signature_count: signature_counts[cursor..end].iter().sum(),
        });
        cursor = end;
    }
    ensure!(cursor == txs.len(), "PoH does not cover all transactions");
    let rewards = if let Some(frame) = &group.rewards {
        let mut value = Rewards::default();
        decode_rewards_from_frame(&frame.data, &mut value, &mut ZstdReusableDecoder::new())?;
        Some(WincodeArchiveV2Rewards {
            source_len: frame.data.len() as u64,
            num_partitions: value
                .num_partitions
                .as_ref()
                .map(|value| value.num_partitions),
            decoded: Some(
                value
                    .rewards
                    .iter()
                    .map(|reward| {
                        Ok(CompactReward {
                            pubkey: CompactPubkey::raw(
                                Pubkey::from_str(&reward.pubkey)?.to_bytes(),
                            ),
                            lamports: reward.lamports,
                            post_balance: reward.post_balance,
                            reward_type: reward.reward_type,
                            commission: if reward.commission.is_empty() {
                                None
                            } else {
                                Some(reward.commission.parse()?)
                            },
                        })
                    })
                    .collect::<Result<Vec<_>>>()?,
            ),
            raw_fallback: None,
            decode_error: None,
        })
    } else {
        None
    };
    let current_blockhash = group.has_blockhash.then_some(group.blockhash);
    let block = WincodeArchiveV2Block {
        header: WincodeArchiveV2BlockHeader {
            compact: CompactBlockHeader {
                slot,
                parent_slot: group.parent_slot.context("missing parent slot")?,
                blockhash: u32::from(group.has_blockhash),
                previous_blockhash: 0,
                block_time: group.block_time,
                block_height: group.block_height,
                shredding: group
                    .shredding
                    .iter()
                    .map(|value| CompactShredding {
                        entry_end_idx: value.entry_end_idx,
                        shred_end_idx: value.shred_end_idx,
                    })
                    .collect(),
                poh_entries,
                rewards: None,
            },
            rewards,
        },
        txs,
    };
    let expected = summarize(&block)?;
    ensure!(
        expected.signed_transactions_sha256 == source_signed_sha256,
        "signed source bytes changed"
    );
    ensure!(
        expected.metadata_rows == source_metadata_rows,
        "metadata row count changed"
    );
    ensure!(
        expected.failed_transactions == source_failed_transactions,
        "failed transaction count changed"
    );
    ensure!(
        expected.inner_instructions == source_inner_instructions,
        "inner instruction count changed"
    );
    ensure!(
        expected.log_events == source_log_lines,
        "log event count changed"
    );
    ensure!(
        !reader.read_until_block_into(&mut CarBlockGroup::new())?,
        "fixture contains several blocks"
    );
    let decoded = wincode::config::serialize(&block, wincode_leb128_config())?;
    let compressed = zstd::bulk::compress(&decoded, 3)?;
    std::fs::create_dir_all(&output)?;
    std::fs::write(output.join("block.compact-v2.zst"), &compressed)?;
    let manifest = serde_json::json!({
        "schema": "blockzilla-sample-block-v1",
        "format": "WincodeArchiveV2Block; LEB128 integers; zstd level 3",
        "source": {
            "kind": "existing Old Faithful CAR fixture",
            "file": source.file_name().unwrap().to_string_lossy(),
            "sha256": sha256(&source_bytes),
            "bytes": source_bytes.len(),
            "original_log_lines": source_log_lines,
            "metadata_rows": source_metadata_rows,
            "failed_transactions": source_failed_transactions,
            "inner_instructions": source_inner_instructions,
        },
        "blockhash_registry": { "1": current_blockhash.map(|value| sha256_hex(&value)) },
        "previous_blockhash": null,
        "compressed_sha256": sha256(&compressed),
        "decoded_sha256": sha256(&decoded),
        "compressed_bytes": compressed.len(),
        "decoded_bytes": decoded.len(),
        "expected": expected,
        "scope": "One complete fixture block with inline public keys and recent hashes. Not a full epoch or chain-continuity proof. The source fixture does not supply the previous blockhash. Existing compact log codec rules apply.",
    });
    std::fs::write(
        output.join("manifest.json"),
        format!("{}\n", serde_json::to_string_pretty(&manifest)?),
    )?;
    println!("{}", serde_json::to_string_pretty(&manifest)?);
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|value| format!("{value:02x}")).collect()
}

fn header(value: MessageHeader) -> CompactMessageHeader {
    CompactMessageHeader {
        num_required_signatures: value.num_required_signatures,
        num_readonly_signed_accounts: value.num_readonly_signed_accounts,
        num_readonly_unsigned_accounts: value.num_readonly_unsigned_accounts,
    }
}

fn instruction(value: &CompiledInstruction) -> OwnedCompactInstruction {
    OwnedCompactInstruction {
        program_id_index: value.program_id_index,
        accounts: value.accounts.clone(),
        data: value.data.clone(),
    }
}

fn transaction_owned(value: &VersionedTransaction<'_>) -> OwnedCompactTransaction {
    let message = match &value.message {
        VersionedMessage::Legacy(message) => {
            OwnedCompactMessage::Legacy(OwnedCompactLegacyMessage {
                header: header(message.header),
                account_keys: message
                    .account_keys
                    .iter()
                    .map(|value| CompactPubkey::raw(**value))
                    .collect(),
                recent_blockhash: OwnedCompactRecentBlockhash::Nonce(*message.recent_blockhash),
                instructions: message.instructions.iter().map(instruction).collect(),
            })
        }
        VersionedMessage::V0(message) => OwnedCompactMessage::V0(OwnedCompactV0Message {
            header: header(message.header),
            account_keys: message
                .account_keys
                .iter()
                .map(|value| CompactPubkey::raw(**value))
                .collect(),
            recent_blockhash: OwnedCompactRecentBlockhash::Nonce(*message.recent_blockhash),
            instructions: message.instructions.iter().map(instruction).collect(),
            address_table_lookups: message
                .address_table_lookups
                .iter()
                .map(|value| OwnedCompactAddressTableLookup {
                    account_key: CompactPubkey::raw(*value.account_key),
                    writable_indexes: value.writable_indexes.clone(),
                    readonly_indexes: value.readonly_indexes.clone(),
                })
                .collect(),
        }),
        VersionedMessage::V1(message) => OwnedCompactMessage::V1(OwnedCompactV1Message {
            header: header(message.header),
            account_keys: message
                .account_keys
                .iter()
                .map(|value| CompactPubkey::raw(**value))
                .collect(),
            recent_blockhash: OwnedCompactRecentBlockhash::Nonce(*message.recent_blockhash),
            instructions: message.instructions.iter().map(instruction).collect(),
            config: CompactTransactionConfig {
                priority_fee: message.config.priority_fee,
                compute_unit_limit: message.config.compute_unit_limit,
                loaded_accounts_data_size_limit: message.config.loaded_accounts_data_size_limit,
                heap_size: message.config.heap_size,
            },
        }),
    };
    OwnedCompactTransaction {
        signatures: value
            .signatures
            .iter()
            .map(|value| value.to_vec())
            .collect(),
        message,
    }
}
