use crate::{
    config::Markets,
    error::{Result, ServerError},
};
use deadpool_postgres::{Manager, Pool};
use taralli_primitives::{
    intents::{offer::ComputeOffer, ComputeIntent},
    server_utils::StoredIntent,
    systems::{System, SystemId},
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

#[derive(Clone)]
pub struct Db {
    pub pool: Pool,
}

impl Db {
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

        let should_recreate_intents = if !table_exists {
            // Table doesn't exist, create it and store the address
            tracing::info!("Creating market_address table for the first time");
            conn.batch_execute(
                "
                CREATE TABLE market_address (
                    id INTEGER PRIMARY KEY CHECK (id = 1),
                    porchetta_address BYTEA NOT NULL
                );
            ",
            )
            .await
            .map_err(|e| ServerError::DatabaseError(e.to_string()))?;

            conn.execute(UPDATE_MARKET_ADDRESS, &[&porchetta_market_address])
                .await
                .map_err(|e| ServerError::DatabaseError(e.to_string()))?;

            true // First run, create intents table
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

    /*async fn run_migrations(&self, porchetta_market_address: Vec<u8>) -> Result<()> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| ServerError::DatabaseError(e.to_string()))?;

        // First create the market address table if it doesn't exist
        conn.batch_execute(CREATE_MARKET_ADDRESS_TABLE)
            .await
            .map_err(|e| ServerError::DatabaseError(e.to_string()))?;

        // Check if we have a stored market address
        let stored_address_rows = conn
            .query(GET_STORED_MARKET_ADDRESS, &[])
            .await
            .map_err(|e| ServerError::DatabaseError(e.to_string()))?;

        let should_recreate_table = if stored_address_rows.is_empty() {
            // No stored address, store the current one
            conn.execute(UPDATE_MARKET_ADDRESS, &[&porchetta_market_address])
                .await
                .map_err(|e| ServerError::DatabaseError(e.to_string()))?;
            false
        } else {
            // We have a stored address, check if it matches the current one
            let stored_address: &[u8] = stored_address_rows[0].get(0);
            if stored_address != porchetta_market_address {
                // Address changed, update it and recreate the table
                tracing::info!("Market address changed from {:?} to {:?}, recreating intents table",
                    stored_address, porchetta_market_address);

                conn.execute(UPDATE_MARKET_ADDRESS, &[&porchetta_market_address])
                    .await
                    .map_err(|e| ServerError::DatabaseError(e.to_string()))?;
                true
            } else {
                false
            }
        };

        // Check if intents table exists
        let table_exists = conn
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


        // Drop the table if it exists to ensure we have the correct column types
        if table_exists {
            tracing::info!("Dropping existing market_address table to update schema");
            conn.batch_execute("DROP TABLE market_address;")
                .await
                .map_err(|e| ServerError::DatabaseError(e.to_string()))?;
        }

        if !table_exists || should_recreate_table {
            // Drop the table if it exists and we need to recreate it
            if table_exists && should_recreate_table {
                conn.batch_execute("DROP TABLE intents;")
                    .await
                    .map_err(|e| ServerError::DatabaseError(e.to_string()))?;
                tracing::info!("Dropped intents table due to market address change");
            }

            // Create the intents table
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

            tracing::info!("Created intents table");
        }

        Ok(())
    }*/
}

// #[derive(Clone, Debug, Serialize, Deserialize)]
// pub struct StoredIntent {
//     pub intent_id: B256,
//     pub system_id: String,
//     pub system: Vec<u8>,
//     pub proof_commitment: Vec<u8>,
//     pub signature: Vec<u8>,
//     pub expiration_ts: DateTime<Utc>,
//     pub created_at: DateTime<Utc>,
//     pub expired_at: Option<DateTime<Utc>>,
// }

// impl TryFrom<Row> for StoredIntent {
//     type Error = ServerError;
//     fn try_from(row: Row) -> Result<Self> {
//         let intent_id = match row.try_get::<_, Vec<u8>>("intent_id") {
//             Ok(bytes) => B256::from_slice(&bytes),
//             Err(e) => {
//                 tracing::error!("Failed to get intent_id: {}", e);
//                 return Err(ServerError::DatabaseError(format!(
//                     "POSTGRES: Failed to get intent_id: {}",
//                     e
//                 )));
//             }
//         };

//         let system_id = match row.try_get::<_, String>("system_id") {
//             Ok(id) => id,
//             Err(e) => {
//                 tracing::error!("Failed to get system_id: {}", e);
//                 return Err(ServerError::DatabaseError(format!(
//                     "POSTGRES: Failed to get system_id: {}",
//                     e
//                 )));
//             }
//         };

//         let system = match row.try_get::<_, Vec<u8>>("system") {
//             Ok(bytes) => bytes,
//             Err(e) => {
//                 tracing::error!("Failed to get system: {}", e);
//                 return Err(ServerError::DatabaseError(format!(
//                     "POSTGRES: Failed to get system: {}",
//                     e
//                 )));
//             }
//         };

//         let proof_commitment = match row.try_get::<_, Vec<u8>>("proof_commitment") {
//             Ok(bytes) => bytes,
//             Err(e) => {
//                 tracing::error!("Failed to get proof_commitment: {}", e);
//                 return Err(ServerError::DatabaseError(format!(
//                     "POSTGRES: Failed to get proof_commitment: {}",
//                     e
//                 )));
//             }
//         };

//         let signature = match row.try_get::<_, Vec<u8>>("signature") {
//             Ok(bytes) => bytes,
//             Err(e) => {
//                 tracing::error!("Failed to get signature: {}", e);
//                 return Err(ServerError::DatabaseError(format!(
//                     "POSTGRES: Failed to get signature: {}",
//                     e
//                 )));
//             }
//         };

//         let expiration_ts = match row.try_get::<_, chrono::DateTime<Utc>>("expiration_ts") {
//             Ok(ts) => ts,
//             Err(e) => {
//                 tracing::error!("Failed to get expiration_ts: {}", e);
//                 return Err(ServerError::DatabaseError(format!(
//                     "POSTGRES: Failed to get expiration_ts: {}",
//                     e
//                 )));
//             }
//         };

//         let created_at = match row.try_get::<_, chrono::DateTime<Utc>>("created_at") {
//             Ok(ts) => ts,
//             Err(e) => {
//                 tracing::error!("Failed to get created_at: {}", e);
//                 return Err(ServerError::DatabaseError(format!(
//                     "POSTGRES: Failed to get created_at: {}",
//                     e
//                 )));
//             }
//         };

//         // This field can be NULL, so use try_get with Option
//         let expired_at = match row.try_get::<_, Option<chrono::DateTime<Utc>>>("expired_at") {
//             Ok(ts) => ts,
//             Err(e) => {
//                 tracing::error!("Failed to get expired_at: {}", e);
//                 return Err(ServerError::DatabaseError(format!(
//                     "POSTGRES: Failed to get expired_at: {}",
//                     e
//                 )));
//             }
//         };

//         Ok(StoredIntent {
//             intent_id,
//             system_id,
//             system,
//             proof_commitment,
//             signature,
//             expiration_ts,
//             created_at,
//             expired_at,
//         })
//     }
// }

// impl<S: System> TryFrom<StoredIntent> for ComputeOffer<S>
// where
//     S: for<'de> serde::Deserialize<'de>,
// {
//     type Error = ServerError;

//     fn try_from(stored: StoredIntent) -> std::result::Result<Self, Self::Error> {
//         // Parse the system from the binary data
//         let system: S = serde_json::from_slice(&stored.system).map_err(|e| {
//             ServerError::DeserializationError(format!("Failed to deserialize system: {}", e))
//         })?;

//         // Parse the proof_commitment from binary data
//         let proof_offer = serde_json::from_slice(&stored.proof_commitment).map_err(|e| {
//             ServerError::DeserializationError(format!(
//                 "Failed to deserialize proof_commitment: {}",
//                 e
//             ))
//         })?;

//         // Convert system_id string to SystemId
//         let system_id = SystemId::try_from(stored.system_id.as_str())
//             .map_err(|e| ServerError::DeserializationError(format!("Invalid system_id: {}", e)))?;

//         // Convert signature bytes to Signature type
//         let signature = PrimitiveSignature::try_from(stored.signature.as_slice())
//             .map_err(|e| ServerError::DeserializationError(format!("Invalid signature: {}", e)))?;

//         // Construct and return the ComputeOffer
//         Ok(ComputeOffer {
//             system,
//             system_id,
//             proof_offer,
//             signature,
//         })
//     }
// }

impl Db {
    /// Store a submitted ComputeOffer within the database
    pub async fn store_offer<S: System>(&self, offer: &ComputeOffer<S>) -> Result<StoredIntent> {
        let offer_id = offer.compute_id();
        let system_bytes = serde_json::to_vec(&offer.system)
            .map_err(|e| ServerError::SerializationError(e.to_string()))?;
        let proof_commitment_bytes = serde_json::to_vec(&offer.proof_offer)
            .map_err(|e| ServerError::SerializationError(e.to_string()))?;
        let expiration_timestamp = offer.proof_offer.endAuctionTimestamp as f64;

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
                    &offer.system_id.as_str(),
                    &system_bytes,
                    &proof_commitment_bytes,
                    &offer.signature.as_bytes().to_vec(),
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
