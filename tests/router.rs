use std::net::SocketAddr;

use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{
        Request, StatusCode,
        header::{
            ACCESS_CONTROL_ALLOW_ORIGIN, ACCESS_CONTROL_REQUEST_METHOD, AUTHORIZATION,
            CACHE_CONTROL, ORIGIN,
        },
    },
};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use crudo::{Config, build_router, connect, prepare_database};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::{AnyPool, any::AnyPoolOptions};
use tower::ServiceExt;

const CONFIG: &str = r#"
[server]
prefix = "api"

[server.cors]
origins = ["http://localhost:8000"]

[server.limits]
requests = 0

[database]
url = "sqlite::memory:"

[altcha]
secret = "test-secret"
key_secret = "test-key-secret"
algorithm = "SHA-256"
cost = 1
max_number = 1

[[endpoints]]
method = "GET"
path = "/value"
action = "value"

[[endpoints]]
method = "POST"
path = "/object"
action = "object"

[[endpoints]]
method = "GET"
path = "/limited"
action = "value"

[endpoints.limits]
requests = 1
window_seconds = 60

[actions.value]
sql = "SELECT 'configured' AS value"
result = "one"

[actions.object]
sql = "SELECT ? AS value"
params = ["value"]
result = "one"
"#;

const OPTIONAL_AUTH_CONFIG: &str = r#"
[server.limits]
requests = 0

[database]
url = "sqlite::memory:"

[altcha]
secret = "test-secret"
key_secret = "test-key-secret"
algorithm = "SHA-256"
cost = 1
max_number = 1

[auth.bearer]
sql = "SELECT user_id FROM sessions WHERE token = ?"
owner = "user_id"

[[endpoints]]
method = "GET"
path = "/protected"
action = "protected"
auth = ["bearer"]
auth_optional = true
altcha = true
altcha_for_authenticated = false

[actions.protected]
sql = "SELECT 'ok' AS value"
result = "one"
"#;

const LIFECYCLE_CONFIG: &str = r#"
[server]
prefix = "api"

[server.limits]
requests = 0

[database]
url = "sqlite::memory:"

[auth.basic]
sql = "SELECT id, password FROM users WHERE email = ?"
owner = "id"
password = "password"

[auth.bearer]
sql = "SELECT user_id FROM sessions WHERE token = ? AND expires_at > unixepoch()"
owner = "user_id"

[[endpoints]]
method = "POST"
path = "/users"
action = "create_user"

[[endpoints]]
method = "POST"
path = "/tokens"
action = "create_token"
auth = ["basic"]

[[endpoints]]
method = "DELETE"
path = "/users/{id}"
action = "delete_user"
auth = ["bearer"]

[actions.create_user]
sql = "INSERT INTO users (email, password) VALUES (?, ?) RETURNING id, email"
params = ["email", "password"]
hash = ["password"]
result = "one"
status = 201

[actions.create_token]
sql = "INSERT INTO sessions (token, user_id, expires_at) VALUES (?, ?, unixepoch() + 60) RETURNING token"
params = ["$token", "$owner"]
result = "one"
status = 201
no_store = true

[actions.delete_user]
sql = "DELETE FROM users WHERE id = ? AND id = ? RETURNING id"
params = ["id", "$owner"]
result = "optional"
"#;

async fn app() -> axum::Router {
    sqlx::any::install_default_drivers();
    let pool: AnyPool = AnyPoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    build_router(pool, Config::parse(CONFIG).unwrap()).unwrap()
}

fn request(method: &str, uri: &str, body: Body) -> Request<Body> {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .body(body)
        .unwrap();
    request.extensions_mut().insert(ConnectInfo(
        "127.0.0.1:12345".parse::<SocketAddr>().unwrap(),
    ));
    request
}

