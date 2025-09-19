# Rust Web Server 01

Production-ready minimal REST API demonstrating core Rust concepts: Immutability, Shadowing, Borrowing, Ownership, and Result.

## Endpoints
- GET / — Index listing available endpoints
- GET /health — Health check
- POST /users — Create user { name, email }
- GET /users/<id> — Get user by UUID

## Run
```
cargo run
```
Expected output:
```
🚀 Server running on http://127.0.0.1:3000
📋 Try these endpoints:
   GET  /health
   POST /users
   GET  /users/<id>
```

## Test
Use the provided script:
```
./test_api.sh
```

## Notes on Rust Concepts
- Immutability: server config bindings are immutable
- Shadowing: String -> Uuid transformation for path id
- Borrowing: validation function takes &str; DB is borrowed for read/write
- Ownership: request body owned by handler; response owns data returned
- Result: all fallible operations return Result with proper HTTP errors
