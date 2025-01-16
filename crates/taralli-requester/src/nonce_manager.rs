use std::marker::PhantomData;
use taralli_primitives::abi::permit2::Permit2::Permit2Instance;
use taralli_primitives::alloy::{
    network::Network,
    primitives::{Address, U256},
    providers::Provider,
    transports::Transport,
};
use taralli_primitives::utils::PERMIT2_ADDRESS;

use crate::error::{RequesterError, Result};

const U256_ONE: U256 = U256::from_limbs([1, 0, 0, 0]);
const U256_256: U256 = U256::from_limbs([256, 0, 0, 0]);

#[derive(Clone)]
pub struct Permit2NonceManager<T, P, N> {
    provider: P,
    signer_address: Address,
    nonce_cache: Option<(U256, U256)>,
    _phantom: PhantomData<(T, N)>,
}

impl<T, P, N> Permit2NonceManager<T, P, N>
where
    T: Transport + Clone + Send,
    P: Provider<T, N> + Clone + Send,
    N: Network + Clone + Send,
{
    pub fn new(provider: P, signer_address: Address) -> Self {
        Self {
            provider,
            nonce_cache: None,
            signer_address,
            _phantom: PhantomData,
        }
    }

    pub async fn get_nonce(&mut self) -> Result<U256> {
        if let Some(nonce_cache) = self.nonce_cache {
            if let Ok(nonce) = self.find_unused_nonce(nonce_cache.0, nonce_cache.1) {
                return Ok(nonce);
            }
        }

        let permit2 = Permit2Instance::new(PERMIT2_ADDRESS, self.provider.clone());
        let (word_pos, bitmap) = self.fetch_next_word(self.signer_address, permit2).await?;
        let nonce = self.find_unused_nonce(word_pos, bitmap)?;
        self.nonce_cache = Some((word_pos, bitmap));
        Ok(nonce)
    }

    async fn fetch_next_word(
        &self,
        signer: Address,
        permit2: Permit2Instance<T, P, N>,
    ) -> Result<(U256, U256)> {
        let mut word_pos = U256::ZERO;
        loop {
            let bitmap = permit2
                .nonceBitmap(signer, word_pos)
                .call()
                .await
                .map_err(|e| RequesterError::RpcRequestError(e.to_string()))?
                ._0;
            if bitmap != U256::MAX {
                return Ok((word_pos, bitmap));
            }
            word_pos += U256::from(1);
        }
    }

    fn find_unused_nonce(&self, word_pos: U256, bitmap: U256) -> Result<U256> {
        for i in 0..256 {
            if bitmap & (U256_ONE << i) == U256::ZERO {
                return Ok(word_pos * U256_256 + U256::from(i));
            }
        }
        Err(RequesterError::FindUnusedNonceError())
    }

    // /// If a signature with a given 'nonce' has been signed then broadcast, but needs
    // /// to be retracted, invalidate_nonce can retract a signature's validity given it executes
    // /// before a bid txs/consumption of the signature has been finalized in a block on-chain
    // pub async fn invalidate_nonce(&mut self, nonce: U256) -> Result<()> {
    //     let word_pos = nonce / *U256_256;
    //     let bit_pos = nonce % *U256_256;
    //     let mask = *U256_ONE << bit_pos;
    //
    //     // create instance
    //     let permit2 = Permit2Instance::new(*PERMIT2_ADDRESS, self.provider.clone());
    //
    //     // Update the bitmap in the contract
    //     let _ = permit2
    //         .invalidateUnorderedNonces(word_pos, mask)
    //         .send()
    //         .await?;
    //
    //     // Update our cache
    //     if let Some((cached_word_pos, mut cached_bitmap)) = self.nonce_cache {
    //         if cached_word_pos == word_pos {
    //             cached_bitmap |= mask;
    //         }
    //     }
    //
    //     Ok(())
    // }
}
