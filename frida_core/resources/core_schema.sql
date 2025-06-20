-- Tables for processing_queue, features, triggered_rules
CREATE TABLE IF NOT EXISTS processing_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    processable_id INTEGER NOT NULL,
    processed_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT (datetime('now'))
); 