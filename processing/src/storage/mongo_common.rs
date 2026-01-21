use crate::model::processible::{ColumnValueTrait, Filter, FilterOperator};
use crate::model::{ConnectedTransaction, DirectConnection, Feature, FraudLevel, GenericError, LabelSource, MatcherConfig, MatchingField, SchemaVersion, ScoringModelType, ScoringResult};
use crate::model::mongo_model::{Label, MatchNode, MatchNodeTransaction, ScoringChannel, ScoringEvent, Transaction};
use crate::storage::common::CommonStorage;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::stream::TryStreamExt;
use mongodb::bson::oid::ObjectId;
use mongodb::bson::{self, Bson, DateTime as BsonDateTime, Document, doc, to_bson};
use mongodb::{Client, Collection, Database};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use futures::StreamExt;
use tracing::debug;

fn bson_datetime(dt: DateTime<Utc>) -> BsonDateTime {
    BsonDateTime::from_millis(dt.timestamp_millis())
}

#[derive(Clone)]
pub struct MongoCommonStorage {
    pub client: Client,
    pub database: Database,
    pub matcher_configs: HashMap<String, MatcherConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GraphNode {
    #[serde(default)]
    confidence: i32,
    matcher: String,
    #[serde(default)]
    payload_numbers: Vec<String>,
    #[serde(default)]
    depth: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AggregatedGraph {
    #[serde(default)]
    graph: Vec<GraphNode>,
    #[serde(default)]
    payload_numbers: Vec<String>,
    matcher: String,
    confidence: i32,
}

impl MongoCommonStorage {
    async fn get_connection(
        conn_str: &str,
        db_name: &str,
    ) -> Result<(Client, Database), GenericError> {
        println!("trying to connect to mongo: {}", conn_str);
        let client = Client::with_uri_str(conn_str).await?;
        let database = client.database(db_name);
        println!("connected to mongo: {}", conn_str);
        Ok((client, database))
    }

    pub async fn new(
        conn_str: &str,
        db_name: &str,
    ) -> Result<Self, GenericError> {
        let (client, database) = Self::get_connection(conn_str, db_name).await?;

        Ok(Self {
            client,
            database,
            matcher_configs: Self::default_configs(),
        })
    }

    pub async fn with_configs(
        conn_str: &str,
        db_name: &str,
        matcher_configs: HashMap<String, MatcherConfig>,
    ) -> Result<Self, GenericError> {
        let (client, database) = Self::get_connection(conn_str, db_name).await?;
        Ok(Self {
            client,
            database,
            matcher_configs,
        })
    }

    fn default_configs() -> HashMap<String, MatcherConfig> {
        let mut config = HashMap::new();
        config.insert("customer.email".to_string(), (100, 90));
        config.insert("billing.payment_details".to_string(), (100, 80));
        config.insert("ip.address".to_string(), (70, 60));
        config.insert("device.id".to_string(), (90, 70));
        config.insert("phone.number".to_string(), (95, 85));
        config
    }

    fn transactions(&self) -> Collection<Transaction> {
        self.database.collection("transactions")
    }

    fn scoring_events(&self) -> Collection<ScoringEvent> {
        self.database.collection("scoring_events")
    }

    fn model_activations(&self) -> Collection<ScoringChannel> {
        self.database.collection("model_activations")
    }

    fn match_nodes(&self) -> Collection<MatchNode> {
        self.database.collection("match_node")
    }

    fn match_node_transactions(&self) -> Collection<MatchNodeTransaction> {
        self.database.collection("match_node_transactions")
    }

    fn build_filter_document(
        filters: &[Filter<Box<dyn ColumnValueTrait>>],
    ) -> mongodb::bson::Document {
        let mut doc = mongodb::bson::Document::new();

        for filter in filters {
            let key = filter.column.clone();
            let op = &filter.operator_value;
            let value = op.to_string();

            let bson_val = match value.parse::<i64>() {
                Ok(v) => Bson::Int64(v),
                Err(_) => match value.parse::<f64>() {
                    Ok(v) => Bson::Double(v),
                    Err(_) => Bson::String(value.clone()),
                },
            };

            let clause = match op {
                FilterOperator::Equal(_) => bson_val.clone(),
                FilterOperator::NotEqual(_) => Bson::Document(doc! { "$ne": bson_val.clone() }),
                FilterOperator::GreaterThan(_) => Bson::Document(doc! { "$gt": bson_val.clone() }),
                FilterOperator::GreaterThanOrEqual(_) => {
                    Bson::Document(doc! { "$gte": bson_val.clone() })
                }
                FilterOperator::LessThan(_) => Bson::Document(doc! { "$lt": bson_val.clone() }),
                FilterOperator::LessThanOrEqual(_) => {
                    Bson::Document(doc! { "$lte": bson_val.clone() })
                }
                FilterOperator::Contains(_) => Bson::Document(doc! { "$regex": bson_val.clone() }),
                FilterOperator::In(values) => {
                    let arr: Vec<Bson> = values
                        .iter()
                        .map(|v| Bson::String(v.to_string()))
                        .collect();
                    Bson::Document(doc! { "$in": arr })
                }
                FilterOperator::NotIn(values) => {
                    let arr: Vec<Bson> = values
                        .iter()
                        .map(|v| Bson::String(v.to_string()))
                        .collect();
                    Bson::Document(doc! { "$nin": arr })
                }
                _ => bson_val.clone(),
            };

            doc.insert(key, clause);
        }

        doc
    }
}

#[async_trait]
impl CommonStorage<ObjectId> for MongoCommonStorage {
    async fn insert_imported_transaction(
        &self,
        payload_number: String,
        payload: serde_json::Value,
        schema_version: SchemaVersion,
    ) -> Result<ObjectId, GenericError> {
        let now = Utc::now().naive_utc();

        let existing = self
            .transactions()
            .find_one(doc! { "payload_number": &payload_number, "is_latest": true })
            .await?;

        if let Some(_) = existing {
            self.transactions()
                .update_many(
                    doc! { "payload_number": &payload_number },
                    doc! { "$set": { "is_latest": false } },
                )
                .await?;
        }

        let next_version = existing
            .as_ref()
            .map(|t| t.transaction_version + 1)
            .unwrap_or(1);
        let carried_label = existing.as_ref().and_then(|t| t.label.clone());
        let carried_comment = existing.as_ref().and_then(|t| t.comment.clone());

        let doc = Transaction {
            _id: ObjectId::new(),
            payload_number,
            transaction_version: next_version,
            is_latest: true,
            payload,
            schema_version_major: schema_version.0,
            schema_version_minor: schema_version.1,
            label: carried_label,
            comment: carried_comment,
            last_scoring_date: None,
            features_set: None,
            processing_complete: false,
            created_at: now,
            updated_at: now,
        };

        self.transactions().insert_one(&doc).await?;
        Ok(doc._id)
    }

    async fn get_transaction(
        &self,
        transaction_id: ObjectId,
    ) -> Result<Transaction, GenericError> {
        let model = self
            .transactions()
            .find_one(doc! { "_id": transaction_id })
            .await?
            .ok_or_else(|| format!("Transaction not found: {}", transaction_id))?;
        Ok(model)
    }

    async fn filter_transactions(
        &self,
        filters: &[Filter<Box<dyn ColumnValueTrait>>],
    ) -> Result<Vec<Transaction>, GenericError> {
        let filter_doc = Self::build_filter_document(filters);
        let mut cursor = self.transactions().find(filter_doc).await?;
        let mut transactions = Vec::new();
        while let Some(doc) = cursor.try_next().await? {
            transactions.push(doc);
        }
        Ok(transactions)
    }

    async fn mark_transaction_processed(
        &self,
        transaction_id: ObjectId,
    ) -> Result<(), GenericError> {
        let now = Utc::now();
        self.transactions()
            .update_one(
                doc! { "_id": transaction_id },
                doc! {
                    "$set": {
                        "processing_complete": true,
                        "last_scoring_date": bson_datetime(now),
                        "updated_at": bson_datetime(now),
                    }
                },
            )
            .await?;
        Ok(())
    }

    async fn get_active_model_activations(
        &self,
    ) -> Result<Vec<ScoringChannel>, GenericError> {
        let activations: Vec<ScoringChannel> = self
            .model_activations()
            .find(doc! { "is_active": true })
            .await?
            .try_collect()
            .await?;
        Ok(activations)
    }

    async fn save_scores(
        &self,
        transaction_id: ObjectId,
        channel: ScoringChannel,
        scoring_result: Box<dyn ScoringResult>,
    ) -> Result<(), GenericError> {
        let now = Utc::now().naive_utc();

        match channel.model.model_type {
            ScoringModelType::ExpressionBased => {
                let triggered_rules: Vec<String> = scoring_result
                    .get_result_payload()
                    .as_array()
                    .expect("Scoring result is not an array while model type is ExpressionBased")
                    .iter()
                    .map(|rule| rule.as_str().expect("Rule is not a string").to_string())
                    .collect::<Vec<String>>();
                
                let scoring_doc = ScoringEvent {
                    _id: ObjectId::new(),
                    transaction_id,
                    channel_id: channel._id,
                    triggered_rules,
                    created_at: now,
                };
                self.scoring_events().insert_one(scoring_doc).await?;
            }
            _ => {
                return Err(format!("Unsupported model type: {:?}", channel.model.model_type).into());
            }
        }


        Ok(())
    }

    async fn save_features<'a>(
        &self,
        transaction_id: ObjectId,
        simple_features: &'a Option<&'a [Feature]>,
        graph_features: &'a [Feature],
    ) -> Result<(), GenericError> {
        self.validate_features(graph_features)?;
        if let Some(features) = simple_features {
            self.validate_features(features)?;
        }

        let graph_json = serde_json::to_value(graph_features)?;
        let simple_json = match simple_features {
            Some(f) => Some(serde_json::to_value(f)?),
            None => None,
        };


        self.transactions().update_one(
            doc! { "_id": transaction_id },
            doc! {
                "simple_features": to_bson(&simple_json)?,
                "graph_features": to_bson(&graph_json)?,
            })
            .await?;

        Ok(())
    }

    async fn find_connected_transactions(
        &self,
        payload_number: &str,
        max_depth: Option<i32>,
        limit_count: Option<i32>,
        _filter_config: Option<serde_json::Value>,
        min_confidence: Option<i32>,
    ) -> Result<Vec<ConnectedTransaction>, GenericError> {
        let max_depth = max_depth.unwrap_or(10).max(1);
        let graph_depth = (max_depth - 1).max(0);
        let min_confidence = min_confidence.unwrap_or(0).clamp(0, 100);

        let pipeline = vec![
            doc! { "$match": { "transaction_data.payload_number": payload_number, "confidence": { "$gte": min_confidence } } },
            doc! { "$addFields": { "payload_numbers": "$payload_numbers" } },
            doc! { "$graphLookup": {
                "from": "match_nodes",
                "startWith": "$payload_numbers",
                "connectFromField": "payload_numbers",
                "connectToField": "payload_numbers",
                "as": "graph",
                "maxDepth": graph_depth,
                "depthField": "depth",
                "restrictSearchWithMatch": {
                    "payload_numbers": { "$ne": payload_number },
                    "confidence": { "$gte": min_confidence },
                }
            }},
            doc! { "$project": {
                "_id": 0,
                "matcher": 1,
                "confidence": 1,
                "payload_numbers": 1,
                "graph": {
                    "matcher": "$graph.matcher",
                    "confidence": "$graph.confidence",
                    "payload_numbers": "$graph.payload_numbers",
                    "depth": "$graph.depth"
                }
            }},
        ];

        let agg_docs: Vec<Document> = self
            .match_nodes()
            .aggregate(pipeline)
            .await?
            .try_collect()
            .await?;

        let mut agg_results = Vec::with_capacity(agg_docs.len());
        for doc in agg_docs {
            agg_results.push(bson::from_document::<AggregatedGraph>(doc)?);
        }

        Ok(Self::collect_connections(payload_number, agg_results, limit_count))
    }

    async fn get_direct_connections(
        &self,
        payload_number: &str,
    ) -> Result<Vec<DirectConnection>, GenericError> {
        let nodes = self
            .match_nodes()
            .find(doc! { "payload_numbers": payload_number })
            .await?
            .try_collect::<Vec<MatchNode>>()
            .await?;

        let mut seen = HashSet::new();
        let mut res = Vec::new();

        for node in nodes {
            for other in node.payload_numbers.iter() {
                if other == payload_number {
                    continue;
                }
                let key = format!("{}::{}", other, node.matcher);
                if seen.insert(key) {
                    res.push(DirectConnection {
                        payload_number: other.clone(),
                        matcher: node.matcher.clone(),
                        confidence: node.confidence,
                        importance: node.importance,
                    });
                }
            }
        }

        Ok(res)
    }

    async fn save_matching_fields_with_timespace(
        &self,
        transaction_id: &ObjectId,
        matching_fields: &[MatchingField],
        datetime_alpha: Option<DateTime<Utc>>,
        datetime_beta: Option<DateTime<Utc>>,
        long_alpha: Option<f64>,
        lat_alpha: Option<f64>,
        long_beta: Option<f64>,
        lat_beta: Option<f64>,
        long_gamma: Option<f64>,
        lat_gamma: Option<f64>,
        long_delta: Option<f64>,
        lat_delta: Option<f64>,
    ) -> Result<(), GenericError> {
        if matching_fields.is_empty() {
            return Ok(());
        }

        let payload_number = self
            .transactions()
            .find_one(doc! { "_id": transaction_id })
            .await?
            .map(|t| t.payload_number)
            .ok_or_else(|| format!("transaction {} not found", transaction_id))?;

        for field in matching_fields {
            let (conf, imp) = self
                .matcher_configs
                .get(&field.matcher)
                .cloned()
                .expect("Matcher config not found");

            let mut maybeNode = self
                .match_nodes()
                .find_one(doc! { "matcher": &field.matcher, "value": &field.value})
                .await?;

            let mnt = MatchNodeTransaction {
                payload_number: payload_number.clone(),
                datetime_alpha: datetime_alpha.map(|dt| dt.naive_utc()),
                datetime_beta: datetime_beta.map(|dt| dt.naive_utc()),
                long_alpha,
                lat_alpha,
                long_beta,
                lat_beta,
                long_gamma,
                lat_gamma,
                long_delta,
                lat_delta,
                created_at: Utc::now().naive_utc(),
            };

            if let Some(mut node) = maybeNode {
                node.transaction_data.push(mnt);
                if !node.payload_numbers.contains(&payload_number) {
                    node.payload_numbers.push(payload_number.clone());
                }
                self.match_nodes().update_one(
                    doc! { "_id": node._id },
                    doc! {
                        "$set": {
                            "transaction_data": to_bson(&node.transaction_data)?,
                            "payload_numbers": to_bson(&node.payload_numbers)?,
                        }
                    }
                ).await?;
            } else {
                let new_node = MatchNode {
                    _id: ObjectId::new(),
                    transaction_data: vec![mnt],
                    payload_numbers: vec![payload_number.clone()],
                    matcher: field.matcher.clone(),
                    value: field.value.clone(),
                    confidence: conf,
                    importance: imp,
                };
                self.match_nodes().insert_one(new_node).await?;
            }
        }

        debug!(
            "Successfully saved {} matching fields for transaction {}",
            matching_fields.len(),
            transaction_id
        );
        Ok(())
    }

    async fn get_scoring_events(
        &self,
        transaction_id: ObjectId,
    ) -> Result<Vec<ScoringEvent>, GenericError> {
        let mut cursor = self
            .scoring_events()
            .find(doc! { "transaction_id": transaction_id })
            .await?;
        let mut events = Vec::new();
        while let Some(doc) = cursor.try_next().await? {
            events.push(doc);
        }
        Ok(events)
    }

    async fn label_transactions(
        &self,
        payload_numbers: &[String],
        fraud_level: &FraudLevel,
        fraud_category: &String,
        label_source: &LabelSource,
        labeled_by: &String,
    ) -> Result<(), GenericError> {

        let latest = self
            .transactions()
            .find(doc! { "payload_number": { "$in": payload_numbers }, "is_latest": true })
            .await?;

        self.transactions()
            .update_many(
                doc! { "payload_number": { "$in": payload_numbers }, "is_latest": true},
                doc! { "$set": { "is_latest": false } },
            )
            .await?;

        let now = Utc::now().naive_utc();
        let new_txs = latest.map(|tx| {
                let mut new_tx = tx.expect("Transaction cannot be retrieved").clone();
                new_tx._id = ObjectId::new();
                new_tx.label = Some(Label {
                    fraud_level: *fraud_level,
                    fraud_category: fraud_category.clone(),
                    label_source: label_source.clone(),
                    labeled_by: labeled_by.clone(),
                    created_at: Utc::now().naive_utc(),
                });
                new_tx.is_latest = true;
                new_tx.updated_at = now;
                new_tx
            })
            .collect::<Vec<_>>()
            .await;

        self.transactions().insert_many(new_txs).await?;


        Ok(())
    }
}

impl MongoCommonStorage {
    /// Build connected transactions from aggregated graph results.
    fn collect_connections(
        root_payload: &str,
        agg_results: Vec<AggregatedGraph>,
        limit_count: Option<i32>,
    ) -> Vec<ConnectedTransaction> {
        let mut connections: HashMap<String, ConnectedTransaction> = HashMap::new();

        let mut process_node = |matcher: &str, confidence: i32, payloads: &[String]| {
            for target in payloads {
                if target == root_payload {
                    continue;
                }

                let parent_opt = payloads
                    .iter()
                    .filter(|p| *p != target)
                    .filter_map(|p| connections.get(p))
                    .max_by(|a, b| a.total_confidence.cmp(&b.total_confidence));

                let (base_path, base_conf) = if let Some(parent) = parent_opt {
                    (parent.path.clone(), parent.total_confidence)
                } else {
                    (Vec::new(), 100)
                };

                let mut new_path = base_path;
                new_path.push(matcher.to_string());
                let new_conf = (base_conf * confidence) / 100;

                connections
                    .entry(target.clone())
                    .and_modify(|existing| {
                        if new_conf > existing.total_confidence
                            || (new_conf == existing.total_confidence
                                && new_path.len() < existing.path.len())
                        {
                            existing.path = new_path.clone();
                            existing.total_confidence = new_conf;
                        }
                    })
                    .or_insert(ConnectedTransaction {
                        payload_number: target.clone(),
                        path: new_path.clone(),
                        total_confidence: new_conf,
                    });
            }
        };

        for agg in agg_results {
            process_node(&agg.matcher, agg.confidence, &agg.payload_numbers);

            let mut graph_nodes = agg.graph.clone();
            graph_nodes.sort_by(|a, b| a.depth.cmp(&b.depth));
            for node in graph_nodes {
                process_node(&node.matcher, node.confidence, &node.payload_numbers);
            }
        }

        let mut result: Vec<ConnectedTransaction> = connections.into_values().collect();
        result.sort_by(|a, b| {
            b.total_confidence
                .cmp(&a.total_confidence)
                .then_with(|| a.payload_number.cmp(&b.payload_number))
        });

        if let Some(limit) = limit_count {
            result.truncate(limit as usize);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_connections_builds_paths_and_confidence() {
        let agg = AggregatedGraph {
            matcher: "email".to_string(),
            confidence: 90,
            payload_numbers: vec!["A".to_string(), "B".to_string()],
            graph: vec![GraphNode {
                matcher: "device".to_string(),
                confidence: 80,
                payload_numbers: vec!["B".to_string(), "C".to_string()],
                depth: 1,
            }],
        };

        let result = MongoCommonStorage::collect_connections("A", vec![agg], Some(10));
        assert_eq!(result.len(), 2);

        let b = result
            .iter()
            .find(|c| c.payload_number == "B")
            .expect("B missing");
        assert_eq!(b.path, vec!["email"]);
        assert_eq!(b.total_confidence, 90);

        let c = result
            .iter()
            .find(|c| c.payload_number == "C")
            .expect("C missing");
        assert_eq!(c.path, vec!["email", "device"]);
        assert_eq!(c.total_confidence, 72); // 90 * 80 / 100
    }
}

/*

[
  {
    _id: ObjectId('696914fd19cac2e1bac55223'),
    txs: [
      ObjectId('6968e549633450d688293ba8'),
      ObjectId('6968e59e633450d68846cb01'),
      ObjectId('6968e5cf633450d6885afd5d')
    ],
    value: [ '434418996312' ],
    matcher: 'billingIdentity.phoneNumbers.phoneNumber',
    confidence: 94,
    graph: [
      {
        _id: ObjectId('696a4fa119cac2e1ba521969'),
        confidence: 100,
        matcher: 'customerAccount.sourceId',
        txs: [
          ObjectId('6968e5b1633450d6884f0fe0'),
          ObjectId('6968e5b1633450d6884f0fff'),
          ObjectId('6968e5b1633450d6884f101f')
        ],
        value: 'ad26368c-0a0f-4c7a-bf4c-cf19e263b343',
        depth: Long('0')
      },
      {
        _id: ObjectId('696a4e5f19cac2e1bafac2ce'),
        confidence: 95,
        matcher: 'billingIdentity.emailAddress.email',
        txs: [ ObjectId('6968e549633450d6882923e8') ],
        value: '46312853-6b0f-4585-aa1a-d35f25bbee4f@yahoo.de',
        depth: Long('0')
      }
    ]
  },





  {
    _id: ObjectId('696a4ead19cac2e1ba1d0c1c'),
    confidence: 95,
    matcher: 'billingIdentity.emailAddress.email',
    txs: [
      ObjectId('6968e5b1633450d6884f1bf5'),
      ObjectId('6968e5bd633450d68853b0da'),
      ObjectId('6968e547633450d688288758')
    ],
    value: 'd36457b2-eb5a-45f2-b043-1561743fe11a@yahoo.de',
    graph: [
      {
        _id: ObjectId('696a4fa119cac2e1ba521969'),
        confidence: 100,
        matcher: 'customerAccount.sourceId',
        txs: [
          ObjectId('6968e5b1633450d6884f0fe0'),
          ObjectId('6968e5b1633450d6884f0fff'),
          ObjectId('6968e5b1633450d6884f101f')
        ],
        value: 'ad26368c-0a0f-4c7a-bf4c-cf19e263b343',
        depth: Long('0')
      }
    ]
  },
  {
    _id: ObjectId('696a4f5c19cac2e1ba322ed1'),
    confidence: 100,
    matcher: 'customerAccount.sourceId',
    txs: [
      ObjectId('6968e5a8633450d6884b2652'),
      ObjectId('6968e591633450d688425d01'),
      ObjectId('6968e548633450d688291d6c')
    ],
    value: '2a34116c-12e3-4d79-8fcb-8f2dd02bb4e1',
    graph: [
      {
        _id: ObjectId('696a4e5f19cac2e1bafac2ce'),
        confidence: 95,
        matcher: 'billingIdentity.emailAddress.email',
        txs: [ ObjectId('6968e549633450d6882923e8') ],
        value: '46312853-6b0f-4585-aa1a-d35f25bbee4f@yahoo.de',
        depth: Long('0')
      }
    ]
  },
  {
    _id: ObjectId('696a54c019cac2e1ba761358'),
    confidence: 80,
    matcher: 'deviceData.smartId',
    txs: [
      ObjectId('6968e5a8633450d6884b4af1'),
      ObjectId('6968e5b6633450d68850b39e'),
      ObjectId('6968e5e7633450d68865d80f')
    ],
    value: 'AITgGp2e7HXbJCzZwQ963Go_E96M',
    graph: [
      {
        _id: ObjectId('696a4e5f19cac2e1bafac2ce'),
        confidence: 95,
        matcher: 'billingIdentity.emailAddress.email',
        txs: [ ObjectId('6968e549633450d6882923e8') ],
        value: '46312853-6b0f-4585-aa1a-d35f25bbee4f@yahoo.de',
        depth: Long('0')
      },
      {
        _id: ObjectId('696a4f7319cac2e1ba3d7723'),
        confidence: 100,
        matcher: 'customerAccount.sourceId',
        txs: [
          ObjectId('6968e613633450d68874e486'),
          ObjectId('6968e5f4633450d6886a9397'),
          ObjectId('6968e60c633450d6887314c2')
        ],
        value: '58740c03-c13f-45cf-a0b1-547aaf21f34b',
        depth: Long('0')
      },
      {
        _id: ObjectId('696a4e4119cac2e1baeaf0f0'),
        confidence: 95,
        matcher: 'billingIdentity.emailAddress.email',
        txs: [
          ObjectId('6968e5e5633450d68864f030'),
          ObjectId('6968e5e7633450d68865cf4e'),
          ObjectId('6968e5e5633450d68864d352')
        ],
        value: '051ba10b-318d-4785-bd98-e9c46caa4eb6@yahoo.de',
        depth: Long('0')
      },
      {
        _id: ObjectId('696a590a19cac2e1bae555ac'),
        confidence: 90,
        matcher: 'deviceData.exactId',
        txs: [ ObjectId('6968e5c8633450d688582468') ],
        value: 'AQHQqj56w2kXRJCiXTAHUAAirrwQ',
        depth: Long('0')
      },
      {
        _id: ObjectId('696a4f8119cac2e1ba43621a'),
        confidence: 100,
        matcher: 'customerAccount.sourceId',
        txs: [ ObjectId('6968e5c8633450d688582468') ],
        value: '70bf7b42-4ef9-4a3d-95c8-460e96bb7736',
        depth: Long('0')
      },
      {
        _id: ObjectId('696a4e5a19cac2e1baf8357d'),
        confidence: 95,
        matcher: 'billingIdentity.emailAddress.email',
        txs: [ ObjectId('6968e5c8633450d688582468') ],
        value: '3bafc8eb-a688-44c8-81ec-257cd82be417@online.de',
        depth: Long('0')
      },
      {
        _id: ObjectId('696a4fa119cac2e1ba521969'),
        confidence: 100,
        matcher: 'customerAccount.sourceId',
        txs: [
          ObjectId('6968e5b1633450d6884f0fe0'),
          ObjectId('6968e5b1633450d6884f0fff'),
          ObjectId('6968e5b1633450d6884f101f')
        ],
        value: 'ad26368c-0a0f-4c7a-bf4c-cf19e263b343',
        depth: Long('0')
      },
      {
        _id: ObjectId('6969150b19cac2e1bae6334b'),
        txs: [ ObjectId('6968e5c8633450d688582468') ],
        value: [ '94649970503' ],
        matcher: 'billingIdentity.phoneNumbers.phoneNumber',
        confidence: 94,
        depth: Long('0')
      },
      {
        _id: ObjectId('6969150919cac2e1bae2dab7'),
        txs: [
          ObjectId('6968e5e7633450d68865cd46'),
          ObjectId('6968e5f9633450d6886c7b40'),
          ObjectId('6968e614633450d688755c35'),
          ObjectId('6968e5e2633450d68863b392')
        ],
        value: [ '894251014735' ],
        matcher: 'billingIdentity.phoneNumbers.phoneNumber',
        confidence: 94,
        depth: Long('0')
      }
    ]
  },
  {
    _id: ObjectId('696a565519cac2e1baa9efd7'),
    confidence: 80,
    matcher: 'deviceData.smartId',
    txs: [
      ObjectId('6968e611633450d68874a920'),
      ObjectId('6968e5e7633450d68865fa16'),
      ObjectId('6968e587633450d6883f03fe')
    ],
    value: 'AITgGp2e7HXbJCzZwQ963Go_E96M',
    graph: [
      {
        _id: ObjectId('696a4e5a19cac2e1baf8357d'),
        confidence: 95,
        matcher: 'billingIdentity.emailAddress.email',
        txs: [ ObjectId('6968e5c8633450d688582468') ],
        value: '3bafc8eb-a688-44c8-81ec-257cd82be417@online.de',
        depth: Long('0')
      },
      {
        _id: ObjectId('696a4e5f19cac2e1bafac2ce'),
        confidence: 95,
        matcher: 'billingIdentity.emailAddress.email',
        txs: [ ObjectId('6968e549633450d6882923e8') ],
        value: '46312853-6b0f-4585-aa1a-d35f25bbee4f@yahoo.de',
        depth: Long('0')
      },
      {
        _id: ObjectId('696a4f7319cac2e1ba3d7723'),
        confidence: 100,
        matcher: 'customerAccount.sourceId',
        txs: [
          ObjectId('6968e613633450d68874e486'),
          ObjectId('6968e5f4633450d6886a9397'),
          ObjectId('6968e60c633450d6887314c2'),
          ObjectId('6968e5e8633450d688666464')
        ],
        value: '58740c03-c13f-45cf-a0b1-547aaf21f34b',
        depth: Long('0')
      },
      {
        _id: ObjectId('696a4e4119cac2e1baeaf0f0'),
        confidence: 95,
        matcher: 'billingIdentity.emailAddress.email',
        txs: [
          ObjectId('6968e5e5633450d68864f030'),
          ObjectId('6968e5e7633450d68865cf4e'),
          ObjectId('6968e5e5633450d68864d352'),
          ObjectId('6968e5e9633450d688668e7c')
        ],
        value: '051ba10b-318d-4785-bd98-e9c46caa4eb6@yahoo.de',
        depth: Long('0')
      },
      {
        _id: ObjectId('696a590a19cac2e1bae555ac'),
        confidence: 90,
        matcher: 'deviceData.exactId',
        txs: [ ObjectId('6968e5c8633450d688582468') ],
        value: 'AQHQqj56w2kXRJCiXTAHUAAirrwQ',
        depth: Long('0')
      },
      {
        _id: ObjectId('696a4f8119cac2e1ba43621a'),
        confidence: 100,
        matcher: 'customerAccount.sourceId',
        txs: [ ObjectId('6968e5c8633450d688582468') ],
        value: '70bf7b42-4ef9-4a3d-95c8-460e96bb7736',
        depth: Long('0')
      },
      {
        _id: ObjectId('6969150b19cac2e1bae6334b'),
        txs: [ ObjectId('6968e5c8633450d688582468') ],
        value: [ '94649970503' ],
        matcher: 'billingIdentity.phoneNumbers.phoneNumber',
        confidence: 94,
        depth: Long('0')
      },
      {
        _id: ObjectId('696a4fa119cac2e1ba521969'),
        confidence: 100,
        matcher: 'customerAccount.sourceId',
        txs: [
          ObjectId('6968e5b1633450d6884f0fe0'),
          ObjectId('6968e5b1633450d6884f0fff'),
          ObjectId('6968e5b1633450d6884f101f')
        ],
        value: 'ad26368c-0a0f-4c7a-bf4c-cf19e263b343',
        depth: Long('0')
      },
      {
        _id: ObjectId('6969150919cac2e1bae2dab7'),
        txs: [
          ObjectId('6968e5e7633450d68865cd46'),
          ObjectId('6968e5f9633450d6886c7b40'),
          ObjectId('6968e614633450d688755c35')
        ],
        value: [ '894251014735' ],
        matcher: 'billingIdentity.phoneNumbers.phoneNumber',
        confidence: 94,
        depth: Long('0')
      }
    ]
  },
  {
    _id: ObjectId('696a593019cac2e1baf50899'),
    confidence: 90,
    matcher: 'deviceData.exactId',
    txs: [
      ObjectId('6968e614633450d68875561e'),
      ObjectId('6968e5f4633450d6886a917c'),
      ObjectId('6968e5e7633450d68865d6bb')
    ],
    value: 'Ahcja4PW3WKuPNBxZyxW3FUXTU6o',
    graph: [
      {
        _id: ObjectId('696a4e5f19cac2e1bafac2ce'),
        confidence: 95,
        matcher: 'billingIdentity.emailAddress.email',
        txs: [ ObjectId('6968e549633450d6882923e8') ],
        value: '46312853-6b0f-4585-aa1a-d35f25bbee4f@yahoo.de',
        depth: Long('0')
      },
      {
        _id: ObjectId('696a4f7319cac2e1ba3d7723'),
        confidence: 100,
        matcher: 'customerAccount.sourceId',
        txs: [
          ObjectId('6968e613633450d68874e486'),
          ObjectId('6968e5f4633450d6886a9397'),
          ObjectId('6968e60c633450d6887314c2')
        ],
        value: '58740c03-c13f-45cf-a0b1-547aaf21f34b',
        depth: Long('0')
      },
      {
        _id: ObjectId('696a4e4119cac2e1baeaf0f0'),
        confidence: 95,
        matcher: 'billingIdentity.emailAddress.email',
        txs: [
          ObjectId('6968e5e5633450d68864f030'),
          ObjectId('6968e5e7633450d68865cf4e'),
          ObjectId('6968e5e5633450d68864d352')
        ],
        value: '051ba10b-318d-4785-bd98-e9c46caa4eb6@yahoo.de',
        depth: Long('0')
      },
      {
        _id: ObjectId('696a4fa119cac2e1ba521969'),
        confidence: 100,
        matcher: 'customerAccount.sourceId',
        txs: [
          ObjectId('6968e5b1633450d6884f0fe0'),
          ObjectId('6968e5b1633450d6884f0fff'),
          ObjectId('6968e5b1633450d6884f101f')
        ],
        value: 'ad26368c-0a0f-4c7a-bf4c-cf19e263b343',
        depth: Long('0')
      },
      {
        _id: ObjectId('6969150919cac2e1bae2dab7'),
        txs: [
          ObjectId('6968e5e7633450d68865cd46'),
          ObjectId('6968e5f9633450d6886c7b40')
      
        ],
        value: [ '894251014735' ],
        matcher: 'billingIdentity.phoneNumbers.phoneNumber',
        confidence: 94,
        depth: Long('0')
      }
    ]
  }
]

   */