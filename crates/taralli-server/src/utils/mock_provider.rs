use alloy::{
    network::Ethereum,
    providers::*,
    rpc::types::{Block, BlockId, BlockTransactionsKind},
    transports::{BoxTransport, TransportResult},
};

pub struct MockProvider {
    mock_block: Block,
}

impl MockProvider {
    pub fn new(mock_block: Block) -> Self {
        Self { mock_block }
    }
}

// Constant mock block
#[async_trait::async_trait]
impl Provider<BoxTransport, Ethereum> for MockProvider {
    fn root(&self) -> &RootProvider<BoxTransport, Ethereum> {
        unimplemented!("Not needed for this example")
    }

    async fn get_block(
        &self,
        _block: BlockId,
        _kind: BlockTransactionsKind,
    ) -> TransportResult<Option<Block>> {
        Ok(Some(self.mock_block.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::rpc::types::{serde_helpers::quantity::deserialize, BlockNumberOrTag};

    const MOCK_BLOCK: &str = r#"{
    "baseFeePerGas":"0x886b221ad",
    "blobGasUsed":"0x0",
    "difficulty":"0x0",
    "excessBlobGas":"0x0",
    "extraData":"0x6265617665726275696c642e6f7267",
    "gasLimit":"0x1c9c380",
    "gasUsed":"0xb0033c",
    "hash":"0x85cdcbe36217fd57bf2c33731d8460657a7ce512401f49c9f6392c82a7ccf7ac",
    "logsBloom":"0xc36919406572730518285284f2293101104140c0d42c4a786c892467868a8806f40159d29988002870403902413a1d04321320308da2e845438429e0012a00b419d8ccc8584a1c28f82a415d04eab8a5ae75c00d07761acf233414c08b6d9b571c06156086c70ea5186e9b989b0c2d55c0213c936805cd2ab331589c90194d070c00867549b1e1be14cb24500b0386cd901197c1ef5a00da453234fa48f3003dcaa894e3111c22b80e17f7d4388385a10720cda1140c0400f9e084ca34fc4870fb16b472340a2a6a63115a82522f506c06c2675080508834828c63defd06bc2331b4aa708906a06a560457b114248041e40179ebc05c6846c1e922125982f427",
    "miner":"0x95222290dd7278aa3ddd389cc1e1d165cc4bafe5",
    "mixHash":"0x4c068e902990f21f92a2456fc75c59bec8be03b7f13682b6ebd27da56269beb5",
    "nonce":"0x0000000000000000",
    "number":"0x128c6df",
    "parentBeaconBlockRoot":"0x2843cb9f7d001bd58816a915e685ed96a555c9aeec1217736bd83a96ebd409cc",
    "parentHash":"0x90926e0298d418181bd20c23b332451e35fd7d696b5dcdc5a3a0a6b715f4c717",
    "receiptsRoot":"0xd43aa19ecb03571d1b86d89d9bb980139d32f2f2ba59646cd5c1de9e80c68c90",
    "sha3Uncles":"0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
    "size":"0xdcc3",
    "stateRoot":"0x707875120a7103621fb4131df59904cda39de948dfda9084a1e3da44594d5404",
    "timestamp":"0x65f5f4c3",
    "transactionsRoot":"0x889a1c26dc42ba829dab552b779620feac231cde8a6c79af022bdc605c23a780",
    "withdrawals":[
       {
          "index":"0x24d80e6",
          "validatorIndex":"0x8b2b6",
          "address":"0x7cd1122e8e118b12ece8d25480dfeef230da17ff",
          "amount":"0x1161f10"
       }
    ],
    "withdrawalsRoot":"0x360c33f20eeed5efbc7d08be46e58f8440af5db503e40908ef3d1eb314856ef7"
 }"#;

    #[tokio::test]
    async fn test_mock_provider_block_time() -> Result<(), Box<dyn std::error::Error>> {
        let block = serde_json::from_str::<Block>(MOCK_BLOCK)?;
        let mock_provider = MockProvider::new(block);
        let retrieved_block = mock_provider
            .get_block(
                BlockId::Number(BlockNumberOrTag::Latest),
                BlockTransactionsKind::Hashes,
            )
            .await?
            .expect("Block should be returned");

        let expected_ts: u64 = deserialize(serde_json::Value::String("0x65f5f4c3".to_string()))
            .expect("Failed to deserialize timestamp");

        assert_eq!(
            retrieved_block.header.timestamp, expected_ts,
            "Block timestamp should match the expected value"
        );

        Ok(())
    }
}
