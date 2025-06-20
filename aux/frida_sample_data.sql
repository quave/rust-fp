-- SQL for populating the Frida AI database with test transactions
-- Sample data for Frida AI fraud detection system
-- This script populates the database with sample transactions for UI testing

-- First, clear existing data to avoid conflicts
DO $$
BEGIN
  -- Disable foreign key checks temporarily to allow truncating related tables
  SET CONSTRAINTS ALL DEFERRED;
  
  -- Clear existing data in reverse dependency order
  TRUNCATE order_items, customers, billing_data, orders, transactions, labels CASCADE;
  ALTER SEQUENCE labels_id_seq RESTART WITH 1;
  ALTER SEQUENCE transactions_id_seq RESTART WITH 1;
  ALTER SEQUENCE orders_id_seq RESTART WITH 1;
  
  -- Re-enable foreign key checks
  SET CONSTRAINTS ALL IMMEDIATE;
END$$;

-- First, let's add some sample fraud labels
INSERT INTO labels (fraud_level, fraud_category, label_source, labeled_by, created_at)
VALUES 
  ('Fraud', 'Payment Fraud', 'Manual', 'Admin', NOW() - INTERVAL '10 days'),
  ('NoFraud', 'Legitimate Transaction', 'Manual', 'Admin', NOW() - INTERVAL '9 days'),
  ('BlockedAutomatically', 'Identity Theft', 'Api', 'System', NOW() - INTERVAL '8 days'),
  ('AccountTakeover', 'Account Takeover', 'Manual', 'Analyst', NOW() - INTERVAL '7 days'),
  ('NotCreditWorthy', 'Chargeback', 'Manual', 'Compliance', NOW() - INTERVAL '6 days');

-- Let's now insert the base transactions with proper label IDs
INSERT INTO transactions (label_id, comment, processing_complete, created_at)
VALUES 
  (1, 'Suspicious IP address', TRUE, NOW() - INTERVAL '10 days'),
  (2, 'Customer verified by phone', TRUE, NOW() - INTERVAL '9 days'),
  (3, 'Automatic block by risk system', TRUE, NOW() - INTERVAL '8 days'),
  (4, 'Multiple login attempts from different locations', TRUE, NOW() - INTERVAL '7 days'),
  (5, 'Previous chargebacks on record', TRUE, NOW() - INTERVAL '6 days'),
  (NULL, NULL, TRUE, NOW() - INTERVAL '5 days'),
  (NULL, NULL, TRUE, NOW() - INTERVAL '4 days'),
  (NULL, NULL, TRUE, NOW() - INTERVAL '3 days'),
  (NULL, NULL, TRUE, NOW() - INTERVAL '2 days'),
  (NULL, NULL, TRUE, NOW() - INTERVAL '1 day');

-- Get the actual transaction IDs for use in orders
DO $$
DECLARE
    tx_ids INT[] := ARRAY[]::INT[];
    order_ids INT[] := ARRAY[]::INT[];
    tx_id INT;
    ord_id INT;
    i INT := 1;
