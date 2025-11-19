#!/usr/bin/env bash
set -euo pipefail

# Importer base URL. Override by setting IMPORTER_URL env var.
# Example: IMPORTER_URL=http://localhost:3000 ./examples/import.sh
IMPORTER_URL="${IMPORTER_URL:-http://localhost:3000}"

post() {
  local body="$1"
  echo "POST ${IMPORTER_URL}/import"
  echo "${body}" | curl -sS -X POST "${IMPORTER_URL}/import" \
    -H "Content-Type: application/json" \
    -d @- || true
  echo
  echo "-----"
}

# 1
post '{
  "order_number": "ORD-1001",
  "created_at": "2025-01-01T10:00:00Z",
  "items": [
    {"name": "Wireless Mouse", "category": "electronics", "price": 29.99},
    {"name": "USB-C Cable", "category": "accessories", "price": 9.99}
  ],
  "customer": {"name": "Alice Johnson", "email": "alice@example.com"},
  "billing": {
    "payment_type": "card",
    "payment_details": "**** **** **** 4242",
    "billing_address": "123 Main St, Springfield"
  },
  "delivery_type": "standard",
  "delivery_details": "UPS Ground"
}'

# 2
post '{
  "order_number": "ORD-1002",
  "created_at": "2025-01-02T14:25:00Z",
  "items": [
    {"name": "Laptop Sleeve", "category": "accessories", "price": 19.50}
  ],
  "customer": {"name": "Bob Smith", "email": "bob@example.com"},
  "billing": {
    "payment_type": "paypal",
    "payment_details": "txn_9f83k2",
    "billing_address": "456 Oak Ave, Metropolis"
  },
  "delivery_type": "express",
  "delivery_details": "DHL Express"
}'

# 3
post '{
  "order_number": "ORD-1003",
  "created_at": "2025-01-03T09:15:30Z",
  "items": [
    {"name": "Smartphone", "category": "electronics", "price": 799.00},
    {"name": "Screen Protector", "category": "accessories", "price": 12.00}
  ],
  "customer": {"name": "Charlie Baker", "email": "charlie@example.com"},
  "billing": {
    "payment_type": "card",
    "payment_details": "**** **** **** 1111",
    "billing_address": "789 Pine Rd, Gotham"
  },
  "delivery_type": "express",
  "delivery_details": "FedEx Priority"
}'

# 4
post '{
  "order_number": "ORD-1004",
  "created_at": "2025-01-04T18:05:00Z",
  "items": [
    {"name": "Bluetooth Speaker", "category": "electronics", "price": 59.95}
  ],
  "customer": {"name": "Dana Lee", "email": "dana@example.com"},
  "billing": {
    "payment_type": "card",
    "payment_details": "**** **** **** 2222",
    "billing_address": "1600 Elm St, Star City"
  },
  "delivery_type": "standard",
  "delivery_details": "USPS"
}'

# 5
post '{
  "order_number": "ORD-1005",
  "created_at": "2025-01-05T12:00:00Z",
  "items": [
    {"name": "Gaming Keyboard", "category": "electronics", "price": 129.99},
    {"name": "Mouse Pad", "category": "accessories", "price": 7.49}
  ],
  "customer": {"name": "Eve Torres", "email": "eve@example.com"},
  "billing": {
    "payment_type": "apple_pay",
    "payment_details": "applepay_abc123",
    "billing_address": "22 Baker St, London"
  },
  "delivery_type": "standard",
  "delivery_details": "Royal Mail"
}'

# 6
post '{
  "order_number": "ORD-1006",
  "created_at": "2025-01-06T07:45:00Z",
  "items": [
    {"name": "4K Monitor", "category": "electronics", "price": 349.99}
  ],
  "customer": {"name": "Frank Miller", "email": "frank@example.com"},
  "billing": {
    "payment_type": "card",
    "payment_details": "**** **** **** 3333",
    "billing_address": "742 Evergreen Terrace, Springfield"
  },
  "delivery_type": "freight",
  "delivery_details": "UPS Freight"
}'

