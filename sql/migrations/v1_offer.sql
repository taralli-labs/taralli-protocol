CREATE TABLE offers (
    offer_id BYTEA PRIMARY KEY,
    proving_system_id TEXT NOT NULL,  -- Store as string, parse to enum
    proving_system BYTEA NOT NULL,       -- Serialized proving system data
    proof_offer BYTEA NOT NULL,          -- Serialized UniversalPorchetta::ProofOffer
    signature BYTEA NOT NULL,
    expiration_timestamp TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expired_at TIMESTAMPTZ DEFAULT NULL
);

CREATE INDEX idx_offers_proving_system ON offers(proving_system_id);
CREATE INDEX idx_offers_expiration ON offers(expiration_timestamp);