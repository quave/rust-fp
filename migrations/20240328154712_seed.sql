INSERT INTO scoring_models (name, features_schema_version_major, features_schema_version_minor, version, model_type)
VALUES ('Ecom', 1, 0, '1.0.0', 'RuleBased');
INSERT INTO channels (name, model_id) VALUES ('Basic', 1);
INSERT INTO channel_model_activations (channel_id, model_id) values (1, 1);

INSERT INTO expression_rules (model_id, name, description, rule, score) VALUES (1, 'High order total', 'High order total', 'amount > 1000.0', 10);
INSERT INTO expression_rules (model_id, name, description, rule, score) VALUES (1, 'Multiple items', 'Multiple items', 'item_count > 3', 11);
INSERT INTO expression_rules (model_id, name, description, rule, score) VALUES (1, 'New customer', 'New customer', 'is_new_customer', 12);
INSERT INTO expression_rules (model_id, name, description, rule, score) VALUES (1, 'High risk country', 'High risk country', 'country_code == "RU" || country_code == "BY"', 13);
INSERT INTO expression_rules (model_id, name, description, rule, score) VALUES (1, 'Late night order', 'Late night order', 'order_time > "22:00:00"', 14);