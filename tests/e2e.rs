use std::{env, time::Duration};

use altcha::{Challenge, Payload, SolveChallengeOptions, solve_challenge};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use crudo::{Config, build_router, connect, prepare_database, serve};
use reqwest::{Client, StatusCode, header::HeaderName};
use serde_json::{Value, json};
use sqlx::AnyPool;

#[derive(Clone, Copy, PartialEq)]
enum Backend {
    Sqlite,
    Postgres,
}

const TEST_WALLET_MNEMONIC: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
const PAYMENT_REQUIRED: HeaderName = HeaderName::from_static("payment-required");

impl Backend {
    fn from_env() -> Self {
        match env::var("E2E_BACKEND").as_deref() {
            Ok("sqlite") => Self::Sqlite,
            Ok("postgres") => Self::Postgres,
            value => panic!("E2E_BACKEND must be sqlite or postgres, got {value:?}"),
        }
    }

    fn config(self) -> String {
        match self {
            Self::Sqlite => {
                let database_url = format!(
                    "sqlite:///tmp/crudo-e2e-{}-{}.db?mode=rwc",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                );
                include_str!("../config/sqlite.toml")
                    .replace("sqlite://crudo-store.db?mode=rwc", &database_url)
                    .replace("requests = 30", "requests = 0")
            }
            Self::Postgres => include_str!("../config/postgres.toml")
                .replace(
                    "${DATABASE_URL}",
                    &env::var("DATABASE_URL").expect("DATABASE_URL is required for postgres E2E"),
                )
                .replace("requests = 30", "requests = 0"),
        }
        .replace("${WALLET_MNEMONIC}", TEST_WALLET_MNEMONIC)
        .replace("${ALTCHA_SECRET}", "test-altcha-secret")
        .replace("${ALTCHA_KEY_SECRET}", "test-altcha-key-secret")
        .replace("cost = 10000", "cost = 1")
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "run with docker compose run --build --rm e2e-sqlite or e2e-postgres"]
async fn real_http_store_lifecycle() {
    let backend = Backend::from_env();
    let config = Config::parse(&backend.config()).unwrap();
    let pool = connect(&config).await.unwrap();
    prepare_database(&pool, &config).await.unwrap();
    reset_store(&pool).await;
    prepare_database(&pool, &config).await.unwrap();
    prepare_database(&pool, &config).await.unwrap();
    assert_seeded_store(&pool).await;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(serve(listener, build_router(pool, config).unwrap()));
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();
    let base = format!("http://{address}/v1");
    let suffix = format!(
        "{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );

    let admin = login(&client, &base, "admin", "admin").await;
    assert_eq!(me(&client, &base, &admin).await["role"], "admin");
    let products = get(&client, &format!("{base}/products"), None).await;
    let products = products.as_array().unwrap();
    assert_eq!(products.len(), 4);
    assert!(
        products
            .iter()
            .all(|product| product.get("fulfillment").is_none())
    );
    let service = products
        .iter()
        .find(|product| product["price"] == 1299)
        .unwrap();
    let service_id = service["id"].as_i64().unwrap();
    let expensive_id = products
        .iter()
        .find(|product| product["price"] == 4999)
        .unwrap()["id"]
        .as_i64()
        .unwrap();

    let alice_email = format!("alice-{suffix}@example.test");
    let bob_email = format!("bob-{suffix}@example.test");
    let alice_id = register(&client, &base, &alice_email).await;
    let bob_id = register(&client, &base, &bob_email).await;
    let alice = login(&client, &base, &alice_email, "secret").await;
    let bob = login(&client, &base, &bob_email, "secret").await;
    assert_customer_restrictions(&client, &base, &alice, service_id, &suffix).await;

    top_up(
        &client,
        &base,
        &alice,
        &format!("alice-credit-{suffix}"),
        5_000,
    )
    .await;
    let duplicate = post(
        &client,
        &format!("{base}/top-ups"),
        &alice,
        json!({"external_id":format!("alice-credit-{suffix}"),"amount":5000}),
    )
    .await;
    assert_ne!(duplicate.status(), StatusCode::CREATED);
    assert_eq!(me(&client, &base, &alice).await["balance"], 5000);
    top_up(&client, &base, &bob, &format!("bob-credit-{suffix}"), 2_000).await;
    assert_eq!(me(&client, &base, &bob).await["balance"], 2000);

    let purchase_external_id = format!("alice-purchase-{suffix}");
    let purchase = post(
        &client,
        &format!("{base}/purchases"),
        &alice,
        json!({"external_id":purchase_external_id,"product_id":service_id}),
    )
    .await;
    assert_eq!(purchase.status(), StatusCode::CREATED);
    let purchase = purchase.json::<Value>().await.unwrap();
    assert_eq!(purchase["amount"], 1299);
    assert!(
        purchase["fulfillment"]
            .as_str()
            .unwrap()
            .contains("Upload your source image")
    );
    assert!(!purchase["license_key"].as_str().unwrap().is_empty());
    assert_eq!(me(&client, &base, &alice).await["balance"], 3701);
    assert_history_ownership(&client, &base, &alice, &bob, &purchase_external_id).await;

    let insufficient_external_id = format!("insufficient-{suffix}");
    let insufficient = post(
        &client,
        &format!("{base}/purchases"),
        &alice,
        json!({"external_id":insufficient_external_id,"product_id":expensive_id}),
    )
    .await;
    assert_eq!(insufficient.status(), StatusCode::PAYMENT_REQUIRED);
    let alice_destinations =
        assert_store_payment_required(payment_required_payload(insufficient).await, alice_id);
    let bob_insufficient = post(
        &client,
        &format!("{base}/purchases"),
        &bob,
        json!({"external_id":format!("bob-insufficient-{suffix}"),"product_id":expensive_id}),
    )
    .await;
    assert_eq!(bob_insufficient.status(), StatusCode::PAYMENT_REQUIRED);
    let bob_destinations =
        assert_store_payment_required(payment_required_payload(bob_insufficient).await, bob_id);
    assert_ne!(alice_destinations, bob_destinations);
    assert_eq!(me(&client, &base, &alice).await["balance"], 3701);
    assert_history_absent(&client, &base, &alice, &insufficient_external_id).await;

    let managed = create_and_cycle_product(&client, &base, &admin, &bob, &suffix).await;
    assert_admin_views(
        &client,
        &base,
        &admin,
        alice_id,
        bob_id,
        &purchase_external_id,
        &managed,
    )
    .await;
    server.abort();
}

async fn reset_store(pool: &AnyPool) {
    sqlx::raw_sql(
        "DELETE FROM transactions; DELETE FROM sessions; DELETE FROM products; DELETE FROM users WHERE role = 'customer';",
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn assert_seeded_store(pool: &AnyPool) {
    let admin: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE email = 'admin' AND role = 'admin'")
            .fetch_one(pool)
            .await
            .unwrap();
    let products: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM products")
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(admin, 1);
    assert_eq!(products, 4);
}

async fn register(client: &Client, base: &str, email: &str) -> i64 {
    let response = client
        .post(format!("{base}/users"))
        .json(&json!({"name":"Customer","email":email,"password":"secret","altcha":altcha_proof(client, base).await}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    response.json::<Value>().await.unwrap()["id"]
        .as_i64()
        .unwrap()
}

async fn login(client: &Client, base: &str, email: &str, password: &str) -> String {
    let response = client
        .post(format!("{base}/tokens"))
        .header(
            "authorization",
            format!("Basic {}", BASE64.encode(format!("{email}:{password}"))),
        )
        .json(&json!({"altcha":altcha_proof(client, base).await}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    response.json::<Value>().await.unwrap()["token"]
        .as_str()
        .unwrap()
        .to_owned()
}

async fn altcha_proof(client: &Client, base: &str) -> String {
    let response = client
        .get(format!("{base}/challenge"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let challenge = response.json::<Challenge>().await.unwrap();
    let solution = solve_challenge(SolveChallengeOptions::new(&challenge))
        .unwrap()
        .unwrap();
    BASE64.encode(
        serde_json::to_vec(&Payload {
            challenge,
            solution,
        })
        .unwrap(),
    )
}

async fn get(client: &Client, url: &str, token: Option<&str>) -> Value {
    let mut request = client.get(url);
    if let Some(token) = token {
        request = request.bearer_auth(token);
    }
    let response = request.send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    response.json().await.unwrap()
}

async fn post(client: &Client, url: &str, token: &str, body: Value) -> reqwest::Response {
    client
        .post(url)
        .bearer_auth(token)
        .json(&body)
        .send()
        .await
        .unwrap()
}

async fn me(client: &Client, base: &str, token: &str) -> Value {
    get(client, &format!("{base}/me"), Some(token)).await
}

async fn payment_required_payload(response: reqwest::Response) -> Value {
    let header = response
        .headers()
        .get(&PAYMENT_REQUIRED)
        .unwrap()
        .as_bytes()
        .to_vec();
    let body = response.bytes().await.unwrap();
    assert_eq!(BASE64.decode(header).unwrap(), body);
    serde_json::from_slice(&body).unwrap()
}

fn assert_store_payment_required(payload: Value, user_id: i64) -> Vec<Value> {
    assert_eq!(payload["x402Version"], 2);
    let accepts = payload["accepts"].as_array().unwrap();
    assert_eq!(accepts.len(), 1);
    assert_eq!(
        accepts[0],
        json!({
            "scheme": "exact",
            "network": "eip155:8453",
            "amount": "49990000",
            "asset": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
            "payTo": "0x9858EfFD232B4033E47d90003D41EC34EcaEda94",
            "maxTimeoutSeconds": 60,
            "extra": { "name": "USDC", "version": "2" },
        })
    );

    let deposit = &payload["extensions"]["deposit"];
    assert!(deposit["info"].is_object());
    assert!(deposit["schema"].is_object());
    assert_eq!(deposit["info"]["uid"], user_id.to_string());
    let destinations = deposit["info"]["destinations"].as_array().unwrap();
    assert_eq!(destinations.len(), 2);
    for (network, asset) in [
        ("eip155:8453", "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"),
        (
            "solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp",
            "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
        ),
    ] {
        let destination = destinations
            .iter()
            .find(|destination| destination["network"] == network)
            .unwrap();
        assert_eq!(destination["asset"], asset);
        assert!(
            destination["payTo"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
        );
        assert_eq!(destination["minAmount"], "49990000");
        assert!(destination["minAmount"].is_string());
    }
    let schema = &deposit["schema"];
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(
        schema["properties"]["destinations"]["items"]["additionalProperties"],
        false
    );
    destinations.to_vec()
}

async fn top_up(client: &Client, base: &str, token: &str, external_id: &str, amount: i64) {
    let response = post(
        client,
        &format!("{base}/top-ups"),
        token,
        json!({"external_id":external_id,"amount":amount}),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
}

async fn assert_customer_restrictions(
    client: &Client,
    base: &str,
    token: &str,
    product_id: i64,
    suffix: &str,
) {
    for path in [
        "admin/summary",
        "admin/users",
        "admin/transactions",
        "admin/users/1/transactions",
        "admin/products",
    ] {
        let response = client
            .get(format!("{base}/{path}"))
            .bearer_auth(token)
            .send()
            .await
            .unwrap();
        if response.status() == StatusCode::OK {
            let body = response.json::<Value>().await.unwrap();
            assert!(body.is_null() || body.as_array().is_some_and(Vec::is_empty));
        } else {
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
        }
    }
    let response = post(client, &format!("{base}/admin/products"), token, json!({"slug":format!("forbidden-{suffix}"),"name":"Forbidden","description":"Forbidden","category":"asset","price":1,"fulfillment":"private"})).await;
    if response.status() == StatusCode::CREATED {
        assert!(response.json::<Value>().await.unwrap().is_null());
    } else {
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
    for (path, body) in [
        (
            format!("{base}/admin/products/{product_id}"),
            json!({"slug":"unchanged","name":"Unchanged","description":"Unchanged","category":"asset","price":1,"fulfillment":"private"}),
        ),
        (
            format!("{base}/admin/products/{product_id}/status"),
            json!({"active":false}),
        ),
    ] {
        let response = client
            .put(path)
            .bearer_auth(token)
            .json(&body)
            .send()
            .await
            .unwrap();
        if response.status() == StatusCode::OK {
            assert!(response.json::<Value>().await.unwrap().is_null());
        } else {
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
        }
    }
}

async fn assert_history_ownership(
    client: &Client,
    base: &str,
    alice: &str,
    bob: &str,
    external_id: &str,
) {
    let alice_history = get(client, &format!("{base}/transactions"), Some(alice)).await;
    let bob_history = get(client, &format!("{base}/transactions"), Some(bob)).await;
    assert!(
        alice_history
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["external_id"] == external_id)
    );
    assert!(
        bob_history
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["external_id"] != external_id)
    );
}

async fn assert_history_absent(client: &Client, base: &str, token: &str, external_id: &str) {
    let history = get(client, &format!("{base}/transactions"), Some(token)).await;
    assert!(
        history
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["external_id"] != external_id)
    );
}

async fn create_and_cycle_product(
    client: &Client,
    base: &str,
    admin: &str,
    bob: &str,
    suffix: &str,
) -> String {
    let slug = format!("managed-{suffix}");
    let created = post(client, &format!("{base}/admin/products"), admin, json!({"slug":slug,"name":"Managed","description":"Managed product","category":"asset","price":777,"fulfillment":"managed fulfillment"})).await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let product = created.json::<Value>().await.unwrap();
    let id = product["id"].as_i64().unwrap();
    let updated = client.put(format!("{base}/admin/products/{id}")).bearer_auth(admin).json(&json!({"slug":slug,"name":"Managed Updated","description":"Updated product","category":"asset","price":888,"fulfillment":"updated fulfillment"})).send().await.unwrap();
    assert_eq!(updated.status(), StatusCode::OK);
    let disabled = client
        .put(format!("{base}/admin/products/{id}/status"))
        .bearer_auth(admin)
        .json(&json!({"active":false}))
        .send()
        .await
        .unwrap();
    assert_eq!(disabled.status(), StatusCode::OK);
    let blocked_id = format!("inactive-{suffix}");
    let blocked = post(
        client,
        &format!("{base}/purchases"),
        bob,
        json!({"external_id":blocked_id,"product_id":id}),
    )
    .await;
    assert_ne!(blocked.status(), StatusCode::CREATED);
    assert_history_absent(client, base, bob, &blocked_id).await;
    let enabled = client
        .put(format!("{base}/admin/products/{id}/status"))
        .bearer_auth(admin)
        .json(&json!({"active":true}))
        .send()
        .await
        .unwrap();
    assert_eq!(enabled.status(), StatusCode::OK);
    let external_id = format!("bob-purchase-{suffix}");
    let purchase = post(
        client,
        &format!("{base}/purchases"),
        bob,
        json!({"external_id":external_id,"product_id":id}),
    )
    .await;
    assert_eq!(purchase.status(), StatusCode::CREATED);
    assert_eq!(purchase.json::<Value>().await.unwrap()["amount"], 888);
    assert_eq!(me(client, base, bob).await["balance"], 1112);
    external_id
}

async fn assert_admin_views(
    client: &Client,
    base: &str,
    admin: &str,
    alice_id: i64,
    bob_id: i64,
    alice_purchase: &str,
    bob_purchase: &str,
) {
    let summary = get(client, &format!("{base}/admin/summary"), Some(admin)).await;
    assert_eq!(summary["user_count"], 3);
    assert_eq!(summary["customer_count"], 2);
    assert_eq!(summary["product_count"], 5);
    assert_eq!(summary["transaction_count"], 4);
    assert_eq!(summary["total_topups"], 7000);
    assert_eq!(summary["total_sales"], 2187);
    let users = get(client, &format!("{base}/admin/users"), Some(admin)).await;
    assert_eq!(users.as_array().unwrap().len(), 3);
    let transactions = get(client, &format!("{base}/admin/transactions"), Some(admin)).await;
    assert!(
        transactions
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["external_id"] == alice_purchase)
    );
    assert!(
        transactions
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["external_id"] == bob_purchase)
    );
    let alice_transactions = get(
        client,
        &format!("{base}/admin/users/{alice_id}/transactions"),
        Some(admin),
    )
    .await;
    assert!(
        alice_transactions
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["user_id"] == alice_id)
    );
    let bob_transactions = get(
        client,
        &format!("{base}/admin/users/{bob_id}/transactions"),
        Some(admin),
    )
    .await;
    assert!(
        bob_transactions
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["user_id"] == bob_id)
    );
}
