// MongoDB index creation for match_nodes to support graph traversal and filters
// Apply with: mongo <db> migrations/20260119_match_node_indexes.js

db.match_nodes.createIndex({ payload_numbers: 1, confidence: -1 });
db.match_nodes.createIndex({ matcher: 1, value: 1 });
db.match_nodes.createIndex({ "transaction_data.payload_number": 1, confidence: -1 });
db.match_nodes.createIndex({ "transaction_data.datetime_alpha": 1 });