fn authorized_request(method: &str, uri: &str, authorization: &str) -> Request<Body> {
    let mut request = request(method, uri, Body::empty());
    request
        .headers_mut()
        .insert(AUTHORIZATION, authorization.parse().unwrap());
    request
}

#[tokio::test]
async fn global_prefix_routes_requests() {
    let prefixed = app()
        .await
        .clone()
        .oneshot(request("GET", "/api/value", Body::empty()))
        .await
        .unwrap();
    let unprefixed = app()
        .await
        .oneshot(request("GET", "/value", Body::empty()))
        .await
        .unwrap();

    assert_eq!(prefixed.status(), StatusCode::OK);
    assert_eq!(unprefixed.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn challenge_disables_caching() {
    let response = app()
        .await
        .oneshot(request("GET", "/api/challenge", Body::empty()))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers().get(CACHE_CONTROL).unwrap(), "no-store");
}

#[tokio::test]
async fn configured_cors_origin_can_preflight_requests() {
    let mut request = request("OPTIONS", "/api/object", Body::empty());
    request
        .headers_mut()
        .insert(ORIGIN, "http://localhost:8000".parse().unwrap());
    request
        .headers_mut()
        .insert(ACCESS_CONTROL_REQUEST_METHOD, "POST".parse().unwrap());
    let response = app().await.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(ACCESS_CONTROL_ALLOW_ORIGIN).unwrap(),
        "http://localhost:8000"
    );
}

