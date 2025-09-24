04-web-server — Axum + SQLx + Redis + Docker

# 🚀 04-web-server

Production-grade evolution of 03-web-server with real infrastructure:
- PostgreSQL via SQLx (async)
- Redis-backed hybrid JWT (whitelist jti)
- Docker multi-stage build and docker-compose orchestration
- Same API surface and handlers as 03-web-server

## Quickstart

1) Start stack with Docker

```bash
cd 04-web-server
docker-compose up -d --build
```

This starts:
- Postgres 15 (user/password = user/password, db = app)
- Redis 7-alpine
- The web app listening on :8080

2) Verify health

```bash
curl -i http://localhost:8080/healthz
```
- 200 with {"status":"ok"} when Postgres and Redis are healthy

3) Register → Login → Me

```bash
# Register
curl -s http://localhost:8080/auth/register \
  -H 'Content-Type: application/json' \
  -d '{"email":"demo@example.com","password":"Password1"}' | jq .

# Login (get token)
TOKEN=$(curl -s http://localhost:8080/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"email":"demo@example.com","password":"Password1"}' | jq -r .token)

echo $TOKEN

# Me
curl -s http://localhost:8080/auth/me -H "Authorization: Bearer $TOKEN" | jq .
```

4) Logout (revokes via Redis)

```bash
# Optional helper route not exposed; use Redis DEL manually or extend handler to call auth.logout
```

## Environment Variables

See .env.example for defaults. Important ones:
- HOST (default 0.0.0.0)
- PORT (default 8080)
- RUST_LOG (default info)
- JWT_SECRET (required, >= 32 chars)
- JWT_EXPIRY_HOURS (default 24)
- DATABASE_URL (default postgres://user:password@localhost:5432/app)
- DB_MAX_CONNECTIONS (default 20)
- REDIS_URL (default redis://127.0.0.1:6379)
- MAX_PAGE_SIZE (default 100)
- BATCH_LIMIT (default 8)

## Migrations

Migrations are located in migrations/ and run automatically on startup via sqlx::migrate!("./migrations").

Schema:
- users(id uuid PK, email text unique, password_hash text, created_at timestamptz, status text)

## Architecture Notes

- Repository pattern swapped to Postgres with SQLx.
- Authentication uses HS256 JWTs with jti embedded and whitelisted in Redis (key: jwt:{jti}, TTL = exp-iat). Validation checks signature, expiry, and Redis presence. Logout removes key.
- Handlers and API shapes are kept identical to 03-web-server.
- Health endpoint checks both Postgres and Redis; returns 200 only if both are OK, otherwise 503 with details.

## Development

```bash
# Local without Docker (requires running Postgres & Redis)
export JWT_SECRET="change-me-32bytes-min-change-me-32bytes-min"
export DATABASE_URL="postgres://user:password@localhost:5432/app"
export REDIS_URL="redis://127.0.0.1:6379"
cargo run -p web_server_04 --bin 04-web-server
```

## Tests

Unit tests can be added around auth claims and password hashing. Integration tests can use the same pattern as 03-web-server but require Postgres & Redis; for simplicity use docker-compose for local verification.
