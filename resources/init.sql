-- init.sql
DROP TABLE IF EXISTS order_items;
DROP TABLE IF EXISTS customers;
DROP TABLE IF EXISTS billing_data;
DROP TABLE IF EXISTS features;
DROP TABLE IF EXISTS triggered_rules;


CREATE TABLE IF NOT EXISTS orders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_number TEXT,
    delivery_type TEXT NOT NULL,
    delivery_details TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS order_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    category TEXT NOT NULL,
    price REAL NOT NULL,
    created_at DATETIME NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (order_id) REFERENCES orders(id)
);

CREATE TABLE IF NOT EXISTS customers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    email TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (order_id) REFERENCES orders(id)
);

CREATE TABLE IF NOT EXISTS billing_data (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_id INTEGER NOT NULL,
    payment_type TEXT NOT NULL,
    payment_details TEXT NOT NULL,
    billing_address TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (order_id) REFERENCES orders(id)
);

CREATE TABLE IF NOT EXISTS triggered_rules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_id INTEGER NOT NULL,
    rule_name TEXT NOT NULL,
    rule_score INTEGER NOT NULL,
    created_at DATETIME NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (order_id) REFERENCES orders(id)
);

CREATE TABLE IF NOT EXISTS features (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_id INTEGER NOT NULL,
    feature_name TEXT NOT NULL,
    feature_value_type TEXT NOT NULL,
    feature_value TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (order_id) REFERENCES orders(id)
);

-- Indexes for better query performance
CREATE INDEX IF NOT EXISTS idx_order_items_order_id ON order_items(order_id);
CREATE INDEX IF NOT EXISTS idx_customers_order_id ON customers(order_id);
CREATE INDEX IF NOT EXISTS idx_billing_data_order_id ON billing_data(order_id);
CREATE INDEX IF NOT EXISTS idx_features_order_id ON features(order_id);
CREATE INDEX IF NOT EXISTS idx_triggered_rules_order_id ON triggered_rules(order_id);
