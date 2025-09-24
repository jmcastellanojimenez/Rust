# Complete Day 2 Web Server Architecture Guide
*Basic Axum Foundation - Learning the Fundamentals*

This guide reflects the actual implementation from your Day 2 learning session - a minimal but complete Axum web server that demonstrates core concepts.

## Table of Contents
1. [Anatomy of a Basic Axum Application](#anatomy)
2. [File Structure and Learning Goals](#structure)
3. [Request Lifecycle](#lifecycle)
4. [Core Rust Patterns Demonstrated](#patterns)
5. [Foundation for Growth](#foundation)
6. [How to Run and Test](#usage)

---

## 1. Anatomy of a Basic Axum Application {#anatomy}

### Application Flow Overview
**Server → Router → Handlers → Extractors → State → Response → Runtime**

**Simple Restaurant Analogy:**
- **Server**: The building that opens for business
- **Router**: The menu that tells customers what's available
- **Handlers**: The cook who prepares each dish
- **Extractors**: The order-taking process
- **State**: The kitchen's shared ingredients and tools
- **Response**: The finished dish served to the customer
- **Runtime**: The electricity powering everything

### Core Components (Day 2 Implementation)

**Single File Structure** (`src/main.rs`):
```rust
// All logic in one file for learning clarity
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Database: shared in-memory storage
    let database: Database = Arc::new(Mutex::new(HashMap::new()));
    
    // Router: define what endpoints exist
    let app = Router::new()
        .route("/", get(index))
        .route("/health", get(health))
        .route("/users", post(create_user))
        .route("/users/:id", get(get_user))
        .with_state(database);
    
    // Server: bind and serve
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}
```

---

## 2. File Structure and Learning Goals {#structure}

### Why Single File?

**Educational Purpose**: Day 2 focused on understanding core concepts without the complexity of multiple modules. This approach lets you see:
- How all pieces fit together in one view
- The minimal code needed for a working web server
- Clear cause-and-effect relationships between components

### What's Actually Implemented

**Domain Types:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: Uuid,
    name: String,
    email: String,
}

#[derive(Debug, Deserialize)]
struct CreateUserRequest {
    name: String,
    email: String,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}
```

**Database Abstraction:**
```rust
type Database = Arc<Mutex<HashMap<Uuid, User>>>;
```

**API Endpoints:**
- `GET /` - API index showing available endpoints
- `GET /health` - Health check returning `{"status": "ok"}`
- `POST /users` - Create new user from JSON
- `GET /users/:id` - Fetch user by UUID

---

## 3. Request Lifecycle {#lifecycle}

### Step-by-Step Flow (As Implemented)

**1. Server Startup:**
```rust
// Bind to localhost:3000
let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
let listener = tokio::net::TcpListener::bind(addr).await?;
```

**2. Router Matching:**
```rust
// Axum matches HTTP method + path to handler function
.route("/users", post(create_user))  // POST /users → create_user()
.route("/users/:id", get(get_user))  // GET /users/123 → get_user()
```

**3. Handler Execution:**
```rust
// Handler receives typed extractors
async fn create_user(
    State(database): State<Database>,        // Shared state
    Json(request): Json<CreateUserRequest>   // Parsed JSON body
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Business logic here
}
```

**4. State Access:**
```rust
// Thread-safe access to shared data
let mut users = database.lock().unwrap();
users.insert(user.id, user.clone());
```

**5. Response Generation:**
```rust
// Return proper HTTP status + JSON
Ok((StatusCode::CREATED, Json(user)))
```

---

## 4. Core Rust Patterns Demonstrated {#patterns}

### Ownership and Borrowing in Action

**Moving Ownership:**
```rust
async fn create_user(
    State(database): State<Database>,        // Borrows shared state
    Json(request): Json<CreateUserRequest>   // Takes ownership of request body
) -> Result<impl IntoResponse, ...> {
    // request is owned by this function, no copying needed
    let user = User {
        id: Uuid::new_v4(),
        name: request.name,    // Move from request
        email: request.email,  // Move from request
    };
}
```

**Borrowing for Validation:**
```rust
fn validate_email(email: &str) -> bool {  // Borrows, doesn't take ownership
    email.contains('@') && email.contains('.')
}

// Usage
if !validate_email(&request.email) {  // Borrow the email field
    return Err(/* validation error */);
}
```

### Concurrency Safety with Arc<Mutex<>>

**Thread-Safe Shared State:**
```rust
// Multiple async tasks can safely access the same data
type Database = Arc<Mutex<HashMap<Uuid, User>>>;

// Usage in handler
let mut users = database.lock().unwrap();  // Exclusive access
users.insert(user.id, user.clone());      // Safe mutation
// Lock automatically released when `users` goes out of scope
```

**Why This Pattern:**
- `Arc`: Multiple handlers can share the same database
- `Mutex`: Ensures only one handler modifies data at a time
- Short lock scope: Prevents blocking other requests

### Result<T, E> Pattern for Error Handling

**Explicit Error Handling:**
```rust
async fn get_user(
    State(database): State<Database>,
    Path(id_str): Path<String>
) -> Result<Json<User>, (StatusCode, Json<ErrorResponse>)> {
    
    // Parse UUID - could fail
    let id = match Uuid::parse_str(&id_str) {
        Ok(id) => id,
        Err(_) => return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { error: "Invalid UUID".to_string() })
        )),
    };
    
    // Look up user - could fail
    let users = database.lock().unwrap();
    match users.get(&id) {
        Some(user) => Ok(Json(user.clone())),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse { error: "User not found".to_string() })
        )),
    }
}
```

### Async Programming Fundamentals

**Non-Blocking I/O:**
```rust
#[tokio::main]  // Tokio runtime manages async operations
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // This doesn't block - other handlers can run concurrently
    axum::serve(listener, app).await?;
    Ok(())
}
```

**Concurrent Request Handling:**
- Each incoming request spawns a new task
- Tasks can run concurrently without blocking each other
- Shared state (Database) is safely accessed via Mutex

---

## 5. Foundation for Growth {#foundation}

### What Day 2 Taught You

**Core Web Server Concepts:**
- HTTP request/response cycle
- Routing and path parameters
- JSON serialization/deserialization
- Shared state management
- Error handling and status codes

**Rust-Specific Patterns:**
- Async/await for non-blocking I/O
- Ownership and borrowing in web handlers
- Thread-safe concurrency with Arc<Mutex<>>
- Type-safe extractors and responses

### Evolution Path to Day 3 (03-web-server)

**Day 2 → Day 3 Progression:**

| Day 2 Concept | Day 3 Evolution |
|---------------|-----------------|
| Single file | Modular architecture (multiple files) |
| HashMap storage | Repository pattern with traits |
| Basic error handling | Comprehensive error types with thiserror |
| Simple validation | Business logic with domain models |
| Mutex for concurrency | RwLock for better read performance |
| Direct handlers | Middleware and authentication |

**Architecture Lessons:**
- Day 2: "Make it work" - understand the basics
- Day 3: "Make it right" - apply professional patterns

---

## 6. How to Run and Test {#usage}

### Running the Server

```bash
# Navigate to Day 2 project
cd 02-web-server

# Run the server
cargo run

# Server starts on http://127.0.0.1:3000
```

### Testing the API

**Health Check:**
```bash
curl http://127.0.0.1:3000/health
# Response: {"status":"ok"}
```

**Create User:**
```bash
curl -X POST http://127.0.0.1:3000/users \
  -H "Content-Type: application/json" \
  -d '{"name":"Alice","email":"alice@example.com"}'

# Response: {"id":"550e8400-e29b-41d4-a716-446655440000","name":"Alice","email":"alice@example.com"}
```

**Get User:**
```bash
# Use the UUID from the create response
curl http://127.0.0.1:3000/users/550e8400-e29b-41d4-a716-446655440000

# Response: {"id":"550e8400-e29b-41d4-a716-446655440000","name":"Alice","email":"alice@example.com"}
```

**Error Handling:**
```bash
# Invalid UUID
curl http://127.0.0.1:3000/users/invalid-id
# Response: 400 Bad Request {"error":"Invalid UUID"}

# User not found
curl http://127.0.0.1:3000/users/00000000-0000-0000-0000-000000000000
# Response: 404 Not Found {"error":"User not found"}
```