BEGIN
    -- Get transaction IDs one by one in order of creation
    FOR tx_id IN SELECT id FROM transactions ORDER BY created_at LIMIT 10
    LOOP
        tx_ids[i] := tx_id;
        i := i + 1;
    END LOOP;
    
    -- Create orders linked to transactions with the actual IDs
    -- Order 1
    INSERT INTO orders (transaction_id, order_number, delivery_type, delivery_details, created_at)
    VALUES (tx_ids[1], 'ORD-2024-001', 'express', 'Send to home address', NOW() - INTERVAL '10 days')
    RETURNING id INTO ord_id;
    order_ids[1] := ord_id;
    
    -- Order 2
    INSERT INTO orders (transaction_id, order_number, delivery_type, delivery_details, created_at)
    VALUES (tx_ids[2], 'ORD-2024-002', 'standard', 'Send to home address', NOW() - INTERVAL '9 days')
    RETURNING id INTO ord_id;
    order_ids[2] := ord_id;
    
    -- Order 3
    INSERT INTO orders (transaction_id, order_number, delivery_type, delivery_details, created_at)
    VALUES (tx_ids[3], 'ORD-2024-003', 'pickup', 'Store pickup', NOW() - INTERVAL '8 days')
    RETURNING id INTO ord_id;
    order_ids[3] := ord_id;
    
    -- Order 4
    INSERT INTO orders (transaction_id, order_number, delivery_type, delivery_details, created_at)
    VALUES (tx_ids[4], 'ORD-2024-004', 'express', 'Send to different address', NOW() - INTERVAL '7 days')
    RETURNING id INTO ord_id;
    order_ids[4] := ord_id;
    
    -- Order 5
    INSERT INTO orders (transaction_id, order_number, delivery_type, delivery_details, created_at)
    VALUES (tx_ids[5], 'ORD-2024-005', 'standard', 'Send to home address', NOW() - INTERVAL '6 days')
    RETURNING id INTO ord_id;
    order_ids[5] := ord_id;
    
    -- Order 6
    INSERT INTO orders (transaction_id, order_number, delivery_type, delivery_details, created_at)
    VALUES (tx_ids[6], 'ORD-2024-006', 'express', 'Send to home address', NOW() - INTERVAL '5 days')
    RETURNING id INTO ord_id;
    order_ids[6] := ord_id;
    
    -- Order 7
    INSERT INTO orders (transaction_id, order_number, delivery_type, delivery_details, created_at)
    VALUES (tx_ids[7], 'ORD-2024-007', 'standard', 'Send to home address', NOW() - INTERVAL '4 days')
    RETURNING id INTO ord_id;
    order_ids[7] := ord_id;
    
    -- Order 8
    INSERT INTO orders (transaction_id, order_number, delivery_type, delivery_details, created_at)
    VALUES (tx_ids[8], 'ORD-2024-008', 'pickup', 'Store pickup', NOW() - INTERVAL '3 days')
    RETURNING id INTO ord_id;
    order_ids[8] := ord_id;
    
    -- Order 9
    INSERT INTO orders (transaction_id, order_number, delivery_type, delivery_details, created_at)
    VALUES (tx_ids[9], 'ORD-2024-009', 'express', 'Send to home address', NOW() - INTERVAL '2 days')
    RETURNING id INTO ord_id;
    order_ids[9] := ord_id;
    
    -- Order 10
    INSERT INTO orders (transaction_id, order_number, delivery_type, delivery_details, created_at)
    VALUES (tx_ids[10], 'ORD-2024-010', 'standard', 'Send to home address', NOW() - INTERVAL '1 day')
    RETURNING id INTO ord_id;
    order_ids[10] := ord_id;

    -- Continue with the rest of your script using order_ids[1], order_ids[2], etc.
    -- Add customer data for each order...
    
    -- The rest of your code remains the same, just using the proper order_ids array
    INSERT INTO customers (order_id, name, email, created_at)
    VALUES 
      (order_ids[1], 'John Doe', 'john.doe@example.com', NOW() - INTERVAL '10 days'),
      (order_ids[2], 'Jane Smith', 'jane.smith@example.com', NOW() - INTERVAL '9 days'),
      (order_ids[3], 'Alex Johnson', 'alex.johnson@example.com', NOW() - INTERVAL '8 days'),
      (order_ids[4], 'Maria Garcia', 'maria.garcia@example.com', NOW() - INTERVAL '7 days'),
      (order_ids[5], 'Robert Brown', 'robert.brown@example.com', NOW() - INTERVAL '6 days'),
      (order_ids[6], 'Sarah Wilson', 'sarah.wilson@example.com', NOW() - INTERVAL '5 days'),
      (order_ids[7], 'Michael Davis', 'michael.davis@example.com', NOW() - INTERVAL '4 days'),
      (order_ids[8], 'Emily Martinez', 'emily.martinez@example.com', NOW() - INTERVAL '3 days'),
      (order_ids[9], 'David Anderson', 'david.anderson@example.com', NOW() - INTERVAL '2 days'),
      (order_ids[10], 'Jennifer Thomas', 'jennifer.thomas@example.com', NOW() - INTERVAL '1 day');

    -- Add billing data for each order
    INSERT INTO billing_data (order_id, payment_type, payment_details, billing_address, created_at)
    VALUES 
      (order_ids[1], 'credit_card', '4111111111111111', '123 Main St, New York, NY 10001', NOW() - INTERVAL '10 days'),
      (order_ids[2], 'paypal', 'jane.smith@example.com', '456 Oak Ave, Los Angeles, CA 90001', NOW() - INTERVAL '9 days'),
      (order_ids[3], 'credit_card', '5555555555554444', '789 Pine St, Chicago, IL 60007', NOW() - INTERVAL '8 days'),
      (order_ids[4], 'credit_card', '378282246310005', '321 Cedar Rd, Miami, FL 33101', NOW() - INTERVAL '7 days'),
      (order_ids[5], 'bank_transfer', 'GB29NWBK60161331926819', '654 Birch Ln, Seattle, WA 98101', NOW() - INTERVAL '6 days'),
      (order_ids[6], 'credit_card', '6011111111111117', '987 Elm St, Austin, TX 78701', NOW() - INTERVAL '5 days'),
      (order_ids[7], 'paypal', 'michael.davis@example.com', '147 Maple Dr, Denver, CO 80201', NOW() - INTERVAL '4 days'),
      (order_ids[8], 'credit_card', '3530111333300000', '258 Willow Ave, Portland, OR 97201', NOW() - INTERVAL '3 days'),
      (order_ids[9], 'bank_transfer', 'FR1420041010050500013M02606', '369 Ash Blvd, Boston, MA 02101', NOW() - INTERVAL '2 days'),
      (order_ids[10], 'credit_card', '4012888888881881', '741 Spruce St, San Francisco, CA 94101', NOW() - INTERVAL '1 day');

    -- Add order items for each order
    -- Order 1: Fraudulent high-value electronics order
    INSERT INTO order_items (order_id, name, category, price, created_at)
    VALUES
      (order_ids[1], 'iPhone 15 Pro', 'Electronics', 1299.99, NOW() - INTERVAL '10 days'),
      (order_ids[1], 'MacBook Pro 16"', 'Electronics', 2499.99, NOW() - INTERVAL '10 days'),
      (order_ids[1], 'Apple Watch Ultra', 'Electronics', 799.99, NOW() - INTERVAL '10 days');

    -- Order 2: Legitimate household items
    INSERT INTO order_items (order_id, name, category, price, created_at)
    VALUES
      (order_ids[2], 'Bed Sheets (Queen)', 'Home Goods', 49.99, NOW() - INTERVAL '9 days'),
      (order_ids[2], 'Towel Set', 'Home Goods', 39.99, NOW() - INTERVAL '9 days'),
      (order_ids[2], 'Shower Curtain', 'Home Goods', 24.99, NOW() - INTERVAL '9 days');

    -- Order 3: Blocked automatically - gift cards (suspicious pattern)
    INSERT INTO order_items (order_id, name, category, price, created_at)
    VALUES
      (order_ids[3], 'Amazon Gift Card', 'Gift Cards', 500.00, NOW() - INTERVAL '8 days'),
      (order_ids[3], 'Steam Gift Card', 'Gift Cards', 200.00, NOW() - INTERVAL '8 days'),
      (order_ids[3], 'Apple Gift Card', 'Gift Cards', 300.00, NOW() - INTERVAL '8 days');

    -- Order 4: Account takeover - unusual purchases
    INSERT INTO order_items (order_id, name, category, price, created_at)
    VALUES
      (order_ids[4], 'Designer Handbag', 'Luxury Goods', 1899.99, NOW() - INTERVAL '7 days'),
      (order_ids[4], 'Gold Necklace', 'Jewelry', 1250.00, NOW() - INTERVAL '7 days');

    -- Order 5: Not credit worthy - expensive furniture
    INSERT INTO order_items (order_id, name, category, price, created_at)
    VALUES
      (order_ids[5], 'Leather Sofa', 'Furniture', 2499.99, NOW() - INTERVAL '6 days'),
      (order_ids[5], 'Coffee Table', 'Furniture', 399.99, NOW() - INTERVAL '6 days'),
      (order_ids[5], 'Dining Set', 'Furniture', 1299.99, NOW() - INTERVAL '6 days');

    -- Order 6: Unlabeled - exercise equipment
    INSERT INTO order_items (order_id, name, category, price, created_at)
    VALUES
      (order_ids[6], 'Treadmill', 'Fitness', 999.99, NOW() - INTERVAL '5 days'),
      (order_ids[6], 'Dumbbells Set', 'Fitness', 249.99, NOW() - INTERVAL '5 days'),
      (order_ids[6], 'Yoga Mat', 'Fitness', 29.99, NOW() - INTERVAL '5 days');

    -- Order 7: Unlabeled - clothing
    INSERT INTO order_items (order_id, name, category, price, created_at)
    VALUES
      (order_ids[7], 'Winter Jacket', 'Clothing', 199.99, NOW() - INTERVAL '4 days'),
      (order_ids[7], 'Jeans', 'Clothing', 79.99, NOW() - INTERVAL '4 days'),
      (order_ids[7], 'T-Shirts (3-pack)', 'Clothing', 39.99, NOW() - INTERVAL '4 days');

    -- Order 8: Unlabeled - pet supplies
    INSERT INTO order_items (order_id, name, category, price, created_at)
    VALUES
      (order_ids[8], 'Dog Food (50lb)', 'Pet Supplies', 89.99, NOW() - INTERVAL '3 days'),
      (order_ids[8], 'Pet Bed', 'Pet Supplies', 59.99, NOW() - INTERVAL '3 days'),
      (order_ids[8], 'Dog Toys Bundle', 'Pet Supplies', 34.99, NOW() - INTERVAL '3 days');

    -- Order 9: Unlabeled - high-end computer components
    INSERT INTO order_items (order_id, name, category, price, created_at)
    VALUES
      (order_ids[9], 'NVIDIA RTX 4090', 'Computer Components', 1999.99, NOW() - INTERVAL '2 days'),
      (order_ids[9], 'Intel i9 Processor', 'Computer Components', 699.99, NOW() - INTERVAL '2 days'),
      (order_ids[9], '32GB RAM Kit', 'Computer Components', 249.99, NOW() - INTERVAL '2 days');

    -- Order 10: Unlabeled - kitchen appliances
    INSERT INTO order_items (order_id, name, category, price, created_at)
    VALUES
      (order_ids[10], 'Stand Mixer', 'Kitchen Appliances', 349.99, NOW() - INTERVAL '1 day'),
      (order_ids[10], 'Espresso Machine', 'Kitchen Appliances', 499.99, NOW() - INTERVAL '1 day'),
      (order_ids[10], 'Blender', 'Kitchen Appliances', 129.99, NOW() - INTERVAL '1 day');

    -- Set order status for the UI
    UPDATE orders
    SET delivery_type = CASE
      WHEN transaction_id = tx_ids[1] OR transaction_id = tx_ids[3] OR transaction_id = tx_ids[4] THEN 'cancelled'
      WHEN transaction_id = tx_ids[2] OR transaction_id = tx_ids[5] THEN 'completed'
      ELSE 'pending'
    END;
END$$;

