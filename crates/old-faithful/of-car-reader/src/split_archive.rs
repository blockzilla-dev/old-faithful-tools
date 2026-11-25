//! Legacy CAR reconstruction records for split historical/runtime archives.

use crate::{
    node::{Shredding, SlotMeta},
    reconstruct::{
        Cid36, NodeLocation, RawBlockNode, RawCidRef, RawDataFrame, RawEntryNode, RawEpochNode,
        RawNode, RawRewardsNode, RawSubsetNode, RawTransactionNode, StandaloneDataFrame,
    },
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LosslessV2Header {
    /// Exact CAR header bytes, including the length varint prefix.
    pub encoded_car_header: Vec<u8>,
}

/// Historical and replay-verifiable node content for a CAR-equivalent archive.
///
/// Together with `RuntimeArchiveRecord` and `ReconstructionArchiveRecord`, this
/// preserves every CAR node needed to validate CIDs and emit the original CAR
/// stream again. This is the path to port to wincode for the full archive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HistoricalArchiveRecord {
    Transaction(HistoricalTransactionRecord),
    Entry(RawEntryNode),
    Block(HistoricalBlockRecord),
    DataFrame(StandaloneDataFrame),
    Subset(RawSubsetNode),
    Epoch(RawEpochNode),
}

/// Runtime and metadata-only node content for a CAR-equivalent archive.
///
/// Transaction metadata and reward dataframes live here so the historical stream
/// can stay focused on replay-critical content without losing any CAR bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuntimeArchiveRecord {
    Transaction(RuntimeTransactionRecord),
    Rewards(RawRewardsNode),
    Block(RuntimeBlockRecord),
}

/// Reconstruction order and original CAR offsets.
///
/// This stream is what makes the split historical/runtime files lossless: it
/// records each node kind, original location, and CID in CAR order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReconstructionArchiveRecord {
    Header(LosslessV2Header),
    Node(ReconstructionNodeRecord),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReconstructionNodeKind {
    Transaction,
    Entry,
    Rewards,
    Block,
    Subset,
    Epoch,
    DataFrame,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconstructionNodeRecord {
    pub location: NodeLocation,
    pub cid: Cid36,
    pub kind: ReconstructionNodeKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalTransactionRecord {
    pub location: NodeLocation,
    pub cid: Cid36,
    pub slot: u64,
    pub index: Option<u64>,
    /// Raw transaction dataframe, including continuation references.
    pub data: RawDataFrame,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeTransactionRecord {
    pub location: NodeLocation,
    pub cid: Cid36,
    /// Raw transaction status metadata dataframe, including continuation refs.
    pub metadata: RawDataFrame,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalBlockRecord {
    pub location: NodeLocation,
    pub cid: Cid36,
    pub slot: u64,
    pub shredding: Vec<Shredding>,
    /// Exact CBOR section from the CAR block payload.
    pub shredding_cbor: Vec<u8>,
    pub entries: Vec<RawCidRef>,
    /// Exact CBOR section from the CAR block payload.
    pub entries_cbor: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeBlockRecord {
    pub location: NodeLocation,
    pub cid: Cid36,
    pub slot: u64,
    pub meta: SlotMeta,
    /// Exact CBOR section from the CAR block payload.
    pub meta_cbor: Vec<u8>,
    /// Optional reward-node CID reference from the block payload.
    pub rewards: Option<RawCidRef>,
}

impl ReconstructionNodeRecord {
    pub fn from_raw(node: &RawNode) -> Self {
        let kind = match node {
            RawNode::Transaction(_) => ReconstructionNodeKind::Transaction,
            RawNode::Entry(_) => ReconstructionNodeKind::Entry,
            RawNode::Rewards(_) => ReconstructionNodeKind::Rewards,
            RawNode::Block(_) => ReconstructionNodeKind::Block,
            RawNode::Subset(_) => ReconstructionNodeKind::Subset,
            RawNode::Epoch(_) => ReconstructionNodeKind::Epoch,
            RawNode::DataFrame(_) => ReconstructionNodeKind::DataFrame,
        };
        Self {
            location: node.location(),
            cid: node.cid(),
            kind,
        }
    }
}

impl HistoricalTransactionRecord {
    pub fn from_raw(node: &RawTransactionNode) -> Self {
        Self {
            location: node.location,
            cid: node.cid,
            slot: node.slot,
            index: node.index,
            data: node.data.clone(),
        }
    }

    pub fn with_runtime(self, runtime: RuntimeTransactionRecord) -> RawTransactionNode {
        RawTransactionNode {
            location: self.location,
            cid: self.cid,
            slot: self.slot,
            index: self.index,
            data: self.data,
            metadata: runtime.metadata,
        }
    }
}

impl RuntimeTransactionRecord {
    pub fn from_raw(node: &RawTransactionNode) -> Self {
        Self {
            location: node.location,
            cid: node.cid,
            metadata: node.metadata.clone(),
        }
    }
}

impl HistoricalBlockRecord {
    pub fn from_raw(node: &RawBlockNode) -> Self {
        Self {
            location: node.location,
            cid: node.cid,
            slot: node.slot,
            shredding: node.shredding.clone(),
            shredding_cbor: node.shredding_cbor.clone(),
            entries: node.entries.clone(),
            entries_cbor: node.entries_cbor.clone(),
        }
    }

    pub fn with_runtime(self, runtime: RuntimeBlockRecord) -> RawBlockNode {
        RawBlockNode {
            location: self.location,
            cid: self.cid,
            slot: self.slot,
            shredding: self.shredding,
            shredding_cbor: self.shredding_cbor,
            entries: self.entries,
            entries_cbor: self.entries_cbor,
            meta: runtime.meta,
            meta_cbor: runtime.meta_cbor,
            rewards: runtime.rewards,
        }
    }
}

impl RuntimeBlockRecord {
    pub fn from_raw(node: &RawBlockNode) -> Self {
        Self {
            location: node.location,
            cid: node.cid,
            slot: node.slot,
            meta: node.meta.clone(),
            meta_cbor: node.meta_cbor.clone(),
            rewards: node.rewards.clone(),
        }
    }
}
