-- Add migration script here
DROP TABLE IF EXISTS order_items;
DROP TABLE IF EXISTS customers;
DROP TABLE IF EXISTS billing_data;


CREATE TABLE IF NOT EXISTS orders (
    id BIGSERIAL PRIMARY KEY,
    transaction_id BIGINT NOT NULL,
    order_number TEXT,
    delivery_type TEXT NOT NULL,
    delivery_details TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    FOREIGN KEY (transaction_id) REFERENCES transactions(id)
);
CREATE INDEX IF NOT EXISTS idx_orders_created_at ON orders(created_at);
CREATE INDEX IF NOT EXISTS idx_orders_transaction_id ON orders(transaction_id);

CREATE TABLE IF NOT EXISTS order_items (
    id BIGSERIAL PRIMARY KEY,
    order_id BIGINT NOT NULL,
    name TEXT NOT NULL,
    category TEXT NOT NULL,
    price REAL NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    FOREIGN KEY (order_id) REFERENCES orders(id)
);
CREATE INDEX IF NOT EXISTS idx_order_items_order_id ON order_items(order_id);
CREATE INDEX IF NOT EXISTS idx_order_items_created_at ON order_items(created_at);

CREATE TABLE IF NOT EXISTS customers (
    id BIGSERIAL PRIMARY KEY,
    order_id BIGINT NOT NULL,
    name TEXT NOT NULL,
    email TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    FOREIGN KEY (order_id) REFERENCES orders(id)
);
CREATE INDEX IF NOT EXISTS idx_customers_order_id ON customers(order_id);
CREATE INDEX IF NOT EXISTS idx_customers_created_at ON customers(created_at);

CREATE TABLE IF NOT EXISTS billing_data (
    id BIGSERIAL PRIMARY KEY,
    order_id BIGINT NOT NULL,
    payment_type TEXT NOT NULL,
    payment_details TEXT NOT NULL,
    billing_address TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    FOREIGN KEY (order_id) REFERENCES orders(id)
);
CREATE INDEX IF NOT EXISTS idx_billing_data_order_id ON billing_data(order_id);
CREATE INDEX IF NOT EXISTS idx_billing_data_created_at ON billing_data(created_at);

