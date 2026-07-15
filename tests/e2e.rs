use std::{env, time::Duration};

use altcha::{Challenge, Payload, SolveChallengeOptions, solve_challenge};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use crudo::{Config, build_router, connect, prepare_database, serve};
use reqwest::{Client, StatusCode};
use serde_json::{Value, json};
use sqlx::{AnyPool, AssertSqlSafe};

#[derive(Clone, Copy, PartialEq)]
enum Backend {
    Sqlite,
    Postgres,
}

impl Backend {
    fn from_env() -> Self {
        match env::var("E2E_BACKEND").as_deref() {
            Ok("sqlite") => Self::Sqlite,
            Ok("postgres") => Self::Postgres,
            value => panic!("E2E_BACKEND must be sqlite or postgres, got {value:?}"),
        }
    }

    fn config(self) -> String {
        let source = match self {
            Self::Sqlite => include_str!("../config/sqlite.toml"),
            Self::Postgres => include_str!("../config/postgres.toml"),
        };
        let source = source
            .replace(
                "sqlite://crudo.db?mode=rwc",
                &env::var("DATABASE_URL").unwrap(),
            )
            .replace("algorithm = \"PBKDF2/SHA-256\"", "algorithm = \"SHA-256\"")
            .replace("cost = 5000", "cost = 1")
            .replace("max_number = 10000", "max_number = 1")
            .replace("requests = 120", "requests = 0");
        let placeholder = if self == Self::Sqlite { "?" } else { "$1" };
        format!(
            r#"{source}

[[endpoints]]
method = "POST"
path = "/limited"
action = "limited"

[endpoints.limits]
body_bytes = 64
requests = 2
window_seconds = 60

[actions.limited]
sql = "SELECT {placeholder} AS value"
params = ["value"]
result = "one"
"#
        )
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "run with docker compose run --build --rm e2e-sqlite or e2e-postgres"]
async fn real_http_financial_lifecycle() {
    let backend = Backend::from_env();
    let config = Config::parse(&backend.config()).unwrap();
    let pool = connect(&config).await.unwrap();
    prepare_database(&pool, &config).await.unwrap();
    reset_database(&pool, backend).await;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = build_router(pool.clone(), config).unwrap();
    let server = tokio::spawn(serve(listener, app));
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();
    let base = format!("http://{address}/v1");

    assert_limits(&client, &base).await;
    let user_ids = register_users_and_verify_altcha(&client, &base).await;
    assert_insufficient_balance_response(&client, &base, &pool, backend, user_ids[0]).await;
    exercise_financial_triggers(&pool, backend, &user_ids).await;

    for user_id in user_ids {
        let response = client
            .get(format!("{base}/users/{user_id}"))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.json::<Value>().await.unwrap()["balance"], 350);
    }

    server.abort();
}

