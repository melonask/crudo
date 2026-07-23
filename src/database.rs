use anyhow::{Context, Result};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use serde_json::{Map, Value};
use sqlx::{AnyPool, AssertSqlSafe, Column, Row, TypeInfo, ValueRef, any::AnyPoolOptions};

use crate::config::Config;
use crate::server::ClientError;

pub async fn connect(config: &Config) -> Result<AnyPool> {
    crate::tls::install_crypto_provider();
    sqlx::any::install_default_drivers();
    AnyPoolOptions::new()
        .connect(&config.database.url)
        .await
        .context("could not connect to configured database")
}

pub async fn prepare_database(pool: &AnyPool, config: &Config) -> Result<()> {
    prepare_database_setup(pool, &config.database.setup).await
}

pub(crate) async fn prepare_database_setup(pool: &AnyPool, setup: &[String]) -> Result<()> {
    let mut transaction = pool
        .begin()
        .await
        .context("could not start database setup transaction")?;
    for (index, statement) in setup.iter().enumerate() {
        sqlx::raw_sql(AssertSqlSafe(statement.as_str()))
            .execute(&mut *transaction)
            .await
            .with_context(|| format!("database setup statement {} failed", index + 1))?;
    }
    transaction
        .commit()
        .await
        .context("could not commit database setup")?;
    Ok(())
}

pub(crate) fn bind<'q>(
    query: sqlx::query::Query<'q, sqlx::Any, sqlx::any::AnyArguments>,
    value: &'q Value,
) -> Result<sqlx::query::Query<'q, sqlx::Any, sqlx::any::AnyArguments>> {
    Ok(match value {
        Value::Null => query.bind(Option::<String>::None),
        Value::Bool(value) => query.bind(*value),
        Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                query.bind(value)
            } else if let Some(value) = value.as_u64() {
                query.bind(
                    i64::try_from(value)
                        .map_err(|_| ClientError::bad_request("integer is too large"))?,
                )
            } else {
                query.bind(
                    value
                        .as_f64()
                        .ok_or_else(|| ClientError::bad_request("number is not representable"))?,
                )
            }
        }
        Value::String(value) => query.bind(value),
        value => query.bind(value.to_string()),
    })
}

pub(crate) fn row_to_json(row: sqlx::any::AnyRow) -> Result<Value> {
    let mut object = Map::new();
    for (index, column) in row.columns().iter().enumerate() {
        let raw = row.try_get_raw(index)?;
        let value = if raw.is_null() {
            Value::Null
        } else {
            match column.type_info().name().to_ascii_uppercase().as_str() {
                "BOOL" | "BOOLEAN" => Value::Bool(row.try_get(index)?),
                "INT2" | "SMALLINT" | "INT4" | "INT" | "INTEGER" | "SERIAL" | "INT8" | "BIGINT"
                | "BIGSERIAL" => Value::Number(row.try_get::<i64, _>(index)?.into()),
                "REAL" | "FLOAT" | "FLOAT4" | "FLOAT8" | "DOUBLE" | "DOUBLE PRECISION" => {
                    serde_json::Number::from_f64(row.try_get(index)?)
                        .map(Value::Number)
                        .context("database returned a non-finite number")?
                }
                "BLOB" | "BYTEA" => Value::String(BASE64.encode(row.try_get::<Vec<u8>, _>(index)?)),
                _ => Value::String(row.try_get(index)?),
            }
        };
        object.insert(column.name().to_owned(), value);
    }
    Ok(Value::Object(object))
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn memory_pool() -> AnyPool {
        sqlx::any::install_default_drivers();
        AnyPoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap()
    }

    fn example_config() -> Config {
        let source = include_str!("../config/sqlite.toml");
        Config::parse(source).unwrap()
    }

    #[tokio::test]
    async fn database_setup_is_atomic() {
        let config = Config::parse(
            r#"
            [database]
            url = "sqlite::memory:"
            setup = [
                "CREATE TABLE incomplete (id INTEGER PRIMARY KEY)",
                "INVALID SQL",
            ]
            "#,
        )
        .unwrap();
        let pool = memory_pool().await;

        let error = prepare_database(&pool, &config).await.unwrap_err();

        assert!(error.to_string().contains("setup statement 2 failed"));
        let table_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'incomplete'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(table_count, 0);
    }

    #[tokio::test]
    async fn empty_database_setup_is_valid() {
        let config = Config::parse(
            r#"
            [database]
            url = "sqlite::memory:"
            "#,
        )
        .unwrap();
        let pool = memory_pool().await;

        prepare_database(&pool, &config).await.unwrap();

        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT 1")
                .fetch_one(&pool)
                .await
                .unwrap(),
            1
        );
    }

    #[tokio::test]
    async fn store_setup_is_idempotent_and_seeds_the_demo_catalog() {
        let config = example_config();
        let pool = memory_pool().await;

        prepare_database(&pool, &config).await.unwrap();
        prepare_database(&pool, &config).await.unwrap();

        let admin_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM users WHERE email = 'admin' AND role = 'admin'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let product_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM products")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(admin_count, 1);
        assert_eq!(product_count, 4);
    }

    #[test]
    fn integers_larger_than_the_database_range_are_rejected() {
        let value = serde_json::json!(9_223_372_036_854_775_808_u64);

        let error = match bind(sqlx::query("SELECT ?"), &value) {
            Ok(_) => panic!("out-of-range integer was accepted"),
            Err(error) => error,
        };

        assert_eq!(error.to_string(), "integer is too large");
    }
}
