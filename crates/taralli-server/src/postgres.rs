use crate::{
    config::Markets,
    error::{Result, ServerError},
};
use deadpool_postgres::{Manager, Pool};
use taralli_primitives::{
    intents::offer::compute_offer_id,
    server_utils::{db::StoredIntent, intents::ComputeOfferCompressed},
    systems::SystemId,
};
use tokio_postgres::{Config, NoTls};

pub const CREATE_MARKET_ADDRESS_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS market_address (
        id INTEGER PRIMARY KEY CHECK (id = 1),
        porchetta_address BYTEA NOT NULL
    );
";

pub const GET_STORED_MARKET_ADDRESS: &str = "
    SELECT porchetta_address FROM market_address WHERE id = 1;
";

pub const UPDATE_MARKET_ADDRESS: &str = "
    INSERT INTO market_address (id, porchetta_address)
    VALUES (1, $1)
    ON CONFLICT (id) DO UPDATE SET porchetta_address = $1;
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
    AND intents.expired_at IS NULL;
";

/// Postgres database used to store compute intents (currently ComputeOffers only)
#[derive(Clone)]
pub struct Db {
    pub pool: Pool,
}

impl Db {
    /// instantiate new/existing postgres intent db
    pub async fn new(markets: Markets) -> Self {
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
        db.run_migrations(markets.universal_porchetta.to_vec())
            .await
            .expect("Failed to run migrations");
        db
    }

    /// Create a fresh postgres table if market addresses change or load the existing intent db
    async fn run_migrations(&self, porchetta_market_address: Vec<u8>) -> Result<()> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| ServerError::DatabaseError(e.to_string()))?;

        // Check if the market_address table exists
        let table_exists = conn
            .query_one(
                "SELECT EXISTS (
                SELECT FROM information_schema.tables 
                WHERE table_schema = 'public' 
                AND table_name = 'market_address'
            )",
                &[],
            )
            .await
            .map_err(|e| ServerError::DatabaseError(e.to_string()))?
            .get::<_, bool>(0);

        // Create market_address table if it doesn't exist
        if !table_exists {
            tracing::info!("Creating market_address table for the first time");
            conn.batch_execute(CREATE_MARKET_ADDRESS_TABLE)
                .await
                .map_err(|e| ServerError::DatabaseError(e.to_string()))?;
        }

        // Check if we need to update the market address
        let should_recreate_intents = if !table_exists {
            // First run, store the address and create intents table
            conn.execute(UPDATE_MARKET_ADDRESS, &[&porchetta_market_address])
                .await
                .map_err(|e| ServerError::DatabaseError(e.to_string()))?;
            true
        } else {
            // Table exists, check if the address matches
            let stored_address_rows = conn
                .query(GET_STORED_MARKET_ADDRESS, &[])
                .await
                .map_err(|e| ServerError::DatabaseError(e.to_string()))?;

            if stored_address_rows.is_empty() {
                // No address stored (shouldn't happen, but handle it)
                conn.execute(UPDATE_MARKET_ADDRESS, &[&porchetta_market_address])
                    .await
                    .map_err(|e| ServerError::DatabaseError(e.to_string()))?;
                true
            } else {
                // Compare stored address with current one
                let stored_address: &[u8] = stored_address_rows[0].get(0);

                if stored_address != porchetta_market_address.as_slice() {
                    // Address changed, update it and recreate intents table
                    tracing::info!("Market address changed, updating and recreating intents table");
                    conn.execute(UPDATE_MARKET_ADDRESS, &[&porchetta_market_address])
                        .await
                        .map_err(|e| ServerError::DatabaseError(e.to_string()))?;
                    true
                } else {
                    // Address matches, no need to recreate intents table
                    tracing::info!("Market address unchanged, using existing tables");
                    false
                }
            }
        };

        // Check if intents table exists
        let intents_exists = conn
            .query_one(
                "SELECT EXISTS (
            SELECT FROM information_schema.tables 
            WHERE table_schema = 'public' 
            AND table_name = 'intents'
        )",
                &[],
            )
            .await
            .map_err(|e| ServerError::DatabaseError(e.to_string()))?
            .get::<_, bool>(0);

        // Drop and recreate intents table if needed
        if should_recreate_intents || !intents_exists {
            if intents_exists {
                tracing::info!("Dropping existing intents table due to market address change");
                conn.batch_execute("DROP TABLE intents;")
                    .await
                    .map_err(|e| ServerError::DatabaseError(e.to_string()))?;
            }

            // Create the intents table
            tracing::info!("Creating intents table");
            conn.batch_execute(
                "CREATE TABLE intents (
                    intent_id BYTEA PRIMARY KEY,
                    system_id TEXT NOT NULL,
                    system BYTEA NOT NULL,
                    proof_commitment BYTEA NOT NULL,
                    signature BYTEA NOT NULL,
                    expiration_ts TIMESTAMPTZ NOT NULL,
                    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    expired_at TIMESTAMPTZ DEFAULT NULL
                );

                CREATE INDEX idx_intents_system ON intents(system_id);
                CREATE INDEX idx_intents_expiration ON intents(expiration_ts);",
            )
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

        tracing::info!("POSTGRES: attempting to store intent");

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

        tracing::info!("POSTGRES: stored intent");

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
