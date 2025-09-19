#!/usr/bin/env bash
set -euo pipefail

BASE_URL="http://127.0.0.1:3000"

echo "1) Health check"
curl -sS ${BASE_URL}/health | jq . || true

echo "\n2) Create user"
CREATE_RES=$(curl -sS -X POST ${BASE_URL}/users \
  -H 'Content-Type: application/json' \
  -d '{"name":"Ada Lovelace","email":"ada@lovelace.org"}')

echo "$CREATE_RES" | jq . || true
USER_ID=$(echo "$CREATE_RES" | jq -r .id)

echo "\n3) Get user by id"
curl -sS ${BASE_URL}/users/${USER_ID} | jq . || true

echo "\n4) Error: invalid email"
curl -sS -X POST ${BASE_URL}/users -H 'Content-Type: application/json' -d '{"name":"x","email":"invalid"}' | jq . || true

