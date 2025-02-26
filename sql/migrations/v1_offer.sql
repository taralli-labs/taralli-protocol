CREATE TABLE intents (
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
CREATE INDEX idx_intents_expiration ON intents(expiration_ts);