async fn assert_insufficient_balance_response(
    client: &Client,
    base: &str,
    pool: &AnyPool,
    backend: Backend,
    user_id: i64,
) {
    let sql = if backend == Backend::Sqlite {
        "INSERT INTO sessions (token, user_id, expires_at) VALUES (?, ?, unixepoch() + 60)"
    } else {
        "INSERT INTO sessions (token, user_id, expires_at) VALUES ($1, $2, EXTRACT(EPOCH FROM now()) + 60)"
    };
    sqlx::query(sql)
        .bind("insufficient-balance-token")
        .bind(user_id)
        .execute(pool)
        .await
        .unwrap();

    let response = client
        .post(format!("{base}/expenses"))
        .bearer_auth("insufficient-balance-token")
        .json(&json!({
            "external_id": "expense-before-deposit",
            "amount": 1,
            "description": "Not funded",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        response.json::<Value>().await.unwrap(),
        json!({ "error": "insufficient balance" })
    );
}

async fn assert_limits(client: &Client, base: &str) {
    let oversized = client
        .post(format!("{base}/limited"))
        .body(format!(r#"{{"value":"{}"}}"#, "x".repeat(128)))
        .send()
        .await
        .unwrap();
    assert_eq!(oversized.status(), StatusCode::PAYLOAD_TOO_LARGE);

    for expected in [
        StatusCode::OK,
        StatusCode::OK,
        StatusCode::TOO_MANY_REQUESTS,
    ] {
        let response = client
            .post(format!("{base}/limited"))
            .json(&json!({ "value": "small" }))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
        if expected == StatusCode::TOO_MANY_REQUESTS {
            assert!(response.headers().contains_key("retry-after"));
        }
    }
}

async fn register_users_and_verify_altcha(client: &Client, base: &str) -> Vec<i64> {
    let missing = client
        .post(format!("{base}/users"))
        .json(&json!({ "name": "Missing", "email": "missing@example.com", "password": "secret" }))
        .send()
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::FORBIDDEN);

    let first_proof = altcha_proof(client, base).await;
    let first = register(client, base, "Alice", "alice@example.com", &first_proof).await;
    let replay = client
        .post(format!("{base}/users"))
        .json(&json!({
            "name": "Replay",
            "email": "replay@example.com",
            "password": "secret",
            "altcha": first_proof,
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(replay.status(), StatusCode::FORBIDDEN);

    let mut ids = vec![first];
    for (name, email) in [("Bob", "bob@example.com"), ("Carol", "carol@example.com")] {
        ids.push(register(client, base, name, email, &altcha_proof(client, base).await).await);
    }
    ids
}

async fn register(client: &Client, base: &str, name: &str, email: &str, proof: &str) -> i64 {
    let response = client
        .post(format!("{base}/users"))
        .json(&json!({ "name": name, "email": email, "password": "secret", "altcha": proof }))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    response.json::<Value>().await.unwrap()["id"]
        .as_i64()
        .unwrap()
}

async fn altcha_proof(client: &Client, base: &str) -> String {
    let response = client
        .get(format!("{base}/challenge"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "no-store");
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

async fn exercise_financial_triggers(pool: &AnyPool, backend: Backend, users: &[i64]) {
    let mut tasks = Vec::new();
    for &user_id in users {
        for top_up in 0..4 {
            let pool = pool.clone();
            tasks.push(tokio::spawn(async move {
                insert_deposit(
                    &pool,
                    backend,
                    &format!("deposit-{user_id}-{top_up}"),
                    user_id,
                    "confirmed",
                    100,
                )
                .await
            }));
        }
    }
    for task in tasks {
        task.await.unwrap().unwrap();
    }

    for &user_id in users {
        let pending = format!("pending-{user_id}");
        insert_deposit(pool, backend, &pending, user_id, "pending", 50)
            .await
            .unwrap();
        update_status(pool, backend, "transactions", &pending, "confirmed")
            .await
            .unwrap();
        update_status(pool, backend, "transactions", &pending, "pending")
            .await
            .unwrap();
        update_status(pool, backend, "transactions", &pending, "confirmed")
            .await
            .unwrap();

        let confirmed = format!("expense-{user_id}");
        insert_expense(pool, backend, &confirmed, user_id, "confirmed", 75)
            .await
            .unwrap();
        let pending_expense = format!("pending-expense-{user_id}");
        insert_expense(pool, backend, &pending_expense, user_id, "pending", 25)
            .await
            .unwrap();
        update_status(pool, backend, "expenses", &pending_expense, "confirmed")
            .await
            .unwrap();
        update_status(pool, backend, "expenses", &pending_expense, "pending")
            .await
            .unwrap();
        update_status(pool, backend, "expenses", &pending_expense, "confirmed")
            .await
            .unwrap();

        assert!(
            insert_deposit(pool, backend, &pending, user_id, "confirmed", 50)
                .await
                .is_err()
        );
        assert!(
            insert_expense(
                pool,
                backend,
                &format!("too-large-{user_id}"),
                user_id,
                "confirmed",
                1_000
            )
            .await
            .is_err()
        );
    }
}

async fn reset_database(pool: &AnyPool, backend: Backend) {
    let sql = if backend == Backend::Sqlite {
        "DELETE FROM expenses; DELETE FROM transactions; DELETE FROM sessions; DELETE FROM users;"
    } else {
        "TRUNCATE expenses, transactions, sessions, users RESTART IDENTITY CASCADE"
    };
    sqlx::raw_sql(sql).execute(pool).await.unwrap();
}

async fn insert_deposit(
    pool: &AnyPool,
    backend: Backend,
    external_id: &str,
    user_id: i64,
    status: &str,
    amount: i64,
) -> Result<(), sqlx::Error> {
    let sql = if backend == Backend::Sqlite {
        "INSERT INTO transactions (external_id, user_id, type, status, amount) VALUES (?, ?, 'deposit', ?, ?)"
    } else {
        "INSERT INTO transactions (external_id, user_id, type, status, amount) VALUES ($1, $2, 'deposit', $3, $4)"
    };
    sqlx::query(sql)
        .bind(external_id)
        .bind(user_id)
        .bind(status)
        .bind(amount)
        .execute(pool)
        .await
        .map(|_| ())
}

async fn insert_expense(
    pool: &AnyPool,
    backend: Backend,
    external_id: &str,
    user_id: i64,
    status: &str,
    amount: i64,
) -> Result<(), sqlx::Error> {
    let sql = if backend == Backend::Sqlite {
        "INSERT INTO expenses (external_id, user_id, status, amount) VALUES (?, ?, ?, ?)"
    } else {
        "INSERT INTO expenses (external_id, user_id, status, amount) VALUES ($1, $2, $3, $4)"
    };
    sqlx::query(sql)
        .bind(external_id)
        .bind(user_id)
        .bind(status)
        .bind(amount)
        .execute(pool)
        .await
        .map(|_| ())
}

async fn update_status(
    pool: &AnyPool,
    backend: Backend,
    table: &str,
    external_id: &str,
    status: &str,
) -> Result<(), sqlx::Error> {
    assert!(matches!(table, "transactions" | "expenses"));
    let sql = if backend == Backend::Sqlite {
        format!("UPDATE {table} SET status = ? WHERE external_id = ?")
    } else {
        format!("UPDATE {table} SET status = $1 WHERE external_id = $2")
    };
    sqlx::query(AssertSqlSafe(sql))
        .bind(status)
        .bind(external_id)
        .execute(pool)
        .await
        .map(|_| ())
}
