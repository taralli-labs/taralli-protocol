use crate::error::{Result, ServerError};
use deadpool_postgres::{Manager, Pool};
use taralli_primitives::{
    compression_utils::{db::StoredIntent, intents::ComputeOfferCompressed},
    intents::offer::compute_offer_id,
    systems::SystemId,
};
use tokio_postgres::{Config, NoTls};

pub const CREATE_INTENTS_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS intents (
        intent_id BYTEA PRIMARY KEY,
        system_id TEXT NOT NULL,
        system BYTEA NOT NULL,
        proof_commitment BYTEA NOT NULL,
        signature BYTEA NOT NULL,
        expiration_ts TIMESTAMPTZ NOT NULL,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        expired_at TIMESTAMPTZ DEFAULT NULL
    );

    CREATE INDEX IF NOT EXISTS idx_intents_system ON intents(system_id);
    CREATE INDEX IF NOT EXISTS idx_intents_expiration ON intents(expiration_ts);
";

pub const INSERT_INTENT: &str = "
    INSERT INTO intents (intent_id, system_id, system, proof_commitment, signature, expiration_ts)
    VALUES ($1, $2, $3, $4, $5, to_timestamp($6))
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
    SELECT intent_id, system_id, system, proof_commitment, signature, expiration_ts, created_at, expired_at FROM intents
    WHERE intents.system_id = $1
    AND intents.expiration_ts > NOW()
    AND intents.expired_at IS NULL;
";

/// Postgres database used to store compute intents (currently ComputeOffers only)
#[derive(Clone)]
pub struct Db {
    pub pool: Pool,
}

impl Db {
    /// instantiate new/existing postgres intent db
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

        let db = Db { pool };
        // Run migrations on startup
        db.run_migrations().await.expect("Failed to run migrations");
        db
    }

    /// Create a fresh postgres table if market addresses change or load the existing intent db
    async fn run_migrations(&self) -> Result<()> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| ServerError::DatabaseError(e.to_string()))?;

        // Define all tables and their creation statements
        let tables = vec![
            ("intents", CREATE_INTENTS_TABLE), // Add more tables here as needed
        ];

        for (table_name, ddl) in tables {
            tracing::info!("Ensuring {} table exists", table_name);
            conn.batch_execute(ddl)
                .await
                .map_err(|e| ServerError::DatabaseError(e.to_string()))?;
        }

        Ok(())
    }

    /// Store a submitted ComputeOffer within the database
    pub async fn store_offer(
        &self,
        compressed_offer: &ComputeOfferCompressed,
    ) -> Result<StoredIntent> {
        let offer_id = compute_offer_id(&compressed_offer.proof_offer, &compressed_offer.signature);
        let proof_commitment_bytes = serde_json::to_vec(&compressed_offer.proof_offer)
            .map_err(|e| ServerError::SerializationError(e.to_string()))?;
        let expiration_timestamp = compressed_offer.proof_offer.endAuctionTimestamp as f64;

        tracing::info!("POSTGRES: attempting to store intent with ID: {}", offer_id);

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
                    &compressed_offer.system_id.as_str(),
                    &compressed_offer.system,
                    &proof_commitment_bytes,
                    &compressed_offer.signature.as_bytes().to_vec(),
                    &expiration_timestamp,
                ],
            )
            .await
            .map_err(|e| ServerError::DatabaseError(e.to_string()))?;

        tracing::info!("POSTGRES: stored intent with ID: {}", offer_id);

        Ok(StoredIntent::try_from(row)?)
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
            .map(|r| r.map_err(ServerError::PrimitivesError))
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
            .map(|r| r.map_err(ServerError::PrimitivesError))
            .collect::<Result<Vec<_>>>()
    }
}
