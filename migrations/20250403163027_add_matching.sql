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
CREATE UNIQUE INDEX idx_match_node_matcher_value ON match_node(matcher, value);

CREATE TABLE IF NOT EXISTS match_node_transactions (
    node_id BIGINT NOT NULL,
    payload_number TEXT NOT NULL,
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
    PRIMARY KEY (node_id, payload_number),
    CONSTRAINT fk_match_node_transactions_node FOREIGN KEY (node_id) REFERENCES match_node(id) ON DELETE CASCADE
);
-- Add indexes for faster lookups
CREATE INDEX idx_match_node_transactions_payload_number ON match_node_transactions(payload_number);
CREATE INDEX idx_match_node_transactions_node_id ON match_node_transactions(node_id);
CREATE INDEX idx_match_node_transactions_payload_number_node_id on match_node_transactions(payload_number, node_id);
CREATE INDEX idx_match_node_transactions_node_id_payload_number on match_node_transactions(node_id, payload_number);

DROP FUNCTION if exists find_connected_transactions;
DROP FUNCTION if exists test_find_connected_transactions;
DROP FUNCTION if exists init_transaction_matching;
DROP FUNCTION if exists find_transaction_connections;
DROP FUNCTION if exists apply_transaction_filters;
DROP FUNCTION if exists init_with_starting_transaction;

create or replace function find_next_layer_direct_connections(
    root_payload_numbers text[],
    min_confidence INTEGER DEFAULT 0
)
returns table (parent_payload_number text, next_payload_number text, node_id bigint)
as $$
begin
    if array_length(root_payload_numbers, 1) = 0 then
        return;
    end if;

    return query select distinct on (mnt_next.payload_number)
        mnt.payload_number as parent_payload_number,
        mnt_next.payload_number as next_payload_number,
        node.id as node_id
    from match_node_transactions mnt
    join match_node node on node.id = mnt.node_id
    join match_node_transactions mnt_next on mnt_next.node_id = node.id
    LEFT JOIN tmp_filter_config cfg ON cfg.matcher = node.matcher
    where mnt.payload_number = ANY(root_payload_numbers)
        and mnt_next.payload_number != ANY (root_payload_numbers)
        and node.confidence >= min_confidence
        AND (
            cfg.matcher IS NULL OR (
                (cfg.timestamp_alpha IS NULL OR
                    mnt.datetime_alpha IS NULL OR
                    mnt_next.datetime_alpha IS NULL OR
                    ABS(EXTRACT(EPOCH FROM (mnt_next.datetime_alpha - mnt.datetime_alpha)))/86400.0 <= cfg.timestamp_alpha
                )
                AND
                (cfg.timestamp_beta IS NULL OR
                    mnt_next.datetime_beta IS NULL OR
                    mnt.datetime_beta IS NULL OR
                    ABS(EXTRACT(EPOCH FROM (mnt_next.datetime_beta - mnt.datetime_beta)))/86400.0 <= cfg.timestamp_beta
                )
                AND
                (cfg.location_alpha IS NULL OR
                    mnt_next.long_alpha IS NULL OR mnt_next.lat_alpha IS NULL OR
                    mnt.long_alpha IS NULL OR mnt.lat_alpha IS NULL OR
                    ST_Distance(
                        ST_SetSRID(ST_MakePoint(mnt.long_alpha, mnt.lat_alpha), 4326)::geography,
                        ST_SetSRID(ST_MakePoint(mnt_next.long_alpha, mnt_next.lat_alpha), 4326)::geography
                    ) <= cfg.location_alpha
                )
                AND
                (cfg.location_beta IS NULL OR
                    mnt_next.long_beta IS NULL OR mnt_next.lat_beta IS NULL OR
                    mnt.long_beta IS NULL OR mnt.lat_beta IS NULL OR
                    ST_Distance(
                        ST_SetSRID(ST_MakePoint(mnt.long_beta, mnt.lat_beta), 4326)::geography,
                        ST_SetSRID(ST_MakePoint(mnt_next.long_beta, mnt_next.lat_beta), 4326)::geography
                    ) <= cfg.location_beta
                )
                AND
                (cfg.location_gamma IS NULL OR
                    mnt_next.long_gamma IS NULL OR mnt_next.lat_gamma IS NULL OR
                    mnt.long_gamma IS NULL OR mnt.lat_gamma IS NULL OR
                    ST_Distance(
                        ST_SetSRID(ST_MakePoint(mnt.long_gamma, mnt.lat_gamma), 4326)::geography,
                        ST_SetSRID(ST_MakePoint(mnt_next.long_gamma, mnt_next.lat_gamma), 4326)::geography
                    ) <= cfg.location_gamma
                )
                AND
                (cfg.location_delta IS NULL OR
                    mnt_next.long_delta IS NULL OR mnt_next.lat_delta IS NULL OR
                    mnt.long_delta IS NULL OR mnt.lat_delta IS NULL OR
                    ST_Distance(
                        ST_SetSRID(ST_MakePoint(mnt.long_delta, mnt.lat_delta), 4326)::geography,
                        ST_SetSRID(ST_MakePoint(mnt_next.long_delta, mnt_next.lat_delta), 4326)::geography
                    ) <= cfg.location_delta
                )
            )
        )
    order by mnt_next.payload_number, node.confidence desc, node.importance desc;
