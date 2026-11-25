pub use blockzilla_solana_wire::slot_meta::*;

#[cfg(test)]
mod tests {
    use minicbor::Encoder;

    fn tuple(n: u64, tail: impl FnOnce(&mut Encoder<Vec<u8>>)) -> Vec<u8> {
        let mut e = Encoder::new(Vec::new());
        e.array(n).unwrap().u64(4).unwrap().i64(-2).unwrap();
        tail(&mut e);
        e.into_writer()
    }
    #[test]
    fn block_decoders_and_lossless_reconstruction_retain_extensions() {
        use crate::{
            CarBlockGroup,
            node::{BlockNode, decode_node},
            reconstruct::{Cid36, NodeLocation, RawNode, decode_raw_node},
        };
        let raw = tuple(6, |e| {
            e.null()
                .unwrap()
                .array(2)
                .unwrap()
                .bytes(&[1, 0, 1, 0, 0])
                .unwrap()
                .bytes(&[255, 7])
                .unwrap()
                .bytes(&[9; 32])
                .unwrap()
                .map(1)
                .unwrap()
                .str("unknown")
                .unwrap()
                .bool(true)
                .unwrap();
        });
        let mut e = Encoder::new(Vec::new());
        // Keep the unrelated optional rewards field absent, as emitted by the
        // existing raw Block encoder. This fixture tests exact SlotMeta bytes.
        e.array(5)
            .unwrap()
            .u8(2)
            .unwrap()
            .u64(12)
            .unwrap()
            .array(0)
            .unwrap()
            .array(0)
            .unwrap();
        e.writer_mut().extend_from_slice(&raw);
        let payload = e.into_writer();
        let block: BlockNode<'_> = minicbor::decode(&payload).unwrap();
        assert_eq!(block.meta.raw_cbor(), raw);
        assert!(matches!(
            decode_node(&payload).unwrap(),
            crate::node::Node::Block(_)
        ));
        let node = decode_raw_node(
            NodeLocation {
                entry_index: 0,
                car_offset: 0,
            },
            Cid36::compute(&payload),
            &payload,
        )
        .unwrap();
        assert_eq!(node.encode_payload(), payload);
        let RawNode::Block(block) = node else {
            panic!("block expected")
        };
        assert_eq!(block.meta_cbor, raw);
        let mut group = CarBlockGroup::new();
        assert!(
            group
                .read_entry_payload_slice_into(&payload, payload.len())
                .unwrap()
        );
        assert_eq!(group.slot_meta.as_ref().unwrap().raw_cbor(), raw);
        group.clear();
        assert!(group.slot_meta.is_none());
    }
}
