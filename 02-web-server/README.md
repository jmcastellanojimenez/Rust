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
- Immutability
  - Bindings like `database`, `app`, and `addr` are created with `let` and never mutated. This is the default in Rust and helps with thread-safety and reasoning about state.
  - Example in `main`: the `Router` and `SocketAddr` are set once and then used without mutation.

- Shadowing
  - Reusing a variable name to transform its value while keeping a clear, single identifier in scope.
  - In `get_user`: the `id` from the URL path is a `String`, then it’s shadowed with a parsed `Uuid` using `let id = Uuid::parse_str(&id)?;`. This keeps the name `id` but changes its type and meaning after validation.
  - In `create_user`: inputs are normalized (e.g., `let name = request.name.trim().to_string();`) so later code deals with the cleaned value under the same name.

- Borrowing
  - Functions can take references to avoid taking ownership. `validate_email(email: &str)` accepts a string slice so callers don’t have to clone or move their `String`.
  - The in-memory DB is an `Arc<Mutex<HashMap<Uuid, User>>>`. Handlers borrow the DB via `State(database)` and then borrow the inner map for the shortest necessary scope:
    - Read: acquire the lock, read, then drop the lock before building the response.
    - Write: acquire the lock, insert, then drop the lock. Keeping lock scopes tight prevents contention.

- Ownership
  - The request body is owned by the handler: `Json(request)` transfers ownership into `create_user`, allowing transformations and moves without extra clones.
  - Inserting into the `HashMap` moves the `User` into the map. When returning a response, the code uses `clone()` so the map keeps its copy while the response owns its own copy. Returning owned data avoids dangling references.

- Result and error mapping
  - Fallible operations return `Result` and are mapped to HTTP errors close to where they can fail:
    - `validate_email(&str) -> Result<&str, String>` for input validation.
    - `Uuid::parse_str(&str) -> Result<Uuid, _>` for ID parsing.
    - `database.lock()` may fail; errors are converted to `(StatusCode, Json<ErrorResponse>)`.
  - Handlers use `Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)>` so happy paths and error paths are explicit, and each error maps to an appropriate status code (400 invalid input, 404 not found, 500 internal errors).