end;
$$ language plpgsql;

CREATE OR REPLACE FUNCTION find_connected_transactions(
    root_payload_number TEXT,
    max_depth INTEGER DEFAULT 10,
    limit_count INTEGER DEFAULT 1000,
    filter_config JSONB DEFAULT NULL,
    min_confidence INTEGER DEFAULT 0
) RETURNS TABLE (
    transaction_id BIGINT,
    parent_transaction_id BIGINT,
    matcher TEXT,
    confidence INTEGER,
    importance INTEGER,
    created_at TIMESTAMP
) AS $$
BEGIN
    DROP TABLE IF EXISTS tmp_filter_config;
    CREATE TEMP TABLE tmp_filter_config (
        matcher TEXT PRIMARY KEY,
        timestamp_alpha NUMERIC,
        timestamp_beta NUMERIC,
        location_alpha DOUBLE PRECISION,
        location_beta DOUBLE PRECISION,
        location_gamma DOUBLE PRECISION,
        location_delta DOUBLE PRECISION
    ) ON COMMIT DROP;

    IF filter_config IS NOT NULL THEN
        INSERT INTO tmp_filter_config (
            matcher,
            timestamp_alpha,
            timestamp_beta,
            location_alpha,
            location_beta,
            location_gamma,
            location_delta
        )
        SELECT 
            cfg.key,
            NULLIF(cfg.value->>'timestamp_alpha', '')::numeric,
            NULLIF(cfg.value->>'timestamp_beta', '')::numeric,
            NULLIF(cfg.value->>'location_alpha', '')::double precision,
            NULLIF(cfg.value->>'location_beta', '')::double precision,
            NULLIF(cfg.value->>'location_gamma', '')::double precision,
            NULLIF(cfg.value->>'location_delta', '')::double precision
        FROM jsonb_each(filter_config) AS cfg(key, value);
    END IF;

    drop table if exists tmp_connections;
    create temp table tmp_connections (payload_number text not null primary key, parent_payload_number text, node_id bigint, depth int not null);
    insert into tmp_connections (payload_number, parent_payload_number, node_id, depth) values (root_payload_number, null, null, 0);

    FOR i IN 1..coalesce(max_depth, 10) LOOP
        insert into tmp_connections (payload_number, parent_payload_number, node_id, depth)
        select next_payload_number, parent_payload_number, node_id, i 
        from find_next_layer_direct_connections(
            (select array_agg(payload_number) from tmp_connections where depth = i - 1)::text[],
            min_confidence
        )
        where next_payload_number not in (select payload_number from tmp_connections);

        if limit_count is not null and (select count(*) from tmp_connections) >= limit_count then
            exit;
        end if;
    END LOOP;
    
    return query 
        select
            (select id from transactions where payload_number = c.payload_number order by transaction_version desc limit 1) as transaction_id,
            (select id from transactions where payload_number = c.parent_payload_number order by transaction_version desc limit 1) as parent_transaction_id,
            node.matcher, 
            node.confidence, 
            node.importance,
            mnt.created_at
        from tmp_connections c
        join match_node_transactions mnt on mnt.payload_number = c.payload_number and mnt.node_id = c.node_id
        join match_node node on node.id = c.node_id
        join transactions t on t.payload_number = c.payload_number
        limit coalesce(limit_count, 2147483647);

    drop table if exists tmp_connections;
