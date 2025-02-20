use crate::error::{Result, ServerError};
use alloy::primitives::B256;
use chrono::{DateTime, Utc};
use deadpool_postgres::{Manager, Pool};
use serde::Serialize;
use taralli_primitives::{
    intents::ComputeOffer,
    systems::{System, SystemId},
    utils::compute_offer_id,
};
use tokio_postgres::{Config, NoTls, Row};

pub const INSERT_OFFER: &str = "
    INSERT INTO offers (offer_id, proving_system_id, proving_system, proof_offer, signature, expiration_timestamp)
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
    RETURNING offer_id, proving_system_id, proving_system, proof_offer, signature, expiration_timestamp, created_at, expired_at;
";

pub const UPDATE_EXPIRED_OFFERS: &str = "
    UPDATE offers
    SET expired_at = NOW()
    WHERE offers.expiration_timestamp > NOW()
    AND expired_at IS NULL
    RETURNING offer_id, proving_system_id, proving_system, proof_offer, signature, expiration_timestamp, created_at, expired_at;
";

pub const GET_OFFER_BY_ID: &str = "
    SELECT (offer_id, proving_system_id, proving_system, proof_offer, signature, expiration_timestamp, created_at, expired_at) FROM offers
    WHERE offers.proving_system_id = $1
    AND offers.expired_at IS NULL;
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
pub struct StoredOffer {
    pub offer_id: B256,
    pub proving_system_id: String,
    pub proving_system: Vec<u8>,
    pub proof_offer: Vec<u8>,
    pub signature: Vec<u8>,
    pub expiration_timestamp: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub expired_at: DateTime<Utc>,
}

impl TryFrom<Row> for StoredOffer {
    type Error = ServerError;
    fn try_from(row: Row) -> Result<Self> {
        let offer_id: B256 = B256::from_slice(row.get("offer_id"));
        let proving_system_id: String = row.get("proving_system_id");
        let proving_system: Vec<u8> = row.get("proving_system");
        let proof_offer: Vec<u8> = row.get("proof_offer");
        let signature: Vec<u8> = row.get("signature");
        let expiration_timestamp: chrono::DateTime<Utc> = row.get("expiration_timestamp");
        let created_at: chrono::DateTime<Utc> = row.get("created_at");
        let expired_at: chrono::DateTime<Utc> = row.get("expired_at");
        Ok(StoredOffer {
            offer_id,
            proving_system_id,
            proving_system,
            proof_offer,
            signature,
            expiration_timestamp,
            created_at,
            expired_at,
        })
    }
}

impl Db {
    /// Store a submitted ComputeOffer within the database
    pub async fn store_offer<S: System>(&self, offer: &ComputeOffer<S>) -> Result<StoredOffer> {
        let offer_id = compute_offer_id(&offer.proof_offer, &offer.signature);
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
            .prepare(INSERT_OFFER)
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
        StoredOffer::try_from(row)
    }

    /// query db for active compute offers by proving system id
    pub async fn get_active_offers_by_id(&self, system_id: SystemId) -> Result<Vec<StoredOffer>> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| ServerError::DatabaseError(e.to_string()))?;
        let prepared_stmt = conn
            .prepare(GET_OFFER_BY_ID)
            .await
            .map_err(|e| ServerError::DatabaseError(e.to_string()))?;
        let rows = conn
            .query(&prepared_stmt, &[&system_id.as_str()])
            .await
            .map_err(|e| ServerError::DatabaseError(e.to_string()))?;

        rows.into_iter()
            .map(StoredOffer::try_from)
            .collect::<Result<Vec<_>>>()
    }

    /// Update all expired compute offers by checking which ones have passed their expiration timestamp
    /// Returns the list of offers that were marked as expired
    pub async fn update_expired_offers(&self) -> Result<Vec<StoredOffer>> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| ServerError::DatabaseError(e.to_string()))?;

        let stmt = conn
            .prepare(UPDATE_EXPIRED_OFFERS)
            .await
            .map_err(|e| ServerError::DatabaseError(e.to_string()))?;

        let rows = conn
            .query(&stmt, &[])
            .await
            .map_err(|e| ServerError::DatabaseError(e.to_string()))?;

        rows.into_iter()
            .map(StoredOffer::try_from)
            .collect::<Result<Vec<_>>>()
    }
}