# 7
post '{
  "order_number": "ORD-1007",
  "created_at": "2025-01-07T21:10:00Z",
  "items": [
    {"name": "Noise-Canceling Headphones", "category": "electronics", "price": 199.00}
  ],
  "customer": {"name": "Grace Hopper", "email": "grace@example.com"},
  "billing": {
    "payment_type": "google_pay",
    "payment_details": "gpay_xyz789",
    "billing_address": "1 Infinite Loop, Cupertino"
  },
  "delivery_type": "express",
  "delivery_details": "FedEx 2Day"
}'

# 8
post '{
  "order_number": "ORD-1008",
  "created_at": "2025-01-08T16:30:00Z",
  "items": [
    {"name": "External SSD 1TB", "category": "electronics", "price": 129.00},
    {"name": "USB Hub", "category": "accessories", "price": 24.99}
  ],
  "customer": {"name": "Hank Pym", "email": "hank@example.com"},
  "billing": {
    "payment_type": "bank_transfer",
    "payment_details": "IBAN DE89 3704 0044 0532 0130 00",
    "billing_address": "Unter den Linden 1, Berlin"
  },
  "delivery_type": "standard",
  "delivery_details": "DHL Paket"
}'

# 9
post '{
  "order_number": "ORD-1009",
  "created_at": "2025-01-09T11:20:00Z",
  "items": [
    {"name": "Action Camera", "category": "electronics", "price": 299.99}
  ],
  "customer": {"name": "Ivy Chen", "email": "ivy@example.com"},
  "billing": {
    "payment_type": "card",
    "payment_details": "**** **** **** 4444",
    "billing_address": "88 Queens Rd, Hong Kong"
  },
  "delivery_type": "standard",
  "delivery_details": "SF Express"
}'

# 10
post '{
  "order_number": "ORD-1010",
  "created_at": "2025-01-10T13:55:00Z",
  "items": [
    {"name": "Fitness Tracker", "category": "electronics", "price": 79.90},
    {"name": "Replacement Bands", "category": "accessories", "price": 14.90}
  ],
  "customer": {"name": "John Doe", "email": "john.doe@example.com"},
  "billing": {
    "payment_type": "card",
    "payment_details": "**** **** **** 5555",
    "billing_address": "500 Market St, San Francisco"
  },
  "delivery_type": "pickup",
  "delivery_details": "Store #12"
}'

# 11 - shares email with ORD-1001 (alice@example.com) to create a connection
post '{
  "order_number": "ORD-1011",
  "created_at": "2025-01-11T09:00:00Z",
  "items": [
    {"name": "USB Charger", "category": "electronics", "price": 15.99}
  ],
  "customer": {"name": "Alice J.", "email": "alice@example.com"},
  "billing": {
    "payment_type": "card",
    "payment_details": "**** **** **** 7777",
    "billing_address": "999 Side St, Springfield"
  },
  "delivery_type": "standard",
  "delivery_details": "UPS Ground"
}'

# 12 - shares payment_details with ORD-1003 (**** **** **** 1111) to create a connection
post '{
  "order_number": "ORD-1012",
  "created_at": "2025-01-12T12:10:00Z",
  "items": [
    {"name": "Phone Case", "category": "accessories", "price": 19.99}
  ],
  "customer": {"name": "Chris Pine", "email": "cpine@example.com"},
  "billing": {
    "payment_type": "card",
    "payment_details": "**** **** **** 1111",
    "billing_address": "101 Center Blvd, Gotham"
  },
  "delivery_type": "standard",
  "delivery_details": "USPS"
}'

# 13 - shares billing_address with ORD-1004 (1600 Elm St, Star City) to create a connection
post '{
  "order_number": "ORD-1013",
  "created_at": "2025-01-13T17:45:00Z",
  "items": [
    {"name": "Portable Speaker", "category": "electronics", "price": 39.95}
  ],
  "customer": {"name": "D. Lee", "email": "dlee@example.com"},
  "billing": {
    "payment_type": "card",
    "payment_details": "**** **** **** 9999",
    "billing_address": "1600 Elm St, Star City"
  },
  "delivery_type": "standard",
  "delivery_details": "USPS"
}'

echo "Done."


