use alloy::primitives::{address, b256, keccak256, Address, B256};
use alloy::sol_types::SolValue;
use lazy_static::lazy_static;

/// permit2 utilities needed for compute intent signing
pub const PERMIT_TRANSFER_FROM_WITNESS_TYPEHASH_STUB: &str =
    "PermitWitnessTransferFrom(TokenPermissions permitted,address spender,uint256 nonce,uint256 deadline,";
pub const TOKEN_PERMISSIONS_TYPE_STRING: &str = "TokenPermissions(address token,uint256 amount)";
pub const PERMIT2_DOMAIN_SEPARATOR: B256 =
    b256!("94c1dec87927751697bfc9ebf6fc4ca506bed30308b518f0e9d6c5f74bbafdb8");
pub const PERMIT2_ADDRESS: Address = address!("000000000022D473030F116dDEE9F6B43aC78BA3");

lazy_static! {
    pub static ref TOKEN_PERMISSIONS_TYPE_HASH: B256 =
        keccak256(TOKEN_PERMISSIONS_TYPE_STRING.as_bytes());
}

#[must_use]
pub fn hash_typed_data(domain_separator: B256, data_hash: B256) -> B256 {
    let final_hash_preimage = [
        "\x19\x01".abi_encode_packed(),
        domain_separator.abi_encode(),
        data_hash.abi_encode(),
    ]
    .concat();

    keccak256(final_hash_preimage)
}