#[tokio::test]
async fn malformed_object_request_returns_bad_request() {
    let response = app()
        .await
        .oneshot(request("POST", "/api/object", Body::from("[]")))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn out_of_range_integer_returns_bad_request() {
    let response = app()
        .await
        .oneshot(request(
            "POST",
            "/api/object",
            Body::from(r#"{"value":9223372036854775808}"#),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn configurable_sql_action_returns_json() {
    let response = app()
        .await
        .oneshot(request("GET", "/api/value", Body::empty()))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body, json!({ "value": "configured" }));
}

#[tokio::test]
async fn endpoint_rate_limits_are_independent() {
    let app = app().await;
    let first = app
        .clone()
        .oneshot(request("GET", "/api/limited", Body::empty()))
        .await
        .unwrap();
    let second = app
        .clone()
        .oneshot(request("GET", "/api/limited", Body::empty()))
        .await
        .unwrap();
    let other_endpoint = app
        .oneshot(request("GET", "/api/value", Body::empty()))
        .await
        .unwrap();

    assert_eq!(first.status(), StatusCode::OK);
    assert_eq!(second.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(other_endpoint.status(), StatusCode::OK);
}

#[tokio::test]
async fn authenticated_clients_can_bypass_altcha_when_configured() {
    sqlx::any::install_default_drivers();
    let pool = AnyPoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::query("CREATE TABLE sessions (token TEXT PRIMARY KEY, user_id INTEGER NOT NULL)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO sessions (token, user_id) VALUES (?, ?)")
        .bind("valid-token")
        .bind(1_i64)
        .execute(&pool)
        .await
        .unwrap();
    let app = build_router(pool, Config::parse(OPTIONAL_AUTH_CONFIG).unwrap()).unwrap();

    let anonymous = app
        .clone()
        .oneshot(request("GET", "/protected", Body::empty()))
        .await
        .unwrap();
    let authenticated = app
        .clone()
        .oneshot(authorized_request(
            "GET",
            "/protected",
            "Bearer valid-token",
        ))
        .await
        .unwrap();
    let invalid = app
        .oneshot(authorized_request(
            "GET",
            "/protected",
            "Bearer invalid-token",
        ))
        .await
        .unwrap();

    assert_eq!(anonymous.status(), StatusCode::FORBIDDEN);
    assert_eq!(authenticated.status(), StatusCode::OK);
    assert_eq!(invalid.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn external_deposits_update_the_balance_exposed_by_the_api() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("integration.db");
    let database_url = format!("sqlite://{}?mode=rwc", database.display());
    let source =
        include_str!("../config/sqlite.toml").replace("sqlite://crudo.db?mode=rwc", &database_url);
    let config = Config::parse(&source).unwrap();
    let api_pool = connect(&config).await.unwrap();
    prepare_database(&api_pool, &config).await.unwrap();
    sqlx::query("INSERT INTO users (name, email, password) VALUES (?, ?, ?)")
        .bind("Owner")
        .bind("owner@example.com")
        .bind("unused")
        .execute(&api_pool)
        .await
        .unwrap();
    let app = build_router(api_pool, config).unwrap();

    let writer = AnyPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO transactions (external_id, user_id, type, status, amount) \
         VALUES (?, ?, 'deposit', 'confirmed', ?)",
    )
    .bind("provider-1")
    .bind(1_i64)
    .bind(1_250_i64)
    .execute(&writer)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO transactions (external_id, user_id, type, status, amount) \
         VALUES (?, ?, 'deposit', 'pending', ?)",
    )
    .bind("provider-2")
    .bind(1_i64)
    .bind(750_i64)
    .execute(&writer)
    .await
    .unwrap();

    assert_eq!(api_balance(&app, 1).await, 1_250);

    sqlx::query("UPDATE transactions SET status = 'confirmed' WHERE external_id = ?")
        .bind("provider-2")
        .execute(&writer)
        .await
        .unwrap();
    assert_eq!(api_balance(&app, 1).await, 2_000);

    let duplicate = sqlx::query(
        "INSERT INTO transactions (external_id, user_id, type, status, amount) \
         VALUES (?, ?, 'deposit', 'confirmed', ?)",
    )
    .bind("provider-1")
    .bind(1_i64)
    .bind(1_250_i64)
    .execute(&writer)
    .await;
    assert!(duplicate.is_err());
    assert_eq!(api_balance(&app, 1).await, 2_000);
}

#[tokio::test]
async fn multiple_users_have_independent_deposits_and_expenses() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("expenses.db");
    let database_url = format!("sqlite://{}?mode=rwc", database.display());
    let source =
        include_str!("../config/sqlite.toml").replace("sqlite://crudo.db?mode=rwc", &database_url);
    let config = Config::parse(&source).unwrap();
    let api_pool = connect(&config).await.unwrap();
    prepare_database(&api_pool, &config).await.unwrap();
    for (name, email) in [
        ("Alice", "alice@example.com"),
        ("Bob", "bob@example.com"),
        ("Carol", "carol@example.com"),
    ] {
        sqlx::query("INSERT INTO users (name, email, password) VALUES (?, ?, ?)")
            .bind(name)
            .bind(email)
            .bind("unused")
            .execute(&api_pool)
            .await
            .unwrap();
    }
    let app = build_router(api_pool, config).unwrap();
    let writer = AnyPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .unwrap();

    for (external_id, user_id, amount) in [
        ("deposit-a", 1_i64, 2_000_i64),
        ("deposit-b", 2, 1_000),
        ("deposit-c", 3, 500),
    ] {
        sqlx::query(
            "INSERT INTO transactions (external_id, user_id, type, status, amount) \
             VALUES (?, ?, 'deposit', 'confirmed', ?)",
        )
        .bind(external_id)
        .bind(user_id)
        .bind(amount)
        .execute(&writer)
        .await
        .unwrap();
    }

    sqlx::query(
        "INSERT INTO expenses (external_id, user_id, status, amount, description) \
         VALUES (?, ?, 'confirmed', ?, ?)",
    )
    .bind("purchase-a")
    .bind(1_i64)
    .bind(600_i64)
    .bind("Books")
    .execute(&writer)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO expenses (external_id, user_id, status, amount, description) \
         VALUES (?, ?, 'pending', ?, ?)",
    )
    .bind("purchase-b")
    .bind(2_i64)
    .bind(250_i64)
    .bind("Groceries")
    .execute(&writer)
    .await
    .unwrap();

    assert_eq!(api_balance(&app, 1).await, 1_400);
    assert_eq!(api_balance(&app, 2).await, 1_000);
    assert_eq!(api_balance(&app, 3).await, 500);

    sqlx::query("UPDATE expenses SET status = 'confirmed' WHERE external_id = ?")
        .bind("purchase-b")
        .execute(&writer)
        .await
        .unwrap();
    assert_eq!(api_balance(&app, 2).await, 750);

    sqlx::query("UPDATE expenses SET status = 'pending' WHERE external_id = ?")
        .bind("purchase-a")
        .execute(&writer)
        .await
        .unwrap();
    sqlx::query("UPDATE expenses SET status = 'confirmed' WHERE external_id = ?")
        .bind("purchase-a")
        .execute(&writer)
        .await
        .unwrap();
    assert_eq!(api_balance(&app, 1).await, 1_400);

    let duplicate = sqlx::query(
        "INSERT INTO expenses (external_id, user_id, status, amount) \
         VALUES (?, ?, 'confirmed', ?)",
    )
    .bind("purchase-a")
    .bind(1_i64)
    .bind(600_i64)
    .execute(&writer)
    .await;
    assert!(duplicate.is_err());
    assert_eq!(api_balance(&app, 1).await, 1_400);

    let insufficient = sqlx::query(
        "INSERT INTO expenses (external_id, user_id, status, amount) \
         VALUES (?, ?, 'confirmed', ?)",
    )
    .bind("purchase-c")
    .bind(3_i64)
    .bind(600_i64)
    .execute(&writer)
    .await;
    assert!(insufficient.is_err());
    let rejected: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM expenses WHERE external_id = 'purchase-c'")
            .fetch_one(&writer)
            .await
            .unwrap();
    assert_eq!(rejected, 0);
    assert_eq!(api_balance(&app, 3).await, 500);
}

async fn api_balance(app: &axum::Router, user_id: i64) -> i64 {
    let response = app
        .clone()
        .oneshot(request(
            "GET",
            &format!("/v1/users/{user_id}"),
            Body::empty(),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    body["balance"].as_i64().unwrap()
}

#[tokio::test]
async fn registration_login_and_owner_deletion_work_together() {
    sqlx::any::install_default_drivers();
    let pool = AnyPoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::raw_sql(
        "CREATE TABLE users (id INTEGER PRIMARY KEY, email TEXT UNIQUE, password TEXT NOT NULL);
         CREATE TABLE sessions (token TEXT PRIMARY KEY, user_id INTEGER NOT NULL, expires_at INTEGER NOT NULL);",
    )
    .execute(&pool)
    .await
    .unwrap();
    let app = build_router(pool.clone(), Config::parse(LIFECYCLE_CONFIG).unwrap()).unwrap();

    let registration = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/users",
            Body::from(r#"{"email":"owner@example.com","password":"secret"}"#),
        ))
        .await
        .unwrap();
    assert_eq!(registration.status(), StatusCode::CREATED);

    let basic = BASE64.encode("owner@example.com:secret");
    let login = app
        .clone()
        .oneshot(authorized_request(
            "POST",
            "/api/tokens",
            &format!("Basic {basic}"),
        ))
        .await
        .unwrap();
    assert_eq!(login.status(), StatusCode::CREATED);
    assert_eq!(login.headers().get(CACHE_CONTROL).unwrap(), "no-store");
    let body: Value =
        serde_json::from_slice(&login.into_body().collect().await.unwrap().to_bytes()).unwrap();
    let token = body["token"].as_str().unwrap();

    let deletion = app
        .oneshot(authorized_request(
            "DELETE",
            "/api/users/1",
            &format!("Bearer {token}"),
        ))
        .await
        .unwrap();
    assert_eq!(deletion.status(), StatusCode::OK);
    let users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(users, 0);
}
