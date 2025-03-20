use alloy::primitives::{PrimitiveSignature, B256};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;

use crate::{
    intents::offer::ComputeOffer,
    systems::{System, SystemId},
    PrimitivesError, Result,
};

/// Opaque compute intent structure stored within the protocol server's database.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredIntent {
    pub intent_id: B256,
    pub system_id: String,
    pub system: Vec<u8>,
    pub proof_commitment: Vec<u8>,
    pub signature: Vec<u8>,
    pub expiration_ts: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub expired_at: Option<DateTime<Utc>>,
}

/// Convert a postgres row returned from a query into the stored intent
impl TryFrom<Row> for StoredIntent {
    type Error = PrimitivesError;
    fn try_from(row: Row) -> Result<Self> {
        let intent_id = match row.try_get::<_, Vec<u8>>("intent_id") {
            Ok(bytes) => B256::from_slice(&bytes),
            Err(e) => {
                return Err(PrimitivesError::DbSerializeError(format!(
                    "Failed to get intent_id: {}",
                    e
                )));
            }
        };

        let system_id = match row.try_get::<_, String>("system_id") {
            Ok(id) => id,
            Err(e) => {
                return Err(PrimitivesError::DbSerializeError(format!(
                    "Failed to get system_id: {}",
                    e
                )));
            }
        };

        let system = match row.try_get::<_, Vec<u8>>("system") {
            Ok(bytes) => bytes,
            Err(e) => {
                return Err(PrimitivesError::DbSerializeError(format!(
                    "Failed to get system: {}",
                    e
                )));
            }
        };

        let proof_commitment = match row.try_get::<_, Vec<u8>>("proof_commitment") {
            Ok(bytes) => bytes,
            Err(e) => {
                return Err(PrimitivesError::DbSerializeError(format!(
                    "Failed to get proof_commitment: {}",
                    e
                )));
            }
        };

        let signature = match row.try_get::<_, Vec<u8>>("signature") {
            Ok(bytes) => bytes,
            Err(e) => {
                return Err(PrimitivesError::DbSerializeError(format!(
                    "Failed to get signature: {}",
                    e
                )));
            }
        };

        let expiration_ts = match row.try_get::<_, chrono::DateTime<Utc>>("expiration_ts") {
            Ok(ts) => ts,
            Err(e) => {
                return Err(PrimitivesError::DbSerializeError(format!(
                    "Failed to get expiration_ts: {}",
                    e
                )));
            }
        };

        let created_at = match row.try_get::<_, chrono::DateTime<Utc>>("created_at") {
            Ok(ts) => ts,
            Err(e) => {
                return Err(PrimitivesError::DbSerializeError(format!(
                    "Failed to get created_at: {}",
                    e
                )));
            }
        };

        // This field can be NULL, so use try_get with Option
        let expired_at = match row.try_get::<_, Option<chrono::DateTime<Utc>>>("expired_at") {
            Ok(ts) => ts,
            Err(e) => {
                return Err(PrimitivesError::DbSerializeError(format!(
                    "Failed to get expired_at: {}",
                    e
                )));
            }
        };

        Ok(StoredIntent {
            intent_id,
            system_id,
            system,
            proof_commitment,
            signature,
            expiration_ts,
            created_at,
            expired_at,
        })
    }
}

/// Convert a StoredIntent into a ComputeOffer
impl<S: System> TryFrom<StoredIntent> for ComputeOffer<S>
where
    S: for<'de> serde::Deserialize<'de>,
{
    type Error = PrimitivesError;

    fn try_from(stored: StoredIntent) -> std::result::Result<Self, Self::Error> {
        // Parse the system from the binary data
        let system: S = serde_json::from_slice(&stored.system).map_err(|e| {
            PrimitivesError::DbDeserializeError(format!("Failed to deserialize system: {}", e))
        })?;

        // Parse the proof_commitment from binary data
        let proof_offer = serde_json::from_slice(&stored.proof_commitment).map_err(|e| {
            PrimitivesError::DbDeserializeError(format!(
                "Failed to deserialize proof_commitment: {}",
                e
            ))
        })?;

        // Convert system_id string to SystemId
        let system_id = SystemId::try_from(stored.system_id.as_str()).map_err(|e| {
            PrimitivesError::DbDeserializeError(format!("Invalid system_id: {}", e))
        })?;

        // Convert signature bytes to Signature type
        let signature = PrimitiveSignature::try_from(stored.signature.as_slice()).map_err(|e| {
            PrimitivesError::DbDeserializeError(format!("Invalid signature: {}", e))
        })?;

        // Construct and return the ComputeOffer
        Ok(ComputeOffer {
            system,
            system_id,
            proof_offer,
            signature,
        })
    }
}
