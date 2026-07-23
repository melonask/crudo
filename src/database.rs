use anyhow::{Context, Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use serde_json::{Map, Value};
use sqlx::{AnyPool, AssertSqlSafe, Column, Row, TypeInfo, ValueRef, any::AnyPoolOptions};

use crate::config::{
    Config, DatabaseBackend, DatabaseSetup, DatabaseSetupDetails, DatabaseSetupSource,
    DatabaseSetupSourceFormat, expand_env,
};
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
    prepare_database_setup(pool, config.database.backend, &config.database.setup).await
}

pub(crate) async fn prepare_database_setup(
    pool: &AnyPool,
    backend: DatabaseBackend,
    setup: &DatabaseSetup,
) -> Result<()> {
    let backend_setup = match backend {
        DatabaseBackend::Sqlite => &setup.sqlite,
        DatabaseBackend::Postgres => &setup.postgres,
    };
    let mut statements = load_setup_statements(backend_setup).await?;
    statements.extend(load_setup_statements(&setup.common).await?);
    let mut transaction = pool
        .begin()
        .await
        .context("could not start database setup transaction")?;
    for (index, statement) in statements.iter().enumerate() {
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

async fn load_setup_statements(setup: &DatabaseSetupDetails) -> Result<Vec<String>> {
    if setup.is_empty() {
        return Ok(Vec::new());
    }
    let mut statements = setup.statements.clone();
    for source in &setup.sources {
        statements.extend(load_setup_source(source).await?);
    }
    Ok(statements)
}

async fn load_setup_source(source: &DatabaseSetupSource) -> Result<Vec<String>> {
    let location = &source.location;
    if location.starts_with("http://") {
        bail!("database setup source must use HTTPS: {location}");
    }
    let content = if location.starts_with("https://") {
        crate::tls::install_crypto_provider();
        reqwest::get(location)
            .await
            .with_context(|| format!("could not fetch database setup source {location}"))?
            .error_for_status()
            .with_context(|| format!("could not fetch database setup source {location}"))?
            .text()
            .await
            .with_context(|| format!("could not read database setup source {location}"))?
    } else {
        tokio::fs::read_to_string(location)
            .await
            .with_context(|| format!("could not read database setup source {location}"))?
    };
    let content = expand_env(&content)
        .with_context(|| format!("could not expand database setup source {location}"))?;

    match source.format {
        DatabaseSetupSourceFormat::Sql => {
            if content.trim().is_empty() {
                bail!("SQL database setup source {location} is empty");
            }
            Ok(vec![content])
        }
        DatabaseSetupSourceFormat::Json => parse_json_setup_source(location, &content),
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct JsonSetupObject {
    statements: Vec<String>,
}

fn parse_json_setup_source(location: &str, content: &str) -> Result<Vec<String>> {
    let value: Value = serde_json::from_str(content)
        .with_context(|| format!("invalid JSON database setup source {location}"))?;
    let statements = match value {
        Value::Array(_) => serde_json::from_value(value),
        Value::Object(_) => serde_json::from_value::<JsonSetupObject>(value)
            .map(|source| source.statements),
        _ => bail!(
            "JSON database setup source {location} must be an array of SQL statements or an object with statements"
        ),
    }
    .with_context(|| format!("invalid JSON database setup source {location}"))?;
    for (index, statement) in statements.iter().enumerate() {
        if statement.trim().is_empty() {
            bail!(
                "JSON database setup source {location} contains an empty statement at index {index}"
            );
        }
    }
    Ok(statements)
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
            // SQLite values are dynamically typed, so a column's declared type can differ from
            // the type of its returned value. Decode according to the latter.
            let type_info = raw.type_info();
            match type_info.name().to_ascii_uppercase().as_str() {
                "BOOL" | "BOOLEAN" => Value::Bool(row.try_get(index)?),
                "INT2" | "SMALLINT" | "INT4" | "INT" | "INTEGER" | "SERIAL" | "INT8" | "BIGINT"
                | "BIGSERIAL" => Value::Number(row.try_get::<i64, _>(index)?.into()),
                "REAL" | "FLOAT" | "FLOAT4" | "FLOAT8" | "DOUBLE" | "DOUBLE PRECISION" => {
                    serde_json::Number::from_f64(row.try_get(index)?)
                        .map(Value::Number)
                        .context("database returned a non-finite number")?
                }
                "BLOB" | "BYTEA" => Value::String(BASE64.encode(row.try_get::<Vec<u8>, _>(index)?)),
                "TEXT" | "VARCHAR" | "CHARACTER VARYING" | "CHAR" | "CHARACTER" | "NAME"
                | "CITEXT" => Value::String(row.try_get(index)?),
                type_name => bail!("unsupported database column type {type_name}"),
            }
        };
        object.insert(column.name().to_owned(), value);
    }
    Ok(Value::Object(object))
}

pub(crate) fn row_to_json_with_booleans(
    row: sqlx::any::AnyRow,
    boolean_columns: &[String],
) -> Result<Value> {
    let mut value = row_to_json(row)?;
    let object = value
        .as_object_mut()
        .expect("row_to_json always returns an object");
    for column in boolean_columns {
        let value = object
            .get_mut(column)
            .with_context(|| format!("action result has no boolean column {column}"))?;
        *value = match value {
            Value::Bool(value) => Value::Bool(*value),
            Value::Number(value) if value.as_i64() == Some(0) => Value::Bool(false),
            Value::Number(value) if value.as_i64() == Some(1) => Value::Bool(true),
            _ => bail!("action result boolean column {column} must be a boolean or integer 0 or 1"),
        };
    }
    Ok(value)
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

    #[tokio::test]
    async fn database_setup_is_atomic() {
        let config = Config::parse(
            r#"
            [database]
            url = "sqlite::memory:"
            [database.setup.sqlite]
            statements = [
                "CREATE TABLE incomplete (id INTEGER PRIMARY KEY)",
            ]

            [database.setup.common]
            statements = ["INVALID SQL"]
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
    async fn local_sql_and_json_sources_execute_in_order() {
        let directory = tempfile::tempdir().unwrap();
        let sql = directory.path().join("seed.sql");
        let json = directory.path().join("seed.json");
        let json_array = directory.path().join("seed-array.json");
        tokio::fs::write(&sql, "INSERT INTO entries (value) VALUES ('sql')")
            .await
            .unwrap();
        tokio::fs::write(
            &json,
            r#"{"statements":["INSERT INTO entries (value) VALUES ('json')"]}"#,
        )
        .await
        .unwrap();
        tokio::fs::write(
            &json_array,
            r#"["INSERT INTO entries (value) VALUES ('array')"]"#,
        )
        .await
        .unwrap();
        let config = Config::parse(&format!(
            r#"
            [database]
            url = "sqlite::memory:"

            [database.setup.sqlite]
            statements = ["CREATE TABLE entries (value TEXT)", "INSERT INTO entries (value) VALUES ('inline')"]
            sources = [
                {{ location = "{}", format = "sql" }},
                {{ location = "{}", format = "json" }},
                {{ location = "{}", format = "json" }},
            ]
            "#,
            sql.display(),
            json.display(),
            json_array.display(),
        ))
        .unwrap();
        let pool = memory_pool().await;

        prepare_database(&pool, &config).await.unwrap();

        let values: Vec<String> = sqlx::query_scalar("SELECT value FROM entries ORDER BY rowid")
            .fetch_all(&pool)
            .await
            .unwrap();
        assert_eq!(values, ["inline", "sql", "json", "array"]);
    }

    #[tokio::test]
    async fn backend_setup_runs_before_common_setup() {
        let config = Config::parse(
            r#"
            [database]
            url = "sqlite::memory:"

            [database.setup.sqlite]
            statements = ["CREATE TABLE entries (value TEXT)"]

            [database.setup.common]
            statements = ["INSERT INTO entries (value) VALUES ('common')"]
            "#,
        )
        .unwrap();
        let pool = memory_pool().await;

        prepare_database(&pool, &config).await.unwrap();

        let value: String = sqlx::query_scalar("SELECT value FROM entries")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(value, "common");
    }

    #[tokio::test]
    async fn invalid_json_source_shape_includes_its_location() {
        let file = tempfile::NamedTempFile::new().unwrap();
        tokio::fs::write(file.path(), r#"{"sql":"SELECT 1"}"#)
            .await
            .unwrap();
        let config = Config::parse(&format!(
            r#"
            [database]
            url = "sqlite::memory:"
            [database.setup.sqlite]
            sources = [{{ location = "{}", format = "json" }}]
            "#,
            file.path().display(),
        ))
        .unwrap();

        let error = prepare_database(&memory_pool().await, &config)
            .await
            .unwrap_err();

        assert!(
            error
                .to_string()
                .contains(&file.path().display().to_string())
        );
    }

    #[tokio::test]
    async fn plain_http_database_source_is_rejected() {
        let config = Config::parse(
            r#"
            [database]
            url = "sqlite::memory:"
            [database.setup.sqlite]
            sources = [{ location = "http://example.invalid/setup.sql", format = "sql" }]
            "#,
        )
        .unwrap();

        let error = prepare_database(&memory_pool().await, &config)
            .await
            .unwrap_err();

        assert!(error.to_string().contains("must use HTTPS"));
    }

    #[tokio::test]
    async fn empty_sql_source_is_rejected_with_its_location() {
        let file = tempfile::NamedTempFile::new().unwrap();
        tokio::fs::write(file.path(), " \n\t ").await.unwrap();
        let config = Config::parse(&format!(
            r#"
            [database]
            url = "sqlite::memory:"
            [database.setup.sqlite]
            sources = [{{ location = "{}", format = "sql" }}]
            "#,
            file.path().display(),
        ))
        .unwrap();

        let error = prepare_database(&memory_pool().await, &config)
            .await
            .unwrap_err();

        assert!(
            error
                .to_string()
                .contains(&file.path().display().to_string())
        );
    }

    #[tokio::test]
    async fn source_statements_are_atomic() {
        let file = tempfile::NamedTempFile::new().unwrap();
        tokio::fs::write(
            file.path(),
            "CREATE TABLE source_incomplete (id INTEGER PRIMARY KEY); INVALID SQL",
        )
        .await
        .unwrap();
        let config = Config::parse(&format!(
            r#"
            [database]
            url = "sqlite::memory:"
            [database.setup.sqlite]
            sources = [{{ location = "{}", format = "sql" }}]
            "#,
            file.path().display(),
        ))
        .unwrap();
        let pool = memory_pool().await;

        assert!(prepare_database(&pool, &config).await.is_err());
        let table_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'source_incomplete'",
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

    #[test]
    fn integers_larger_than_the_database_range_are_rejected() {
        let value = serde_json::json!(9_223_372_036_854_775_808_u64);

        let error = match bind(sqlx::query("SELECT ?"), &value) {
            Ok(_) => panic!("out-of-range integer was accepted"),
            Err(error) => error,
        };

        assert_eq!(error.to_string(), "integer is too large");
    }

    #[tokio::test]
    async fn sqlite_integer_action_boolean_columns_serialize_as_json_booleans() {
        let pool = memory_pool().await;
        sqlx::raw_sql(AssertSqlSafe(
            "CREATE TABLE flags (active INTEGER NOT NULL); INSERT INTO flags VALUES (0), (1)",
        ))
        .execute(&pool)
        .await
        .unwrap();
        let rows = sqlx::query("SELECT active FROM flags ORDER BY rowid")
            .fetch_all(&pool)
            .await
            .unwrap();

        let values = rows
            .into_iter()
            .map(|row| row_to_json_with_booleans(row, &["active".into()]))
            .collect::<Result<Vec<_>>>()
            .unwrap();

        assert_eq!(
            values,
            [
                serde_json::json!({ "active": false }),
                serde_json::json!({ "active": true })
            ]
        );
    }

    #[tokio::test]
    async fn invalid_action_boolean_column_value_is_rejected() {
        let pool = memory_pool().await;
        sqlx::raw_sql(AssertSqlSafe(
            "CREATE TABLE flags (active INTEGER NOT NULL); INSERT INTO flags VALUES (2)",
        ))
        .execute(&pool)
        .await
        .unwrap();
        let row = sqlx::query("SELECT active FROM flags")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(
            row_to_json_with_booleans(row, &["active".into()])
                .unwrap_err()
                .to_string()
                .contains("must be a boolean or integer 0 or 1")
        );
    }

    #[tokio::test]
    async fn sqlite_values_are_decoded_by_runtime_type() {
        let pool = memory_pool().await;
        let row = sqlx::query("SELECT 42 AS value")
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(
            row_to_json(row).unwrap(),
            serde_json::json!({ "value": 42 })
        );
    }
}
