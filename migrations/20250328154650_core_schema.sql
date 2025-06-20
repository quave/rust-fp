-- Add migration script here
DROP TABLE IF EXISTS transactions;
DROP TABLE IF EXISTS features;
DROP TABLE IF EXISTS triggered_rules;
DROP TABLE IF EXISTS processing_queue;

CREATE TABLE IF NOT EXISTS transactions (
    id BIGSERIAL PRIMARY KEY,
    created_at TIMESTAMP NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS triggered_rules (
    id BIGSERIAL PRIMARY KEY,
    transaction_id BIGINT NOT NULL,
    rule_name TEXT NOT NULL,
    rule_score INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    FOREIGN KEY (transaction_id) REFERENCES transactions(id)
);
CREATE INDEX IF NOT EXISTS idx_triggered_rules_transaction_id ON triggered_rules(transaction_id);
CREATE INDEX IF NOT EXISTS idx_triggered_rules_created_at ON triggered_rules(created_at);

CREATE TABLE IF NOT EXISTS features (
    id BIGSERIAL PRIMARY KEY,
    transaction_id BIGINT NOT NULL,
    schema_version_major INTEGER NOT NULL,
    schema_version_minor INTEGER NOT NULL,
    payload JSONB NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    FOREIGN KEY (transaction_id) REFERENCES transactions(id)
);
CREATE INDEX IF NOT EXISTS idx_features_transaction_id ON features(transaction_id);
CREATE INDEX IF NOT EXISTS idx_features_created_at ON features(created_at);

CREATE TABLE IF NOT EXISTS processing_queue (
    id BIGSERIAL PRIMARY KEY,
    processable_id BIGINT NOT NULL,
    processed_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_processing_queue_processable_id ON processing_queue(processable_id);
CREATE INDEX IF NOT EXISTS idx_processing_queue_created_at ON processing_queue(created_at);
