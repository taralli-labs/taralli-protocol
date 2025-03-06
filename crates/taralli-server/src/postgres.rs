use crate::error::{Result, ServerError};
use alloy::primitives::B256;
use chrono::{DateTime, Utc};
use deadpool_postgres::{Manager, Pool};
use serde::Serialize;
use taralli_primitives::{
    intents::{offer::ComputeOffer, ComputeIntent},
    systems::{System, SystemId},
};
use tokio_postgres::{Config, NoTls, Row};

pub const INSERT_INTENT: &str = "
    INSERT INTO intents (intent_id, system_id, system, proof_commitment, signature, expiration_ts)
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
    RETURNING intent_id, system_id, system, proof_commitment, signature, expiration_ts, created_at, expired_at;
";

pub const UPDATE_EXPIRED_INTENTS: &str = "
    UPDATE intents
    SET expired_at = NOW()
    WHERE intents.expiration_ts > NOW()
    AND expired_at IS NULL
    RETURNING intent_id, system_id, system, proof_commitment, signature, expiration_ts, created_at, expired_at;
";

pub const GET_INTENTS_BY_ID: &str = "
    SELECT (intent_id, system_id, system, proof_commitment, signature, expiration_ts, created_at, expired_at) FROM intents
    WHERE intents.system_id = $1
    AND intents.expired_at IS NULL;
";

#[derive(Clone)]
pub struct Db {
    pub pool: Pool,
}

impl Db {
    pub async fn new() -> Self {
        let mut config: Config = Config::new();
        config.host(std::env::var("POSTGRES_URL").unwrap_or("localhost".to_string()));
        let postgres_port = std::env::var("POSTGRES_PORT").unwrap_or("5432".to_string());
        config.port(postgres_port.parse::<u16>().unwrap_or(5432));
        config.user(std::env::var("POSTGRES_USER").unwrap_or("taralli".to_string()));
        config.dbname(std::env::var("POSTGRES_DB").unwrap_or("taralli-db".to_string()));

        let manager = Manager::new(config, NoTls);
        let pool = Pool::builder(manager)
            .max_size(5)
            .build()
            .expect("deadpool builder failed");
        {
            let client = pool.get().await.expect("deadpool get() failed");
            client
                .simple_query("SELECT 1")
                .await
                .expect("deadpool simple_query() failed");
        }
        Db { pool }
    }
}

#[derive(Debug, Serialize)]
pub struct StoredIntent {
    pub intent_id: B256,
    pub system_id: String,
    pub system: Vec<u8>,
    pub proof_commitment: Vec<u8>,
    pub signature: Vec<u8>,
    pub expiration_ts: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub expired_at: DateTime<Utc>,
}

impl TryFrom<Row> for StoredIntent {
    type Error = ServerError;
    fn try_from(row: Row) -> Result<Self> {
        let intent_id: B256 = B256::from_slice(row.get("intent_id"));
        let system_id: String = row.get("system_id");
        let system: Vec<u8> = row.get("system");
        let proof_commitment: Vec<u8> = row.get("proof_commitment");
        let signature: Vec<u8> = row.get("signature");
        let expiration_ts: chrono::DateTime<Utc> = row.get("expiration_ts");
        let created_at: chrono::DateTime<Utc> = row.get("created_at");
        let expired_at: chrono::DateTime<Utc> = row.get("expired_at");
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

impl Db {
    /// Store a submitted ComputeOffer within the database
    pub async fn store_offer<S: System>(&self, offer: &ComputeOffer<S>) -> Result<StoredIntent> {
        let offer_id = offer.compute_id();
        let proving_system_bytes = serde_json::to_vec(&offer.system)
            .map_err(|e| ServerError::SerializationError(e.to_string()))?;
        let proof_offer_bytes = serde_json::to_vec(&offer.proof_offer)
            .map_err(|e| ServerError::SerializationError(e.to_string()))?;
        let expiration_timestamp = offer.proof_offer.endAuctionTimestamp as i64;

        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| ServerError::DatabaseError(e.to_string()))?;

        let prepared_stmt = conn
            .prepare(INSERT_INTENT)
            .await
            .map_err(|e| ServerError::DatabaseError(e.to_string()))?;

        let row = conn
            .query_one(
                &prepared_stmt,
                &[
                    &offer_id.as_slice(),
                    &offer.system_id.as_str(),
                    &proving_system_bytes,
                    &proof_offer_bytes,
                    &offer.signature.as_bytes().to_vec(),
                    &expiration_timestamp,
                ],
            )
            .await
            .map_err(|e| ServerError::DatabaseError(e.to_string()))?;
        StoredIntent::try_from(row)
    }

    /// query db for active compute intents by system id
    pub async fn get_active_intents_by_id(&self, system_id: SystemId) -> Result<Vec<StoredIntent>> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| ServerError::DatabaseError(e.to_string()))?;
        let prepared_stmt = conn
            .prepare(GET_INTENTS_BY_ID)
            .await
            .map_err(|e| ServerError::DatabaseError(e.to_string()))?;
        let rows = conn
            .query(&prepared_stmt, &[&system_id.as_str()])
            .await
            .map_err(|e| ServerError::DatabaseError(e.to_string()))?;

        rows.into_iter()
            .map(StoredIntent::try_from)
            .collect::<Result<Vec<_>>>()
    }

    /// Update all expired compute offers by checking which ones have passed their expiration timestamp
    /// Returns the list of offers that were marked as expired
    pub async fn update_expired_intents(&self) -> Result<Vec<StoredIntent>> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| ServerError::DatabaseError(e.to_string()))?;

        let stmt = conn
            .prepare(UPDATE_EXPIRED_INTENTS)
            .await
            .map_err(|e| ServerError::DatabaseError(e.to_string()))?;

        let rows = conn
            .query(&stmt, &[])
            .await
            .map_err(|e| ServerError::DatabaseError(e.to_string()))?;

        rows.into_iter()
            .map(StoredIntent::try_from)
            .collect::<Result<Vec<_>>>()
    }
}
