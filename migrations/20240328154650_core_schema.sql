-- Add migration script here
DROP TABLE IF EXISTS transactions CASCADE;
DROP TABLE IF EXISTS features CASCADE;
DROP TABLE IF EXISTS triggered_rules CASCADE;
DROP TABLE IF EXISTS processing_queue CASCADE;
DROP TABLE IF EXISTS scoring_events CASCADE;
DROP TABLE IF EXISTS channels CASCADE;
DROP TABLE IF EXISTS models CASCADE;
DROP TABLE IF EXISTS labels CASCADE;
DROP TABLE IF EXISTS scoring_rules CASCADE;

CREATE TABLE IF NOT EXISTS labels (
    id BIGSERIAL PRIMARY KEY,
    fraud_level TEXT NOT NULL,
    fraud_category TEXT NOT NULL,
    label_source TEXT NOT NULL,
    labeled_by TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_labels_created_at ON labels(created_at);
CREATE INDEX IF NOT EXISTS idx_labels_fraud_level ON labels(fraud_level);
CREATE INDEX IF NOT EXISTS idx_labels_fraud_category ON labels(fraud_category);
CREATE INDEX IF NOT EXISTS idx_labels_label_source ON labels(label_source);

CREATE TABLE IF NOT EXISTS transactions (
    id BIGSERIAL PRIMARY KEY,
    label_id BIGINT,
    comment TEXT,
    last_scoring_date TIMESTAMP,
    processing_complete BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    FOREIGN KEY (label_id) REFERENCES labels(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_transactions_created_at ON transactions(created_at);
CREATE INDEX IF NOT EXISTS idx_transactions_label_id ON transactions(label_id);
CREATE INDEX IF NOT EXISTS idx_transactions_last_scoring_date ON transactions(last_scoring_date);
CREATE INDEX IF NOT EXISTS idx_transactions_processing_complete ON transactions(processing_complete);

CREATE TABLE IF NOT EXISTS models (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    features_schema_version_major INTEGER NOT NULL,
    features_schema_version_minor INTEGER NOT NULL,
    version TEXT NOT NULL DEFAULT '1.0',
    model_type TEXT NOT NULL DEFAULT 'rule_based',
    created_at TIMESTAMP NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_models_created_at ON models(created_at);
CREATE INDEX IF NOT EXISTS idx_models_model_type_version ON models(model_type, version);
CREATE INDEX IF NOT EXISTS idx_models_features_schema_version_major_minor 
    ON models(features_schema_version_major, features_schema_version_minor);


CREATE TABLE IF NOT EXISTS channels (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    model_id BIGINT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    FOREIGN KEY (model_id) REFERENCES models(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_channels_created_at ON channels(created_at);
CREATE INDEX IF NOT EXISTS idx_channels_model_id ON channels(model_id);
CREATE INDEX IF NOT EXISTS idx_channels_name ON channels(name);


create table if not exists scoring_events (
    id BIGSERIAL PRIMARY KEY,
    transaction_id BIGINT NOT NULL,
    channel_id BIGINT NOT NULL,
    total_score INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    FOREIGN KEY (transaction_id) REFERENCES transactions(id) ON DELETE CASCADE,
    FOREIGN KEY (channel_id) REFERENCES channels(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_scoring_events_transaction_id ON scoring_events(transaction_id);
CREATE INDEX IF NOT EXISTS idx_scoring_events_channel_id ON scoring_events(channel_id);
CREATE INDEX IF NOT EXISTS idx_scoring_events_created_at ON scoring_events(created_at);


CREATE TABLE IF NOT EXISTS scoring_rules (
    id BIGSERIAL PRIMARY KEY,
    model_id BIGINT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    rule JSONB NOT NULL,
    score INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    FOREIGN KEY (model_id) REFERENCES models(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_scoring_rules_model_id ON scoring_rules(model_id);
CREATE INDEX IF NOT EXISTS idx_scoring_rules_name ON scoring_rules(name);
CREATE INDEX IF NOT EXISTS idx_scoring_rules_created_at ON scoring_rules(created_at);


CREATE TABLE IF NOT EXISTS triggered_rules (
    id BIGSERIAL PRIMARY KEY,
    scoring_events_id BIGINT NOT NULL,
    rule_id BIGINT NOT NULL,
    FOREIGN KEY (scoring_events_id) REFERENCES scoring_events(id) ON DELETE CASCADE,
    FOREIGN KEY (rule_id) REFERENCES scoring_rules(id)
);
CREATE INDEX IF NOT EXISTS idx_triggered_rules_scoring_events_id ON triggered_rules(scoring_events_id);
CREATE INDEX IF NOT EXISTS idx_triggered_rules_rule_id ON triggered_rules(rule_id);


CREATE TABLE IF NOT EXISTS features (
    id BIGSERIAL PRIMARY KEY,
    transaction_id BIGINT NOT NULL,
    transaction_version INT NOT NULL,
    schema_version_major INTEGER NOT NULL,
    schema_version_minor INTEGER NOT NULL,
    simple_features JSONB,
    graph_features JSONB NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    FOREIGN KEY (transaction_id) REFERENCES transactions(id) ON DELETE CASCADE
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_features_transaction_id_version ON features(transaction_id, transaction_version);
CREATE INDEX IF NOT EXISTS idx_features_transaction_id ON features(transaction_id);
CREATE INDEX IF NOT EXISTS idx_features_created_at ON features(created_at);
CREATE INDEX IF NOT EXISTS idx_features_schema_version_major_minor ON features(schema_version_major, schema_version_minor);

CREATE TABLE IF NOT EXISTS processing_queue (
    id BIGSERIAL PRIMARY KEY,
    processable_id BIGINT NOT NULL,
    processed_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_processing_queue_processable_id ON processing_queue(processable_id);
CREATE INDEX IF NOT EXISTS idx_processing_queue_created_at ON processing_queue(created_at);

CREATE TABLE IF NOT EXISTS recalculation_queue (
    id BIGSERIAL PRIMARY KEY,
    processable_id BIGINT NOT NULL,
    processed_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_recalculation_queue_processable_id ON recalculation_queue(processable_id);
CREATE INDEX IF NOT EXISTS idx_recalculation_queue_created_at ON recalculation_queue(created_at);
