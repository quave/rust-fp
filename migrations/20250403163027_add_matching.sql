-- Enable PostGIS for geographic distance calculations
CREATE EXTENSION IF NOT EXISTS postgis;

DROP TABLE IF EXISTS match_node_transactions;
DROP TABLE IF EXISTS match_node;

CREATE TABLE IF NOT EXISTS match_node (
    id BIGSERIAL PRIMARY KEY,
    matcher TEXT NOT NULL,
    value TEXT NOT NULL,
    confidence INTEGER NOT NULL,
    importance INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS match_node_transactions (
    node_id BIGINT NOT NULL,
    transaction_id BIGINT NOT NULL,
    datetime_alpha TIMESTAMP NULL,
    datetime_beta TIMESTAMP NULL,
    long_alpha DOUBLE PRECISION NULL,
    lat_alpha DOUBLE PRECISION NULL,
    long_beta DOUBLE PRECISION NULL,
    lat_beta DOUBLE PRECISION NULL,
    long_gamma DOUBLE PRECISION NULL,
    lat_gamma DOUBLE PRECISION NULL,
    long_delta DOUBLE PRECISION NULL,
    lat_delta DOUBLE PRECISION NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    PRIMARY KEY (node_id, transaction_id),
    CONSTRAINT fk_match_node_transactions_node FOREIGN KEY (node_id) REFERENCES match_node(id),
    CONSTRAINT fk_match_node_transactions_transaction FOREIGN KEY (transaction_id) REFERENCES transactions(id)
);

-- Add indexes for faster lookups
CREATE UNIQUE INDEX idx_match_node_matcher_value ON match_node(matcher, value);
CREATE INDEX idx_match_node_transactions_transaction_id ON match_node_transactions(transaction_id);
CREATE INDEX idx_match_node_transactions_node_id ON match_node_transactions(node_id);


DROP FUNCTION if exists find_connected_transactions;
DROP FUNCTION if exists find_connected_transactions_impl;
DROP FUNCTION if exists test_find_connected_transactions;
DROP FUNCTION if exists init_transaction_matching;
DROP FUNCTION if exists find_transaction_connections;
DROP FUNCTION if exists apply_transaction_filters;
DROP FUNCTION if exists init_with_starting_transaction;

-- Helper function to initialize transaction matching tables
CREATE OR REPLACE FUNCTION init_transaction_matching() 
RETURNS TABLE (
    process_queue_table TEXT,
    next_batch_table TEXT,
    result_set_table TEXT,
    all_processed_table TEXT
) AS $$
BEGIN
    -- Create temporary tables with indexes
    CREATE TEMP TABLE IF NOT EXISTS process_queue (
        id BIGINT PRIMARY KEY,
        created_at TIMESTAMP,
        path_matchers TEXT[],
        path_values TEXT[],
        confidence INTEGER,
        importance INTEGER,
        depth INTEGER
    ) ON COMMIT DROP;
    
    CREATE TEMP TABLE IF NOT EXISTS next_batch (
        id BIGINT PRIMARY KEY,
        created_at TIMESTAMP,
        path_matchers TEXT[],
        path_values TEXT[],
        confidence INTEGER,
        importance INTEGER,
        depth INTEGER
    ) ON COMMIT DROP;
    
    CREATE TEMP TABLE IF NOT EXISTS result_set (
        id BIGINT PRIMARY KEY,
        created_at TIMESTAMP,
        path_matchers TEXT[],
        path_values TEXT[],
        confidence INTEGER,
        importance INTEGER,
        depth INTEGER
    ) ON COMMIT DROP;
    
    -- Table to track all processed IDs to avoid cycles
    CREATE TEMP TABLE IF NOT EXISTS all_processed (
        id BIGINT PRIMARY KEY
    ) ON COMMIT DROP;
    
    -- Clear temporary tables
    TRUNCATE process_queue;
    TRUNCATE next_batch;
    TRUNCATE result_set;
    TRUNCATE all_processed;
    
    -- Return table names
    RETURN QUERY SELECT 
        'process_queue'::TEXT, 
        'next_batch'::TEXT, 
        'result_set'::TEXT, 
        'all_processed'::TEXT;
END;
$$ LANGUAGE plpgsql;

-- Helper function to find connections from current transaction batch
CREATE OR REPLACE FUNCTION find_transaction_connections(
    min_confidence INTEGER,
    root_transaction_id BIGINT,
    root_payload_number TEXT,
    filter_config JSONB DEFAULT NULL
) RETURNS VOID AS $$
BEGIN
    -- Find all connections from process_queue
    INSERT INTO next_batch (id, created_at, path_matchers, path_values, confidence, importance, depth)
    SELECT
        t.id,
        t.created_at,
        pq.path_matchers || ARRAY[mn.matcher],
        pq.path_values || ARRAY[mn.value],
        mn.confidence,
        mn.importance,
        pq.depth + 1
    FROM process_queue pq
    JOIN match_node_transactions mnt1 ON mnt1.transaction_id = pq.id
    LEFT JOIN match_node_transactions root_mnt 
        ON root_mnt.node_id = mnt1.node_id AND root_mnt.transaction_id = root_transaction_id
    JOIN match_node_transactions mnt ON mnt.node_id = mnt1.node_id
    JOIN match_node mn ON mn.id = mnt1.node_id
    JOIN transactions t ON t.id = mnt.transaction_id
    WHERE 
        t.id != pq.id
        AND t.payload_number != root_payload_number
        AND NOT EXISTS (SELECT 1 FROM all_processed ap WHERE ap.id = t.id)
        AND mn.confidence >= min_confidence
        AND (
            filter_config IS NULL OR NOT (filter_config ? mn.matcher) OR (
                -- timestamp_alpha in days threshold
                (NOT (filter_config->mn.matcher ? 'timestamp_alpha') OR
                    root_mnt.datetime_alpha IS NULL OR mnt.datetime_alpha IS NULL OR
                    abs(extract(epoch from (mnt.datetime_alpha - root_mnt.datetime_alpha)))/86400.0 <=
                        (filter_config->mn.matcher->>'timestamp_alpha')::numeric
                )
                AND
                (NOT (filter_config->mn.matcher ? 'timestamp_beta') OR
                    root_mnt.datetime_beta IS NULL OR mnt.datetime_beta IS NULL OR
                    abs(extract(epoch from (mnt.datetime_beta - root_mnt.datetime_beta)))/86400.0 <=
                        (filter_config->mn.matcher->>'timestamp_beta')::numeric
                )
                AND
                (NOT (filter_config->mn.matcher ? 'location_alpha') OR
                    root_mnt.long_alpha IS NULL OR root_mnt.lat_alpha IS NULL OR
                    mnt.long_alpha IS NULL OR mnt.lat_alpha IS NULL OR
                    ST_Distance(
                        ST_SetSRID(ST_MakePoint(root_mnt.long_alpha, root_mnt.lat_alpha), 4326)::geography,
                        ST_SetSRID(ST_MakePoint(mnt.long_alpha, mnt.lat_alpha), 4326)::geography
                    ) <= (filter_config->mn.matcher->>'location_alpha')::numeric
                )
                AND
                (NOT (filter_config->mn.matcher ? 'location_beta') OR
                    root_mnt.long_beta IS NULL OR root_mnt.lat_beta IS NULL OR
                    mnt.long_beta IS NULL OR mnt.lat_beta IS NULL OR
                    ST_Distance(
                        ST_SetSRID(ST_MakePoint(root_mnt.long_beta, root_mnt.lat_beta), 4326)::geography,
                        ST_SetSRID(ST_MakePoint(mnt.long_beta, mnt.lat_beta), 4326)::geography
                    ) <= (filter_config->mn.matcher->>'location_beta')::numeric
                )
                AND
                (NOT (filter_config->mn.matcher ? 'location_gamma') OR
                    root_mnt.long_gamma IS NULL OR root_mnt.lat_gamma IS NULL OR
                    mnt.long_gamma IS NULL OR mnt.lat_gamma IS NULL OR
                    ST_Distance(
                        ST_SetSRID(ST_MakePoint(root_mnt.long_gamma, root_mnt.lat_gamma), 4326)::geography,
                        ST_SetSRID(ST_MakePoint(mnt.long_gamma, mnt.lat_gamma), 4326)::geography
                    ) <= (filter_config->mn.matcher->>'location_gamma')::numeric
                )
                AND
                (NOT (filter_config->mn.matcher ? 'location_delta') OR
                    root_mnt.long_delta IS NULL OR root_mnt.lat_delta IS NULL OR
                    mnt.long_delta IS NULL OR mnt.lat_delta IS NULL OR
                    ST_Distance(
                        ST_SetSRID(ST_MakePoint(root_mnt.long_delta, root_mnt.lat_delta), 4326)::geography,
                        ST_SetSRID(ST_MakePoint(mnt.long_delta, mnt.lat_delta), 4326)::geography
                    ) <= (filter_config->mn.matcher->>'location_delta')::numeric
                )
            )
        )
    ON CONFLICT (id) DO NOTHING;
    
    -- Mark all as processed to avoid cycles
    INSERT INTO all_processed
    SELECT nb.id FROM next_batch nb
    ON CONFLICT (id) DO NOTHING;
END;
$$ LANGUAGE plpgsql;

-- Helper function to apply filters to transaction results
CREATE OR REPLACE FUNCTION apply_transaction_filters(
    limit_count INTEGER
) RETURNS VOID AS $$
DECLARE
    remaining_count INTEGER;
BEGIN
    -- Add transactions that meet filter criteria to results (with deterministic ordering and proper limiting)
    IF limit_count IS NULL THEN
        INSERT INTO result_set (id, created_at, path_matchers, path_values, confidence, importance, depth)
        SELECT 
            nb.id, nb.created_at, nb.path_matchers, nb.path_values, nb.confidence, nb.importance, nb.depth
        FROM next_batch nb
        ORDER BY 
            nb.confidence DESC,
            nb.importance DESC,
            nb.depth,
            nb.id;
    ELSE
        remaining_count := GREATEST(limit_count - (SELECT COUNT(*) FROM result_set), 0);
        IF remaining_count > 0 THEN
            INSERT INTO result_set (id, created_at, path_matchers, path_values, confidence, importance, depth)
            SELECT 
                nb.id, nb.created_at, nb.path_matchers, nb.path_values, nb.confidence, nb.importance, nb.depth
            FROM next_batch nb
            ORDER BY 
                nb.confidence DESC,
                nb.importance DESC,
                nb.depth,
                nb.id
            LIMIT remaining_count;
        END IF;
    END IF;
        
    -- Move qualifying transactions to next level for traversal
    TRUNCATE process_queue;
    INSERT INTO process_queue (id, created_at, path_matchers, path_values, confidence, importance, depth)
    SELECT nb.id, nb.created_at, nb.path_matchers, nb.path_values, nb.confidence, nb.importance, nb.depth
    FROM next_batch nb;
END;
$$ LANGUAGE plpgsql;

-- Helper function to initialize with the starting transaction
CREATE OR REPLACE FUNCTION init_with_starting_transaction(
    input_transaction_id BIGINT
) RETURNS VOID AS $$
BEGIN
    -- Initialize with start transaction
    INSERT INTO process_queue (id, created_at, path_matchers, path_values, confidence, importance, depth)
    SELECT 
        t.id, 
        t.created_at,
        ARRAY[mn.matcher],
        ARRAY[mn.value],
        mn.confidence,
        mn.importance,
        0
    FROM transactions t
    JOIN match_node_transactions mnt ON t.id = mnt.transaction_id
    JOIN match_node mn ON mnt.node_id = mn.id
    WHERE t.id = input_transaction_id
    LIMIT 1;
    
    -- Add the starting transaction to results
    INSERT INTO result_set (id, created_at, path_matchers, path_values, confidence, importance, depth)
    SELECT * FROM process_queue;
    
    -- Mark as processed
    INSERT INTO all_processed VALUES (input_transaction_id);
END;
$$ LANGUAGE plpgsql;

-- Main function to find connected transactions
-- Traverses transaction connections based on match_node links
-- Uses breadth-first search to find connections efficiently
-- Supports filtering by confidence threshold, max depth, result limit, and filter_config JSON thresholds
CREATE OR REPLACE FUNCTION find_connected_transactions(
    input_transaction_id BIGINT,
    max_depth INTEGER DEFAULT NULL,
    limit_count INTEGER DEFAULT NULL,
    filter_config JSONB DEFAULT NULL,
    min_confidence INTEGER DEFAULT 0,
    min_connections INTEGER DEFAULT 1 -- Kept for backward compatibility but not used internally
) RETURNS TABLE (
    transaction_id BIGINT,
    path_matchers TEXT[],
    path_values TEXT[],
    depth INTEGER,
    confidence INTEGER,
    importance INTEGER,
    created_at TIMESTAMP
) AS $$
DECLARE
    max_traverse_depth INTEGER;
    root_payload_number TEXT;
BEGIN
    -- Initialize max traverse depth - ensure we don't process beyond our limit
    max_traverse_depth := CASE WHEN max_depth IS NULL THEN NULL ELSE max_depth - 1 END;
    
    -- Determine the root transaction payload to avoid connecting to other versions of the same payload
    SELECT payload_number INTO root_payload_number FROM transactions WHERE id = input_transaction_id;
    IF root_payload_number IS NULL THEN
        RAISE EXCEPTION 'Transaction % not found for matching', input_transaction_id;
    END IF;
    
    -- Initialize all the temporary tables
    PERFORM init_transaction_matching();
    
    -- Initialize with the starting transaction
    PERFORM init_with_starting_transaction(input_transaction_id);
    
    -- Process transactions level by level (breadth-first)
    WHILE EXISTS (SELECT 1 FROM process_queue) 
    AND (max_traverse_depth IS NULL OR (SELECT MAX(pq.depth) FROM process_queue pq) < max_traverse_depth) 
    AND (limit_count IS NULL OR (SELECT COUNT(*) FROM result_set) < limit_count) LOOP
        -- Clear next batch
        TRUNCATE next_batch;
        
        -- Find connections from current batch
        PERFORM find_transaction_connections(min_confidence, input_transaction_id, root_payload_number, filter_config);
        
        -- Apply filters to results
        PERFORM apply_transaction_filters(limit_count);
        
        -- Exit if no more transactions to process or limit reached
        IF NOT EXISTS (SELECT 1 FROM process_queue) OR 
           (limit_count IS NOT NULL AND (SELECT COUNT(*) FROM result_set) >= limit_count) THEN
            EXIT;
        END IF;
    END LOOP;
    
    -- Return results with ordering
    RETURN QUERY
    WITH filtered_results AS (
        SELECT 
            r.id,
            r.path_matchers,
            r.path_values,
            r.depth,
            r.confidence,
            r.importance,
            r.created_at
        FROM result_set r
        WHERE 
            -- Apply min_confidence filter
            (min_confidence = 0 OR r.confidence >= min_confidence)
            -- Apply max_depth filter
            AND (max_depth IS NULL OR r.depth < max_depth)
    )
    SELECT 
        fr.id as transaction_id,
        fr.path_matchers,
        fr.path_values,
        fr.depth,
        fr.confidence,
        fr.importance,
        fr.created_at
    FROM filtered_results fr
    ORDER BY 
        -- Sort by confidence (higher first)
        fr.confidence DESC,
        -- Then by importance
        fr.importance DESC,
        -- Then by depth (shallower nodes first)
        fr.depth,
        -- Then by transaction ID for consistency
        fr.id
    -- Apply the provided limit or no limit if NULL
    LIMIT CASE WHEN limit_count IS NULL THEN 2147483647 ELSE limit_count END;
    
    -- Clean up
    DROP TABLE IF EXISTS process_queue;
    DROP TABLE IF EXISTS next_batch;
    DROP TABLE IF EXISTS result_set;
    DROP TABLE IF EXISTS all_processed;
    
    RETURN;
END;
$$ LANGUAGE plpgsql;

-- Test Function for find_connected_transactions
CREATE OR REPLACE FUNCTION test_find_connected_transactions() RETURNS TABLE (
    case_number INT,
    description TEXT,
    expected INT,
    actual INT,
    pass_fail BOOLEAN
) AS $$
DECLARE
    result_count INTEGER;
    expected_count INTEGER;
    result_set BIGINT[];
    expected_set BIGINT[];
BEGIN
    create temporary table test_results (
        case_number INT,
        description TEXT,
        expected INT,
        actual INT,
        pass_fail BOOLEAN
    ) on commit drop;

    -- Clear all existing data
    TRUNCATE TABLE match_node_transactions CASCADE;
    TRUNCATE TABLE match_node CASCADE;
    TRUNCATE TABLE transactions CASCADE;

    -- Test Case 1: 10 transactions, all connected through customer.email
    RAISE NOTICE 'Setting up Test Case 1: 10 transactions, all connected through customer.email';
    INSERT INTO transactions (id, payload_number, payload, created_at) VALUES 
    (1, 'TEST1', '{}', '2024-01-01'),
    (2, 'TEST2', '{}', '2024-01-02'),
    (3, 'TEST3', '{}', '2024-01-03'),
    (4, 'TEST4', '{}', '2024-01-04'),
    (5, 'TEST5', '{}', '2024-01-05'),
    (6, 'TEST6', '{}', '2024-01-06'),
    (7, 'TEST7', '{}', '2024-01-07'),
    (8, 'TEST8', '{}', '2024-01-08'),
    (9, 'TEST9', '{}', '2024-01-09'),
    (10, 'TEST10', '{}', '2024-01-10');

    INSERT INTO match_node (id, matcher, value, confidence, importance) VALUES
    (1, 'customer.email', 'test@test.com', 100, 0);

    -- Insert connections one by one to avoid conflicts
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (1, 1);
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (1, 2);
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (1, 3);
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (1, 4);
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (1, 5);
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (1, 6);
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (1, 7);
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (1, 8);
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (1, 9);
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (1, 10);

    -- Test Case 1: All connected transactions (testing basic connection finding)
    RAISE NOTICE 'Running Test Case 1: All connected transactions';
    result_count := (SELECT COUNT(*) FROM find_connected_transactions(1));
    expected_count := 10;
    insert into test_results VALUES (1, 'All transactions connected through email', expected_count, result_count, result_count = expected_count);

    -- Test Case 2: 10 transactions, some not connected
    RAISE NOTICE 'Setting up Test Case 2: 10 transactions, some not connected';
    TRUNCATE TABLE match_node_transactions CASCADE;
    TRUNCATE TABLE match_node CASCADE;
    TRUNCATE TABLE transactions CASCADE;
    
    INSERT INTO transactions (id, payload_number, payload, created_at) VALUES 
    (1, 'TEST1', '{}', '2024-01-01'),
    (2, 'TEST2', '{}', '2024-01-02'),
    (3, 'TEST3', '{}', '2024-01-03'),
    (4, 'TEST4', '{}', '2024-01-04'),
    (5, 'TEST5', '{}', '2024-01-05'),
    (6, 'TEST6', '{}', '2024-01-06'),
    (7, 'TEST7', '{}', '2024-01-07'),
    (8, 'TEST8', '{}', '2024-01-08'),
    (9, 'TEST9', '{}', '2024-01-09'),
    (10, 'TEST10', '{}', '2024-01-10');

    INSERT INTO match_node (id, matcher, value, confidence, importance) VALUES
    (1, 'customer.email', 'group1@test.com', 100, 0),
    (2, 'customer.email', 'group2@test.com', 100, 0);

    -- Insert Group 1 connections
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (1, 1);
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (1, 2);
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (1, 3);
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (1, 4);
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (1, 5);
    
    -- Insert Group 2 connections
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (2, 6);
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (2, 7);
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (2, 8);
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (2, 9);
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (2, 10);

    -- Test Case 2: Verify the function only finds connected transactions (not from unconnected groups)
    RAISE NOTICE 'Running Test Case 2: Some connected transactions';
    result_count := (SELECT COUNT(*) FROM find_connected_transactions(1));
    expected_count := 5; -- Should only see transactions from group 1
    insert into test_results VALUES (2, 'Only finds transactions from connected group', expected_count, result_count, result_count = expected_count);

    -- Test Case 3: 10 transactions in a chain (testing deep traversal)
    RAISE NOTICE 'Setting up Test Case 3: 10 transactions in a chain';
    TRUNCATE TABLE match_node_transactions CASCADE;
    TRUNCATE TABLE match_node CASCADE;
    TRUNCATE TABLE transactions CASCADE;
    
    INSERT INTO transactions (id, payload_number, payload, created_at) VALUES 
    (1, 'TEST1', '{}', '2024-01-01'),
    (2, 'TEST2', '{}', '2024-01-02'),
    (3, 'TEST3', '{}', '2024-01-03'),
    (4, 'TEST4', '{}', '2024-01-04'),
    (5, 'TEST5', '{}', '2024-01-05'),
    (6, 'TEST6', '{}', '2024-01-06'),
    (7, 'TEST7', '{}', '2024-01-07'),
    (8, 'TEST8', '{}', '2024-01-08'),
    (9, 'TEST9', '{}', '2024-01-09'),
    (10, 'TEST10', '{}', '2024-01-10');

    INSERT INTO match_node (id, matcher, value, confidence, importance) VALUES
    (1, 'link.1-2', 'chain1', 100, 0),
    (2, 'link.2-3', 'chain2', 100, 0),
    (3, 'link.3-4', 'chain3', 100, 0),
    (4, 'link.4-5', 'chain4', 100, 0),
    (5, 'link.5-6', 'chain5', 100, 0),
    (6, 'link.6-7', 'chain6', 100, 0),
    (7, 'link.7-8', 'chain7', 100, 0),
    (8, 'link.8-9', 'chain8', 100, 0),
    (9, 'link.9-10', 'chain9', 100, 0);

    -- Connect transactions in a chain
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (1, 1);
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (1, 2);
    
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (2, 2);
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (2, 3);
    
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (3, 3);
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (3, 4);
    
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (4, 4);
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (4, 5);
    
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (5, 5);
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (5, 6);
    
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (6, 6);
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (6, 7);
    
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (7, 7);
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (7, 8);
    
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (8, 8);
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (8, 9);
    
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (9, 9);
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (9, 10);

    -- Test Case 3: Full chain traversal
    RAISE NOTICE 'Running Test Case 3: Chain with depth 10';
    result_count := (SELECT COUNT(*) FROM find_connected_transactions(1));
    expected_count := 10; -- Should find all in chain
    insert into test_results VALUES (3, 'Finds all transactions in a chain', expected_count, result_count, result_count = expected_count);

    -- Test Case 4: Chain with depth limit 5 (testing max_depth filter)
    RAISE NOTICE 'Running Test Case 4: Chain with depth limit 5';
    result_count := (SELECT COUNT(*) FROM find_connected_transactions(1, 5));
    expected_count := 5; -- Should only reach ids 1-5 with depth limit 5
    insert into test_results VALUES (4, 'Respects max_depth limit of 5', expected_count, result_count, result_count = expected_count);
    
    -- Test Case 5: Limit number of results (testing limit_count parameter)
    RAISE NOTICE 'Running Test Case 5: Limit number of results';
    result_count := (SELECT COUNT(*) FROM find_connected_transactions(1, NULL, 5));
    expected_count := 5; -- Should only return 5 results due to limit
    insert into test_results VALUES (5, 'Respects limit_count of 5', expected_count, result_count, result_count = expected_count);

    -- Test Case 6: Filter by timestamp_alpha using filter_config (days)
    RAISE NOTICE 'Setting up Test Case 6: timestamp_alpha filter via filter_config';
    TRUNCATE TABLE match_node_transactions CASCADE;
    TRUNCATE TABLE match_node CASCADE;
    TRUNCATE TABLE transactions CASCADE;
    
    INSERT INTO transactions (id, payload_number, payload, created_at) VALUES 
    (1, 'TEST1', '{}', '2024-01-01'),
    (2, 'TEST2', '{}', '2024-01-02'),
    (3, 'TEST3', '{}', '2024-02-15');
    
    INSERT INTO match_node (id, matcher, value, confidence, importance) VALUES
    (1, 'customer.email', 'email@test.com', 100, 0);
    
    -- Root (1) with datetime_alpha 2024-01-01, candidate 2 within 1 day, candidate 3 far away
    INSERT INTO match_node_transactions (node_id, transaction_id, datetime_alpha) VALUES (1, 1, '2024-01-01');
    INSERT INTO match_node_transactions (node_id, transaction_id, datetime_alpha) VALUES (1, 2, '2024-01-02');
    INSERT INTO match_node_transactions (node_id, transaction_id, datetime_alpha) VALUES (1, 3, '2024-02-15');
    
    RAISE NOTICE 'Running Test Case 6: timestamp_alpha <= 1 day retains 1 & 2, excludes 3';
    result_count := (SELECT COUNT(*) FROM find_connected_transactions(
        1,
        NULL,
        NULL,
        '{"customer.email": {"timestamp_alpha": 1}}'::jsonb
    ));
    expected_count := 2; -- Should include 1 and 2 only
    insert into test_results VALUES (6, 'timestamp_alpha filter via filter_config', expected_count, result_count, result_count = expected_count);
    
    -- Test Case 7: Complex connections with multiple matchers
    RAISE NOTICE 'Setting up Test Case 7: Complex connections with multiple matchers';
    TRUNCATE TABLE match_node_transactions CASCADE;
    TRUNCATE TABLE match_node CASCADE;
    TRUNCATE TABLE transactions CASCADE;
    
    INSERT INTO transactions (id, payload_number, payload, created_at) VALUES 
    (1, 'TEST1', '{}', '2024-01-01'),
    (2, 'TEST2', '{}', '2024-01-02'),
    (3, 'TEST3', '{}', '2024-01-03'),
    (4, 'TEST4', '{}', '2024-01-04'),
    (5, 'TEST5', '{}', '2024-01-05'),
    (6, 'TEST6', '{}', '2024-01-06'),
    (7, 'TEST7', '{}', '2024-01-07'),
    (8, 'TEST8', '{}', '2024-01-08'),
    (9, 'TEST9', '{}', '2024-01-09'),
    (10, 'TEST10', '{}', '2024-01-10');

    INSERT INTO match_node (id, matcher, value, confidence, importance) VALUES
    (1, 'customer.email', 'email1@test.com', 100, 10),
    (2, 'customer.phone', '+1234567890', 90, 5),
    (3, 'payment.card', '1234XXXX', 80, 20),
    (4, 'device.id', 'device123', 70, 15),
    (5, 'ip.address', '192.168.1.1', 60, 0);

    -- Create complex connections one by one
    -- Transaction 1 has email, phone, card
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (1, 1);
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (2, 1);
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (3, 1);
    
    -- Transaction 2 shares email with 1
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (1, 2);
    
    -- Transaction 3 shares phone with 1
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (2, 3);
    
    -- Transaction 4 shares card with 1
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (3, 4);
    
    -- Transaction 5 shares device with 6
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (4, 5);
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (4, 6);
    
    -- Transaction 7, 8, 9, 10 share IP
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (5, 7);
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (5, 8);
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (5, 9);
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (5, 10);

    -- Test Case 7: Complex connections (testing multi-attribute connections)
    RAISE NOTICE 'Running Test Case 7: Complex connections with multiple matchers';
    result_count := (SELECT COUNT(*) FROM find_connected_transactions(1));
    expected_count := 4; -- Should find 1,2,3,4 (transactions directly connected to 1)
    insert into test_results VALUES (7, 'Finds correct connections in complex network', expected_count, result_count, result_count = expected_count);
    
    -- Test Case 8: Confidence filter test
    RAISE NOTICE 'Setting up Test Case 8: Confidence threshold testing';
    TRUNCATE TABLE match_node_transactions CASCADE;
    TRUNCATE TABLE match_node CASCADE;
    TRUNCATE TABLE transactions CASCADE;
    
    INSERT INTO transactions (id, payload_number, payload, created_at) VALUES 
    (1, 'TEST1', '{}', '2024-01-01'),
    (2, 'TEST2', '{}', '2024-01-02'),
    (3, 'TEST3', '{}', '2024-01-03'),
    (4, 'TEST4', '{}', '2024-01-04');

    INSERT INTO match_node (id, matcher, value, confidence, importance) VALUES
    (1, 'high.conf', 'high', 100, 0),
    (2, 'medium.conf', 'medium', 70, 0),
    (3, 'low.conf', 'low', 30, 0);

    -- Create connections with different confidence levels
    -- Transaction 1 connects to others with different confidence
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (1, 1);
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (1, 2);
    
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (2, 1);
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (2, 3);
    
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (3, 1);
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (3, 4);

    -- Test Case 8a: High confidence filter (should only include transactions with high conf)
    RAISE NOTICE 'Running Test Case 8a: High confidence filter';
    result_count := (SELECT COUNT(*) FROM find_connected_transactions(1, NULL, NULL, NULL, 80));
    expected_count := 2; -- Should only include 1,2 (high confidence)
    insert into test_results VALUES (8, 'Filters by min_confidence correctly', expected_count, result_count, result_count = expected_count);

    -- Test Case 9: Avoid cycles in traversal
    RAISE NOTICE 'Setting up Test Case 9: Cycle detection';
    TRUNCATE TABLE match_node_transactions CASCADE;
    TRUNCATE TABLE match_node CASCADE;
    TRUNCATE TABLE transactions CASCADE;
    
    INSERT INTO transactions (id, payload_number, payload, created_at) VALUES 
    (1, 'TEST1', '{}', '2024-01-01'),
    (2, 'TEST2', '{}', '2024-01-02'),
    (3, 'TEST3', '{}', '2024-01-03'),
    (4, 'TEST4', '{}', '2024-01-04');

    INSERT INTO match_node (id, matcher, value, confidence, importance) VALUES
    (1, 'link.a', 'link-a', 100, 0),
    (2, 'link.b', 'link-b', 100, 0),
    (3, 'link.c', 'link-c', 100, 0);

    -- Create a cycle: 1 -> 2 -> 3 -> 4 -> 1
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (1, 1);
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (1, 2);
    
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (2, 2);
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (2, 3);
    
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (3, 3);
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (3, 4);
    
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (1, 4);

    -- Test Case 9: Cycle detection (should not get stuck in loop)
    RAISE NOTICE 'Running Test Case 9: Cycle detection';
    result_count := (SELECT COUNT(*) FROM find_connected_transactions(1));
    expected_count := 4; -- Should find all 4 transactions
    insert into test_results VALUES (9, 'Avoids endless loops in cycles', expected_count, result_count, result_count = expected_count);

    -- Test Case 10: Filter by location_alpha using filter_config (meters)
    RAISE NOTICE 'Setting up Test Case 10: location_alpha filter via filter_config';
    TRUNCATE TABLE match_node_transactions CASCADE;
    TRUNCATE TABLE match_node CASCADE;
    TRUNCATE TABLE transactions CASCADE;
    
    INSERT INTO transactions (id, payload_number, payload, created_at) VALUES 
    (1, 'TEST1', '{}', '2024-01-01'),
    (2, 'TEST2', '{}', '2024-01-02'),
    (3, 'TEST3', '{}', '2024-01-03');
    
    INSERT INTO match_node (id, matcher, value, confidence, importance) VALUES
    (1, 'customer.email', 'geo@test.com', 100, 0);
    
    -- Root at NYC, near candidate within ~30m, far candidate in LA
    INSERT INTO match_node_transactions (node_id, transaction_id, long_alpha, lat_alpha) VALUES (1, 1, -74.0060, 40.7128);
    INSERT INTO match_node_transactions (node_id, transaction_id, long_alpha, lat_alpha) VALUES (1, 2, -74.0062, 40.7130);
    INSERT INTO match_node_transactions (node_id, transaction_id, long_alpha, lat_alpha) VALUES (1, 3, -118.2437, 34.0522);
    
    RAISE NOTICE 'Running Test Case 10: location_alpha <= 200m retains 1 & 2, excludes 3';
    result_count := (SELECT COUNT(*) FROM find_connected_transactions(
        1,
        NULL,
        NULL,
        '{"customer.email": {"location_alpha": 200}}'::jsonb
    ));
    expected_count := 2; -- Should include 1 and 2 only
    insert into test_results VALUES (10, 'location_alpha filter via filter_config', expected_count, result_count, result_count = expected_count);

    -- Test Case 11: Combined filter on timestamp_beta (days) and location_beta (meters)
    RAISE NOTICE 'Setting up Test Case 11: combined timestamp_beta and location_beta filters';
    TRUNCATE TABLE match_node_transactions CASCADE;
    TRUNCATE TABLE match_node CASCADE;
    TRUNCATE TABLE transactions CASCADE;
    
    INSERT INTO transactions (id, payload_number, payload, created_at) VALUES 
    (1, 'TEST1', '{}', '2024-01-01'),
    (2, 'TEST2', '{}', '2024-01-02'),
    (3, 'TEST3', '{}', '2024-01-03');
    
    INSERT INTO match_node (id, matcher, value, confidence, importance) VALUES
    (1, 'customer.email', 'both@test.com', 100, 0);
    
    -- Root beta at NYC, date 2024-01-01; candidate 2 close in space and time; candidate 3 close in time but far in space
    INSERT INTO match_node_transactions (node_id, transaction_id, datetime_beta, long_beta, lat_beta) VALUES (1, 1, '2024-01-01', -74.0060, 40.7128);
    INSERT INTO match_node_transactions (node_id, transaction_id, datetime_beta, long_beta, lat_beta) VALUES (1, 2, '2024-01-02', -74.0061, 40.7129);
    INSERT INTO match_node_transactions (node_id, transaction_id, datetime_beta, long_beta, lat_beta) VALUES (1, 3, '2024-01-02', -118.2437, 34.0522);
    
    RAISE NOTICE 'Running Test Case 11: timestamp_beta <= 2d AND location_beta <= 200m retains 1 & 2 only';
    result_count := (SELECT COUNT(*) FROM find_connected_transactions(
        1,
        NULL,
        NULL,
        '{"customer.email": {"timestamp_beta": 2, "location_beta": 200}}'::jsonb
    ));
    expected_count := 2; -- Should include 1 and 2 only (3 fails due to distance)
    insert into test_results VALUES (11, 'combined beta time+geo filters via filter_config', expected_count, result_count, result_count = expected_count);

    -- Test Case 12: Ensure transactions sharing payload_number are excluded
    RAISE NOTICE 'Setting up Test Case 12: identical payload numbers should not connect';
    TRUNCATE TABLE match_node_transactions CASCADE;
    TRUNCATE TABLE match_node CASCADE;
    TRUNCATE TABLE transactions CASCADE;
    
    INSERT INTO transactions (id, payload_number, transaction_version, is_latest, payload, created_at) VALUES
    (1, 'SAMEPAY', 1, TRUE, '{}', '2024-03-01'),
    (2, 'SAMEPAY', 2, TRUE, '{}', '2024-03-02');
    
    INSERT INTO match_node (id, matcher, value, confidence, importance) VALUES
    (1, 'customer.email', 'same@payload.com', 100, 0);
    
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (1, 1);
    INSERT INTO match_node_transactions (node_id, transaction_id) VALUES (1, 2);
    
    RAISE NOTICE 'Running Test Case 12: ensure same payload transactions are filtered out';
    result_set := COALESCE(ARRAY(SELECT transaction_id FROM find_connected_transactions(1)), ARRAY[]::BIGINT[]);
    expected_set := ARRAY[1];
    insert into test_results VALUES (
        12,
        'Excludes transactions sharing payload_number',
        COALESCE(array_length(expected_set, 1), 0),
        COALESCE(array_length(result_set, 1), 0),
        result_set = expected_set
    );

    return query select * from test_results;
END;
$$ LANGUAGE plpgsql;

-- Run the test function and show results
select * from test_find_connected_transactions();