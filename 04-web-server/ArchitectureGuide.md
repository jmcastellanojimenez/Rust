# 04-web-server — Production Architecture Guide

This service evolves 03-web-server to a production-grade stack:
- PostgreSQL (SQLx) for persistence and pooling
- Redis-backed Hybrid JWT (whitelist jti) for stateless auth with revocation
- Docker (multi-stage) + docker-compose for local prod-like runs

The HTTP API surface and handler signatures remain the same as 03-web-server; only infrastructure and wiring changed.

## 1) High-Level Architecture

Request flow:
Client → Axum Router → Middleware (trace/CORS) → Handlers → (Auth + Repo)
- Persistence: Postgres via SQLx pool
- Auth: HS256 JWT + Redis whitelist (jti with TTL)
- Observability: tower-http TraceLayer
- Runtime: Tokio multi-threaded

Key improvements over 03:
- In-memory HashMap → Postgres (durability, concurrency, transactions)
- Stateless-only JWT → Hybrid JWT (instant logout/revocation, TTL auto-cleanup)
- Local-only → Dockerized stack (Postgres, Redis, App) with healthchecks

## 2) Modules and Responsibilities

- src/main.rs
  - Boot: load AppConfig (env), init tracing
  - Wire: Postgres PgPool, Redis client, Auth service, Repo
  - Migrate: sqlx::migrate! at startup
  - Serve: Axum router with graceful shutdown

- src/config.rs
  - AppConfig + validation (no dotenv in prod)
  - ServerConfig, JwtConfig, DatabaseConfig, RedisConfig
  - Sensible defaults: PORT=8080, JWT_EXPIRY_HOURS=24, DB_MAX_CONNECTIONS=20, REDIS_URL=redis://127.0.0.1:6379
  - Security check: JWT_SECRET length >= 32

- src/repository.rs
  - Trait: UserRepository { create, find_by_id, find_by_email, list, stats }
  - Impl: PostgresUserRepository { pool: PgPool } using SQLx queries
  - Pagination: ListOptions::clamp
  - Aggregates: UserStats

- src/auth.rs
  - Jwt/Hybrid service (HS256):
    - generate_token: creates Claims { sub, iat, exp, jti }, writes jwt:{jti} to Redis with TTL=exp
    - validate_token: verify signature+exp and EXISTS jwt:{jti}
    - logout: DEL jwt:{jti}
  - Passwords: bcrypt via spawn_blocking
  - Helper: bearer token extraction

- src/handlers.rs
  - Same routes as 03:
    - POST /auth/register
    - POST /auth/login
    - GET  /auth/me
    - GET  /users
    - GET  /users/stats
    - POST /users/batch
    - GET  /healthz
  - Differences under the hood: repository now uses Postgres; auth checks Redis on every protected request

- migrations/
  - 001_create_users.sql: users(id uuid pk, email unique, password_hash, created_at timestamptz default now(), status text)
  - Optional indexes (email)

## 3) Runtime and Infrastructure

- PostgreSQL
  - sqlx PgPool (configurable max_connections)
  - Migrations run on boot; SELECT 1 used for health checks
  - Queries are async, prepared; errors mapped to AppError

- Redis
  - Async client
  - JWT whitelist (EXISTS jwt:{jti}); TTL matches token expiry
  - Health check via PING

- Docker
  - Multi-stage Dockerfile (builder → slim runtime)
  - docker-compose: postgres, redis, app
  - .env.example with all required variables
  - Healthchecks: pg_isready, redis-cli ping

## 4) Security Model

- JWT: HS256 with strong secret (>= 32 bytes)
- Hybrid revocation:
  - Login: create token + store jti in Redis with TTL
  - Request: verify signature + check jti exists
  - Logout: delete jti → immediate revocation
- Passwords: bcrypt hashed/verified via spawn_blocking
- CORS: explicit allowlist; adjust in main.rs as needed

## 5) Health, Observability, and Resilience

- /healthz:
  - OK if Postgres SELECT 1 and Redis PING both succeed
  - 503 with details on failure
- Tracing: tower-http TraceLayer + RUST_LOG env filter
- Graceful shutdown: SIGTERM/Ctrl+C handling

## 6) Configuration (env)

- SERVER
  - HOST=0.0.0.0
  - PORT=8080
- JWT
  - JWT_SECRET=change-me-32bytes-min
  - JWT_EXPIRY_HOURS=24
- DATABASE
  - DATABASE_URL=postgres://user:password@postgres:5432/app
  - DB_MAX_CONNECTIONS=20
- REDIS
  - REDIS_URL=redis://redis:6379
- APP
  - MAX_PAGE_SIZE=100
  - BATCH_LIMIT=8
  - RUST_LOG=info

## 7) Request Lifecycles (Representative)

- Login
  1. Verify credentials against Postgres (bcrypt verify)
  2. Create Claims { sub, iat, exp, jti } and sign (HS256)
  3. SETEX jwt:{jti} with TTL=exp
  4. Return token

- Protected route (/auth/me)
  1. Extract bearer token
  2. Verify signature + exp
  3. EXISTS jwt:{jti} in Redis (if missing → unauthorized)
  4. Load user by sub from Postgres
  5. Return DTO

- Logout
  1. Extract/verify token
  2. DEL jwt:{jti}
  3. Return OK

## 8) Why This Architecture

- Durability & scale: Postgres replaces ephemeral HashMap
- Stateless + control: JWT stays stateless, Redis enables revocation
- Operability: Dockerized stack mirrors production; health checks and tracing included
- Backwards-compatible: API unchanged; infra swappable (DI via traits)

## 9) Quickstart

- With Docker
  - docker-compose up -d
  - export JWT_SECRET="dev-secret-change-me-but-32+"
  - curl http://localhost:8080/healthz
  - Register/Login/Me as in 03-web-server

- Without Docker
  - Start Postgres + Redis locally
  - Set env vars per section 6
  - cargo run -p 04-web-server

## 10) Extension Paths

- RS256 tokens (JWKS) for multi-service deployments
- Role-based access control in Claims
- Rate limiting via Redis counters
- SQLx offline builds for containerized CI
- Observability: Prometheus metrics + OpenTelemetry tracing
