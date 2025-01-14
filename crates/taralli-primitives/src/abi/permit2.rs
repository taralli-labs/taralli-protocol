use alloy::sol;
use serde::{Deserialize, Serialize};

sol! {
    #[sol(rpc)]
    #[derive(Serialize, Deserialize, Debug)]
    Permit2,
    "Permit2.json"
}
