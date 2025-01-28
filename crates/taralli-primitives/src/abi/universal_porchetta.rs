use alloy::sol;
use serde::{Deserialize, Serialize};

sol! {
    #[sol(rpc)]
    #[derive(Serialize, Deserialize, Debug)]
    UniversalPorchetta,
    "UniversalPorchetta.json"
}

// UniversalPorchetta.VerifierDetails
sol! {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct VerifierDetails {
        address verifier;
        bytes4 selector;
        bool isShaCommitment;
        uint256 inputsOffset;
        uint256 inputsLength;
    }
}

pub type ProofOfferVerifierDetails = VerifierDetails;
