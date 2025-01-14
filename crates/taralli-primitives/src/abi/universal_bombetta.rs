use alloy::sol;
use serde::{Deserialize, Serialize};

sol! {
    #[sol(rpc)]
    #[derive(Serialize, Deserialize, Debug)]
    UniversalBombetta,
    "UniversalBombetta.json"
}

// UniversalBombetta.VerifierDetails
sol! {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct VerifierDetails {
        address verifier;
        bytes4 selector;
        bool isShaCommitment;
        uint256 publicInputsOffset;
        uint256 publicInputsLength;
        bool hasPartialCommitmentResultCheck;
        uint256 submittedPartialCommitmentResultOffset;
        uint256 submittedPartialCommitmentResultLength;
        bytes32 predeterminedPartialCommitment;
    }
}