END $$ language plpgsql;


-------------------- TESTS -------------------------------------------------------------
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
    latest_id BIGINT;
BEGIN
    CREATE TEMPORARY TABLE test_results (
        case_number INT,
        description TEXT,
        expected INT,
        actual INT,
        pass_fail BOOLEAN
    ) ON COMMIT DROP;

    -- Test Case 1: 10 payloads, all connected through customer.email
    RAISE NOTICE 'Setting up Test Case 1: 10 payloads, all connected through customer.email';
    TRUNCATE TABLE match_node_transactions CASCADE;
    TRUNCATE TABLE match_node CASCADE;
    TRUNCATE TABLE transactions CASCADE;

    INSERT INTO transactions (id, payload_number, transaction_version, payload, created_at) VALUES 
    (1, 'TEST1', 1, '{}', '2024-01-01'),
    (2, 'TEST2', 1, '{}', '2024-01-02'),
    (3, 'TEST3', 1, '{}', '2024-01-03'),
    (4, 'TEST4', 1, '{}', '2024-01-04'),
    (5, 'TEST5', 1, '{}', '2024-01-05'),
    (6, 'TEST6', 1, '{}', '2024-01-06'),
    (7, 'TEST7', 1, '{}', '2024-01-07'),
    (8, 'TEST8', 1, '{}', '2024-01-08'),
    (9, 'TEST9', 1, '{}', '2024-01-09'),
    (10, 'TEST10', 1, '{}', '2024-01-10');

    INSERT INTO match_node (id, matcher, value, confidence, importance) VALUES
    (1, 'customer.email', 'test@test.com', 100, 0);

    INSERT INTO match_node_transactions (node_id, payload_number) VALUES
    (1, 'TEST1'), (1, 'TEST2'), (1, 'TEST3'), (1, 'TEST4'), (1, 'TEST5'),
    (1, 'TEST6'), (1, 'TEST7'), (1, 'TEST8'), (1, 'TEST9'), (1, 'TEST10');

    RAISE NOTICE 'Running Test Case 1: All connected payloads';
    result_count := (SELECT COUNT(*) FROM find_connected_transactions('TEST1'));
    expected_count := 9; -- root payload excluded
    INSERT INTO test_results VALUES (1, 'All payloads connected through email', expected_count, result_count, result_count = expected_count);

    -- Test Case 2: Two disconnected groups
    RAISE NOTICE 'Setting up Test Case 2: disconnected groups';
    TRUNCATE TABLE match_node_transactions CASCADE;
    TRUNCATE TABLE match_node CASCADE;
    TRUNCATE TABLE transactions CASCADE;

    INSERT INTO transactions (id, payload_number, transaction_version, payload, created_at) VALUES 
    (1, 'TEST1', 1, '{}', '2024-01-01'),
    (2, 'TEST2', 1, '{}', '2024-01-02'),
    (3, 'TEST3', 1, '{}', '2024-01-03'),
    (4, 'TEST4', 1, '{}', '2024-01-04'),
    (5, 'TEST5', 1, '{}', '2024-01-05'),
    (6, 'TEST6', 1, '{}', '2024-01-06'),
    (7, 'TEST7', 1, '{}', '2024-01-07'),
    (8, 'TEST8', 1, '{}', '2024-01-08'),
    (9, 'TEST9', 1, '{}', '2024-01-09'),
    (10, 'TEST10', 1, '{}', '2024-01-10');

    INSERT INTO match_node (id, matcher, value, confidence, importance) VALUES
    (1, 'customer.email', 'group1@test.com', 100, 0),
    (2, 'customer.email', 'group2@test.com', 100, 0);

    INSERT INTO match_node_transactions (node_id, payload_number) VALUES
    (1, 'TEST1'),(1, 'TEST2'),(1, 'TEST3'),(1, 'TEST4'),(1, 'TEST5'),
    (2, 'TEST6'),(2, 'TEST7'),(2, 'TEST8'),(2, 'TEST9'),(2, 'TEST10');

    RAISE NOTICE 'Running Test Case 2: Only group1 reachable';
    result_count := (SELECT COUNT(*) FROM find_connected_transactions('TEST1'));
    expected_count := 4; -- excluding root
    INSERT INTO test_results VALUES (2, 'Only finds payloads from connected group', expected_count, result_count, result_count = expected_count);

    -- Test Case 3: Chain traversal
    RAISE NOTICE 'Setting up Test Case 3: chain';
    TRUNCATE TABLE match_node_transactions CASCADE;
    TRUNCATE TABLE match_node CASCADE;
    TRUNCATE TABLE transactions CASCADE;

    INSERT INTO transactions (id, payload_number, transaction_version, payload, created_at) VALUES 
    (1, 'TEST1', 1, '{}', '2024-01-01'),
    (2, 'TEST2', 1, '{}', '2024-01-02'),
    (3, 'TEST3', 1, '{}', '2024-01-03'),
    (4, 'TEST4', 1, '{}', '2024-01-04'),
    (5, 'TEST5', 1, '{}', '2024-01-05'),
    (6, 'TEST6', 1, '{}', '2024-01-06'),
    (7, 'TEST7', 1, '{}', '2024-01-07'),
    (8, 'TEST8', 1, '{}', '2024-01-08'),
    (9, 'TEST9', 1, '{}', '2024-01-09'),
    (10, 'TEST10', 1, '{}', '2024-01-10');

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

    INSERT INTO match_node_transactions (node_id, payload_number) VALUES
    (1, 'TEST1'),(1, 'TEST2'),
    (2, 'TEST2'),(2, 'TEST3'),
    (3, 'TEST3'),(3, 'TEST4'),
    (4, 'TEST4'),(4, 'TEST5'),
    (5, 'TEST5'),(5, 'TEST6'),
    (6, 'TEST6'),(6, 'TEST7'),
    (7, 'TEST7'),(7, 'TEST8'),
    (8, 'TEST8'),(8, 'TEST9'),
    (9, 'TEST9'),(9, 'TEST10');

    RAISE NOTICE 'Running Test Case 3: full chain';
    result_count := (SELECT COUNT(*) FROM find_connected_transactions('TEST1'));
    expected_count := 9; -- excluding root
    INSERT INTO test_results VALUES (3, 'Finds all payloads in a chain', expected_count, result_count, result_count = expected_count);

    RAISE NOTICE 'Running Test Case 4: depth limit 5';
    result_count := (SELECT COUNT(*) FROM find_connected_transactions('TEST1', 5));
    expected_count := 5; -- excluding root
    INSERT INTO test_results VALUES (4, 'Respects max_depth limit of 5', expected_count, result_count, result_count = expected_count);

    RAISE NOTICE 'Running Test Case 5: limit count 5';
    result_count := (SELECT COUNT(*) FROM find_connected_transactions('TEST1', NULL, 5));
    expected_count := 4; -- excluding root
    INSERT INTO test_results VALUES (5, 'Respects limit_count of 5', expected_count, result_count, result_count = expected_count);

    -- Test Case 6: timestamp_alpha filter
    RAISE NOTICE 'Setting up Test Case 6: timestamp_alpha filter via filter_config';
    TRUNCATE TABLE match_node_transactions CASCADE;
    TRUNCATE TABLE match_node CASCADE;
    TRUNCATE TABLE transactions CASCADE;

    INSERT INTO transactions (id, payload_number, transaction_version, payload, created_at) VALUES 
    (1, 'TEST1', 1, '{}', '2024-01-01'),
    (2, 'TEST2', 1, '{}', '2024-01-02'),
    (3, 'TEST3', 1, '{}', '2024-02-15');

    INSERT INTO match_node (id, matcher, value, confidence, importance) VALUES
    (1, 'customer.email', 'email@test.com', 100, 0);

    INSERT INTO match_node_transactions (node_id, payload_number, datetime_alpha) VALUES
    (1, 'TEST1', '2024-01-01'),
    (1, 'TEST2', '2024-01-02'),
    (1, 'TEST3', '2024-02-15');

    RAISE NOTICE 'Running Test Case 6';
    result_count := (SELECT COUNT(*) FROM find_connected_transactions(
        'TEST1',
        NULL,
        NULL,
        '{"customer.email": {"timestamp_alpha": 1}}'::jsonb
    ));
    expected_count := 1; -- root excluded
    INSERT INTO test_results VALUES (6, 'timestamp_alpha filter via filter_config', expected_count, result_count, result_count = expected_count);

    -- Test Case 7: Complex connections with multiple matchers
    RAISE NOTICE 'Setting up Test Case 7: complex network';
    TRUNCATE TABLE match_node_transactions CASCADE;
    TRUNCATE TABLE match_node CASCADE;
    TRUNCATE TABLE transactions CASCADE;

    INSERT INTO transactions (id, payload_number, transaction_version, payload, created_at) VALUES 
    (1, 'TEST1', 1, '{}', '2024-01-01'),
    (2, 'TEST2', 1, '{}', '2024-01-02'),
    (3, 'TEST3', 1, '{}', '2024-01-03'),
    (4, 'TEST4', 1, '{}', '2024-01-04'),
    (5, 'TEST5', 1, '{}', '2024-01-05'),
    (6, 'TEST6', 1, '{}', '2024-01-06'),
    (7, 'TEST7', 1, '{}', '2024-01-07'),
    (8, 'TEST8', 1, '{}', '2024-01-08'),
    (9, 'TEST9', 1, '{}', '2024-01-09'),
    (10, 'TEST10', 1, '{}', '2024-01-10');

    INSERT INTO match_node (id, matcher, value, confidence, importance) VALUES
    (1, 'customer.email', 'email1@test.com', 100, 10),
    (2, 'customer.phone', '+1234567890', 90, 5),
    (3, 'payment.card', '1234XXXX', 80, 20),
    (4, 'device.id', 'device123', 70, 15),
    (5, 'ip.address', '192.168.1.1', 60, 0);

    INSERT INTO match_node_transactions (node_id, payload_number) VALUES
    (1, 'TEST1'),(2, 'TEST1'),(3, 'TEST1'),
    (1, 'TEST2'),
    (2, 'TEST3'),
    (3, 'TEST4'),
    (4, 'TEST5'),(4, 'TEST6'),
    (5, 'TEST7'),(5, 'TEST8'),(5, 'TEST9'),(5, 'TEST10');

    RAISE NOTICE 'Running Test Case 7';
    result_count := (SELECT COUNT(*) FROM find_connected_transactions('TEST1'));
    expected_count := 3; -- excluding root
    INSERT INTO test_results VALUES (7, 'Finds correct connections in complex network', expected_count, result_count, result_count = expected_count);

    -- Test Case 8: Confidence filter
    RAISE NOTICE 'Setting up Test Case 8: confidence threshold';
    TRUNCATE TABLE match_node_transactions CASCADE;
    TRUNCATE TABLE match_node CASCADE;
    TRUNCATE TABLE transactions CASCADE;

    INSERT INTO transactions (id, payload_number, transaction_version, payload, created_at) VALUES 
    (1, 'TEST1', 1, '{}', '2024-01-01'),
    (2, 'TEST2', 1, '{}', '2024-01-02'),
    (3, 'TEST3', 1, '{}', '2024-01-03'),
    (4, 'TEST4', 1, '{}', '2024-01-04');

    INSERT INTO match_node (id, matcher, value, confidence, importance) VALUES
    (1, 'high.conf', 'high', 100, 0),
    (2, 'medium.conf', 'medium', 70, 0),
    (3, 'low.conf', 'low', 30, 0);

    INSERT INTO match_node_transactions (node_id, payload_number) VALUES
    (1, 'TEST1'),(1, 'TEST2'),
    (2, 'TEST1'),(2, 'TEST3'),
    (3, 'TEST1'),(3, 'TEST4');

    RAISE NOTICE 'Running Test Case 8';
    result_count := (SELECT COUNT(*) FROM find_connected_transactions('TEST1', NULL, NULL, NULL, 80));
    expected_count := 1; -- excluding root
    INSERT INTO test_results VALUES (8, 'Filters by min_confidence correctly', expected_count, result_count, result_count = expected_count);

    -- Test Case 9: Cycle detection
    RAISE NOTICE 'Setting up Test Case 9: cycles';
    TRUNCATE TABLE match_node_transactions CASCADE;
    TRUNCATE TABLE match_node CASCADE;
    TRUNCATE TABLE transactions CASCADE;

    INSERT INTO transactions (id, payload_number, transaction_version, payload, created_at) VALUES 
    (1, 'TEST1', 1, '{}', '2024-01-01'),
    (2, 'TEST2', 1, '{}', '2024-01-02'),
    (3, 'TEST3', 1, '{}', '2024-01-03'),
    (4, 'TEST4', 1, '{}', '2024-01-04');

    INSERT INTO match_node (id, matcher, value, confidence, importance) VALUES
    (1, 'link.a', 'link-a', 100, 0),
    (2, 'link.b', 'link-b', 100, 0),
    (3, 'link.c', 'link-c', 100, 0);

    INSERT INTO match_node_transactions (node_id, payload_number) VALUES
    (1, 'TEST1'),(1, 'TEST2'),
    (2, 'TEST2'),(2, 'TEST3'),
    (3, 'TEST3'),(3, 'TEST4'),
    (1, 'TEST4');

    RAISE NOTICE 'Running Test Case 9';
    result_count := (SELECT COUNT(*) FROM find_connected_transactions('TEST1'));
    expected_count := 3; -- excluding root
    INSERT INTO test_results VALUES (9, 'Avoids endless loops in cycles', expected_count, result_count, result_count = expected_count);

    -- Test Case 10: location_alpha filter
    RAISE NOTICE 'Setting up Test Case 10: location_alpha filter';
    TRUNCATE TABLE match_node_transactions CASCADE;
    TRUNCATE TABLE match_node CASCADE;
    TRUNCATE TABLE transactions CASCADE;

    INSERT INTO transactions (id, payload_number, transaction_version, payload, created_at) VALUES 
    (1, 'TEST1', 1, '{}', '2024-01-01'),
    (2, 'TEST2', 1, '{}', '2024-01-02'),
    (3, 'TEST3', 1, '{}', '2024-01-03');

    INSERT INTO match_node (id, matcher, value, confidence, importance) VALUES
    (1, 'customer.email', 'geo@test.com', 100, 0);

    INSERT INTO match_node_transactions (node_id, payload_number, long_alpha, lat_alpha) VALUES
    (1, 'TEST1', -74.0060, 40.7128),
    (1, 'TEST2', -74.0062, 40.7130),
    (1, 'TEST3', -118.2437, 34.0522);

    RAISE NOTICE 'Running Test Case 10';
    result_count := (SELECT COUNT(*) FROM find_connected_transactions(
        'TEST1',
        NULL,
        NULL,
        '{"customer.email": {"location_alpha": 200}}'::jsonb
    ));
    expected_count := 1; -- root excluded
    INSERT INTO test_results VALUES (10, 'location_alpha filter via filter_config', expected_count, result_count, result_count = expected_count);

    -- Test Case 11: combined timestamp_beta + location_beta
    RAISE NOTICE 'Setting up Test Case 11: combined beta filters';
    TRUNCATE TABLE match_node_transactions CASCADE;
    TRUNCATE TABLE match_node CASCADE;
    TRUNCATE TABLE transactions CASCADE;

    INSERT INTO transactions (id, payload_number, transaction_version, payload, created_at) VALUES 
    (1, 'TEST1', 1, '{}', '2024-01-01'),
    (2, 'TEST2', 1, '{}', '2024-01-02'),
    (3, 'TEST3', 1, '{}', '2024-01-03');

    INSERT INTO match_node (id, matcher, value, confidence, importance) VALUES
    (1, 'customer.email', 'both@test.com', 100, 0);

    INSERT INTO match_node_transactions (node_id, payload_number, datetime_beta, long_beta, lat_beta) VALUES
    (1, 'TEST1', '2024-01-01', -74.0060, 40.7128),
    (1, 'TEST2', '2024-01-02', -74.0061, 40.7129),
    (1, 'TEST3', '2024-01-02', -118.2437, 34.0522);

    RAISE NOTICE 'Running Test Case 11';
    result_count := (SELECT COUNT(*) FROM find_connected_transactions(
        'TEST1',
        NULL,
        NULL,
        '{"customer.email": {"timestamp_beta": 2, "location_beta": 200}}'::jsonb
    ));
    expected_count := 1; -- root excluded
    INSERT INTO test_results VALUES (11, 'combined beta time+geo filters', expected_count, result_count, result_count = expected_count);

    -- Test Case 12: only latest version returned for a payload
    RAISE NOTICE 'Setting up Test Case 12: latest version only';
    TRUNCATE TABLE match_node_transactions CASCADE;
    TRUNCATE TABLE match_node CASCADE;
    TRUNCATE TABLE transactions CASCADE;

    INSERT INTO transactions (id, payload_number, transaction_version, payload, created_at) VALUES
    (1, 'SAMEPAY', 1, '{}', '2024-03-01'),
    (2, 'SAMEPAY', 2, '{}', '2024-03-02');

    INSERT INTO match_node (id, matcher, value, confidence, importance) VALUES
    (1, 'customer.email', 'same@payload.com', 100, 0);

    INSERT INTO match_node_transactions (node_id, payload_number) VALUES
    (1, 'SAMEPAY');

    SELECT id INTO latest_id FROM transactions WHERE payload_number = 'SAMEPAY' ORDER BY transaction_version DESC LIMIT 1;

    RAISE NOTICE 'Running Test Case 12';
    result_set := COALESCE(ARRAY(SELECT transaction_id FROM find_connected_transactions('SAMEPAY')), ARRAY[]::BIGINT[]);
    expected_set := ARRAY[]::BIGINT[]; -- root excluded, no other payloads expected
    INSERT INTO test_results VALUES (
        12,
        'Returns only latest version for payload',
        COALESCE(array_length(expected_set, 1), 0),
        COALESCE(array_length(result_set, 1), 0),
        result_set = expected_set
    );

    RETURN QUERY SELECT * FROM test_results;
END;
$$ LANGUAGE plpgsql;

select * from test_find_connected_transactions();
-- realign sequences after explicit inserts in tests
truncate table match_node_transactions CASCADE;
truncate table match_node CASCADE;
truncate table transactions CASCADE;