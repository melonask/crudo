use std::{
    collections::{BTreeMap, HashMap},
    fmt,
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use altcha::{
    Challenge, CreateChallengeOptions, Payload, VerifySolutionOptions, create_challenge,
    verify_solution,
};
use anyhow::{Context, Result, bail};
use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use axum::{
    Json, Router,
    body::Bytes,
    extract::{ConnectInfo, DefaultBodyLimit, MatchedPath, Path, Query, State},
    http::{
        HeaderMap, HeaderValue, Method, StatusCode,
        header::{AUTHORIZATION, CACHE_CONTROL, CONTENT_TYPE, RETRY_AFTER},
    },
    response::{IntoResponse, Response},
    routing::{MethodFilter, on},
};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use hmac::{Hmac, KeyInit, Mac};
use rand::RngExt;
use serde_json::{Map, Value, json};
use sha2::Sha256;
use sqlx::{AnyPool, AssertSqlSafe, Row};
use tower::limit::ConcurrencyLimitLayer;
use tower_http::{
    cors::{AllowOrigin, Any, CorsLayer},
    timeout::TimeoutLayer,
};

use crate::{
    config::{Action, Altcha, AuthMethod, Authentication, Config, Cors, ResultMode},
    database::{bind, row_to_json},
    wallet::{WalletGenerator, path_placeholders},
};

struct Route {
    action: String,
    auth: Vec<AuthMethod>,
    auth_optional: bool,
    altcha: bool,
    altcha_for_authenticated: bool,
    rate_limit: RateLimit,
}

struct ActionRequest {
    method: Method,
    path: String,
    path_params: HashMap<String, String>,
    query_params: HashMap<String, String>,
    headers: HeaderMap,
    ip: IpAddr,
    body: Bytes,
}

struct AppState {
    pool: AnyPool,
    actions: HashMap<String, Action>,
    routes: HashMap<(Method, String), Route>,
    auth: Authentication,
    altcha: Option<Altcha>,
    wallets: Option<WalletGenerator>,
    used_challenges: Mutex<HashMap<String, u64>>,
    challenge_rate_limit: RateLimit,
}

struct RateLimit {
    requests: u32,
    window: Duration,
    clients: Mutex<HashMap<IpAddr, (Instant, u32)>>,
}

impl RateLimit {
    fn check(&self, ip: IpAddr) -> Result<()> {
        if self.requests == 0 {
            return Ok(());
        }
        let mut clients = self.clients.lock().unwrap();
        let now = Instant::now();
        if !clients.contains_key(&ip) && clients.len() >= 100_000 {
            clients.retain(|_, (started, _)| now.duration_since(*started) < self.window);
            if clients.len() >= 100_000 {
                return Err(RateLimited(self.window.as_secs().max(1)).into());
            }
        }
        let entry = clients.entry(ip).or_insert((now, 0));
        if now.duration_since(entry.0) >= self.window {
            *entry = (now, 0);
        }
        if entry.1 >= self.requests {
            let retry_after = self
                .window
                .saturating_sub(now.duration_since(entry.0))
                .as_secs()
                .max(1);
            return Err(RateLimited(retry_after).into());
        }
        entry.1 += 1;
        Ok(())
    }
}

#[derive(Debug)]
struct Unauthorized;

impl fmt::Display for Unauthorized {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("invalid or missing credentials")
    }
}

impl std::error::Error for Unauthorized {}

#[derive(Debug)]
struct AltchaRejected;

impl fmt::Display for AltchaRejected {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("invalid, expired, or reused ALTCHA proof")
    }
}

impl std::error::Error for AltchaRejected {}

#[derive(Debug)]
struct RateLimited(u64);

impl fmt::Display for RateLimited {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("request limit exceeded")
    }
}

impl std::error::Error for RateLimited {}

#[derive(Debug)]
struct X402ConstructionFailed(String);

impl fmt::Display for X402ConstructionFailed {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for X402ConstructionFailed {}

#[derive(Debug)]
pub(crate) struct ClientError {
    status: StatusCode,
    message: String,
    x402: Option<Value>,
}

impl ClientError {
    pub(crate) fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
            x402: None,
        }
    }
}

impl fmt::Display for ClientError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ClientError {}

pub fn build_router(pool: AnyPool, config: Config) -> Result<Router> {
    if config.endpoints.is_empty() {
        bail!("no endpoints configured");
    }

    let wallets = config.wallets.map(WalletGenerator::new).transpose()?;
    let default_limits = config.server.limits;
    let prefix = config.server.prefix;
    let cors = config.server.cors;
    validate_limits(default_limits, "server")?;
    for (name, action) in &config.actions {
        if let Some(code) = action.status {
            let status = StatusCode::from_u16(code)
                .with_context(|| format!("action {name} has invalid status {code}"))?;
            if !status.is_success() {
                bail!("action {name} status must be a 2xx success status");
            }
        }
        for error in &action.errors {
            let status = StatusCode::from_u16(error.status).with_context(|| {
                format!("action {name} has invalid error status {}", error.status)
            })?;
            if !status.is_client_error() && !status.is_server_error() {
                bail!("action {name} error status must be between 400 and 599");
            }
            if error.x402.is_some() && status != StatusCode::PAYMENT_REQUIRED {
                bail!("action {name} x402 error mapping must have status 402");
            }
            if let Some(x402) = &error.x402 {
                if x402.column.is_empty() {
                    bail!("action {name} x402 column must not be empty");
                }
                if x402.params.iter().any(String::is_empty) {
                    bail!("action {name} x402 parameters must not be empty");
                }
            }
        }
        if let Some(stage) = &action.wallets {
            if action.result != ResultMode::One {
                bail!("action {name} with wallets must use result = \"one\"");
            }
            let generator = wallets.as_ref().with_context(|| {
                format!("action {name} uses wallets, but [wallets] is not configured")
            })?;
            if stage.profiles.is_empty() == stage.profile.is_none() {
                bail!(
                    "action {name} wallets must configure either profiles or profile, but not both"
                );
            }
            let mut seen = std::collections::HashSet::new();
            for profile in &stage.profiles {
                if generator.profile(profile).is_none() {
                    bail!("action {name} references unknown wallet profile {profile}");
                }
                if !seen.insert(profile) {
                    bail!("action {name} repeats wallet profile {profile}");
                }
            }
            if stage.profile.as_ref().is_some_and(String::is_empty) {
                bail!("action {name} wallet profile input must not be empty");
            }
            let profiles: Vec<_> = if stage.profile.is_some() {
                generator.profiles().collect()
            } else {
                stage
                    .profiles
                    .iter()
                    .map(|profile| generator.profile(profile).expect("profile was validated"))
                    .collect()
            };
            for profile in profiles {
                let placeholders = path_placeholders(&profile.path)?;
                if placeholders.len() != stage.values.len()
                    || placeholders
                        .iter()
                        .any(|placeholder| !stage.values.contains_key(placeholder))
                {
                    bail!(
                        "action {name} wallet values do not match profile {} path placeholders",
                        profile.name
                    );
                }
            }
            for expression in stage.values.values() {
                if expression.parse::<u32>().is_err()
                    && expression
                        .strip_prefix("$result.")
                        .is_none_or(str::is_empty)
                {
                    bail!("action {name} has invalid wallet path value {expression}");
                }
            }
            for parameter in &stage.params {
                validate_wallet_parameter(parameter)
                    .with_context(|| format!("action {name} has invalid wallet parameter"))?;
            }
        }
    }
    let mut router = Router::new();
    let mut routes = HashMap::new();
    let mut uses_altcha = false;

    for endpoint in config.endpoints {
        if !config.actions.contains_key(&endpoint.action) {
            bail!(
                "endpoint {} references unknown action {}",
                endpoint.path,
                endpoint.action
            );
        }

        let method = endpoint.method.parse::<Method>()?;
        let filter = method_filter(&method)?;
        let path = api_path(&prefix, &endpoint.path)?;
        let limits = default_limits.with_overrides(endpoint.limits);
        validate_limits(limits, &format!("endpoint {method} {path}"))?;
        if endpoint.auth_optional && endpoint.auth.is_empty() {
            bail!("endpoint {method} {path} sets auth_optional without auth methods");
        }
        if !endpoint.altcha_for_authenticated && !endpoint.altcha {
            bail!("endpoint {method} {path} sets altcha_for_authenticated without ALTCHA");
        }
        if !endpoint.altcha_for_authenticated && endpoint.auth.is_empty() {
            bail!("endpoint {method} {path} cannot skip ALTCHA without authentication methods");
        }
        for auth in &endpoint.auth {
            match auth {
                AuthMethod::Basic if config.auth.basic.is_none() => {
                    bail!("endpoint {path} requires missing [auth.basic] configuration");
                }
                AuthMethod::Bearer if config.auth.bearer.is_none() => {
                    bail!("endpoint {path} requires missing [auth.bearer] configuration");
                }
                _ => {}
            }
        }
        if routes.contains_key(&(method.clone(), path.clone())) {
            bail!("duplicate endpoint {method} {path}");
        }
        uses_altcha |= endpoint.altcha;
        routes.insert(
            (method, path.clone()),
            Route {
                action: endpoint.action,
                auth: endpoint.auth,
                auth_optional: endpoint.auth_optional,
                altcha: endpoint.altcha,
                altcha_for_authenticated: endpoint.altcha_for_authenticated,
                rate_limit: RateLimit {
                    requests: limits.requests,
                    window: Duration::from_secs(limits.window_seconds),
                    clients: Mutex::new(HashMap::new()),
                },
            },
        );
        let endpoint_router = Router::new()
            .route(&path, on(filter, handle))
            .layer(DefaultBodyLimit::max(limits.body_bytes))
            .layer(ConcurrencyLimitLayer::new(limits.concurrency))
            .layer(TimeoutLayer::with_status_code(
                StatusCode::REQUEST_TIMEOUT,
                Duration::from_secs(limits.timeout_seconds),
            ));
        router = router.merge(endpoint_router);
    }

    if uses_altcha && config.altcha.is_none() {
        bail!("an endpoint requires ALTCHA, but [altcha] is not configured");
    }
    if let Some(altcha) = &config.altcha {
        let path = api_path(&prefix, &altcha.path)?;
        if routes.contains_key(&(Method::GET, path.clone())) {
            bail!("ALTCHA challenge path {} conflicts with an endpoint", path);
        }
        router = router.route(&path, axum::routing::get(altcha_challenge));
    }

    let state = Arc::new(AppState {
        pool,
        actions: config.actions,
        routes,
        auth: config.auth,
        altcha: config.altcha,
        wallets,
        used_challenges: Mutex::new(HashMap::new()),
        challenge_rate_limit: RateLimit {
            requests: default_limits.requests,
            window: Duration::from_secs(default_limits.window_seconds),
            clients: Mutex::new(HashMap::new()),
        },
    });
    let mut router = router.with_state(state);
    if let Some(cors) = cors {
        router = router.layer(cors_layer(cors)?);
    }
    Ok(router)
}

fn validate_wallet_parameter(parameter: &str) -> Result<()> {
    match parameter {
        "$profile.name"
        | "$profile.caip2"
        | "$profile.max_addresses"
        | "$wallet.address"
        | "$wallet.derivation_path" => Ok(()),
        value
            if value
                .strip_prefix("$result.")
                .is_some_and(|column| !column.is_empty()) =>
        {
            Ok(())
        }
        _ => bail!("unsupported parameter {parameter}"),
    }
}

fn cors_layer(cors: Cors) -> Result<CorsLayer> {
    if cors.origins.is_empty() {
        bail!("server.cors.origins must contain at least one origin");
    }
    let origins = cors
        .origins
        .into_iter()
        .map(|origin| {
            origin
                .parse::<HeaderValue>()
                .with_context(|| format!("invalid CORS origin {origin}"))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods(Any)
        .allow_headers([AUTHORIZATION, CONTENT_TYPE]))
}

fn validate_limits(limits: crate::config::Limits, context: &str) -> Result<()> {
    if limits.body_bytes == 0 || limits.timeout_seconds == 0 || limits.concurrency == 0 {
        bail!("{context}: body_bytes, timeout_seconds, and concurrency must be greater than zero");
    }
    if limits.requests > 0 && limits.window_seconds == 0 {
        bail!("{context}: window_seconds must be greater than zero when rate limiting is enabled");
    }
    Ok(())
}

fn api_path(prefix: &str, path: &str) -> Result<String> {
    if !path.starts_with('/') {
        bail!("endpoint path {path} must start with /");
    }
    let prefix = prefix.trim_matches('/');
    if prefix.is_empty() {
        Ok(path.to_owned())
    } else {
        Ok(format!("/{prefix}{path}"))
    }
}

fn method_filter(method: &Method) -> Result<MethodFilter> {
    Ok(match *method {
        Method::GET => MethodFilter::GET,
        Method::POST => MethodFilter::POST,
        Method::PUT => MethodFilter::PUT,
        Method::PATCH => MethodFilter::PATCH,
        Method::DELETE => MethodFilter::DELETE,
        Method::HEAD => MethodFilter::HEAD,
        Method::OPTIONS => MethodFilter::OPTIONS,
        Method::TRACE => MethodFilter::TRACE,
        _ => bail!("unsupported HTTP method {method}"),
    })
}

async fn handle(
    State(state): State<Arc<AppState>>,
    ConnectInfo(address): ConnectInfo<SocketAddr>,
    path: MatchedPath,
    Path(path_params): Path<HashMap<String, String>>,
    Query(query_params): Query<HashMap<String, String>>,
    request: (HeaderMap, Method, Bytes),
) -> Response {
    let (headers, method, body) = request;
    let route = state
        .routes
        .get(&(method.clone(), path.as_str().to_owned()));
    let Some(route) = route else {
        return error_response(anyhow::anyhow!("route has no action"));
    };
    if let Err(error) = route.rate_limit.check(address.ip()) {
        return error_response(error);
    }
    let request = ActionRequest {
        method,
        path: path.as_str().to_owned(),
        path_params,
        query_params,
        headers,
        ip: address.ip(),
        body,
    };
    match run_action(state, request).await {
        Ok(response) => response,
        Err(error) => error_response(error),
    }
}

fn error_response(error: anyhow::Error) -> Response {
    let (status, message) = if error.downcast_ref::<Unauthorized>().is_some() {
        (StatusCode::UNAUTHORIZED, error.to_string())
    } else if error.downcast_ref::<AltchaRejected>().is_some() {
        (StatusCode::FORBIDDEN, error.to_string())
    } else if let Some(error) = error.downcast_ref::<ClientError>() {
        if let Some(payload) = &error.x402 {
            let bytes = match serde_json::to_vec(payload) {
                Ok(bytes) => bytes,
                Err(error) => return error_response(error.into()),
            };
            let mut response = (StatusCode::PAYMENT_REQUIRED, bytes.clone()).into_response();
            response
                .headers_mut()
                .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
            response.headers_mut().insert(
                "PAYMENT-REQUIRED",
                HeaderValue::from_str(&BASE64.encode(bytes))
                    .expect("base64 is a valid header value"),
            );
            return response;
        }
        (error.status, error.message.clone())
    } else if let Some(error) = error.downcast_ref::<sqlx::Error>() {
        match error {
            sqlx::Error::RowNotFound => (StatusCode::NOT_FOUND, "resource not found".into()),
            sqlx::Error::Database(error)
                if error.is_unique_violation() || error.is_foreign_key_violation() =>
            {
                (StatusCode::CONFLICT, "database constraint conflict".into())
            }
            _ => {
                eprintln!("{error}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal server error".into(),
                )
            }
        }
    } else if error.downcast_ref::<RateLimited>().is_some() {
        (StatusCode::TOO_MANY_REQUESTS, error.to_string())
    } else {
        eprintln!("{error:#}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal server error".into(),
        )
    };
    let mut response = (status, Json(json!({ "error": message }))).into_response();
    if let Some(limit) = error.downcast_ref::<RateLimited>() {
        response.headers_mut().insert(
            RETRY_AFTER,
            HeaderValue::from_str(&limit.0.to_string()).unwrap(),
        );
    }
    response
}

async fn action_error(
    error: sqlx::Error,
    action: &Action,
    pool: &AnyPool,
    input: &Map<String, Value>,
) -> anyhow::Error {
    let response = error.as_database_error().and_then(|database| {
        action
            .errors
            .iter()
            .find(|response| response.database_message == database.message())
    });
    match response {
        Some(response) => {
            let x402 = match &response.x402 {
                Some(x402) => match x402_payload(x402, pool, input).await {
                    Ok(payload) => Some(payload),
                    Err(error) => {
                        return X402ConstructionFailed(format!(
                            "could not construct x402 payment requirement: {error:#}"
                        ))
                        .into();
                    }
                },
                None => None,
            };
            ClientError {
                status: StatusCode::from_u16(response.status).expect("error status was validated"),
                message: response.message.clone(),
                x402,
            }
            .into()
        }
        None => error.into(),
    }
}

async fn x402_payload(
    x402: &crate::config::ActionX402,
    pool: &AnyPool,
    input: &Map<String, Value>,
) -> Result<Value> {
    let mut query = sqlx::query(AssertSqlSafe(x402.sql.as_str()));
    for name in &x402.params {
        let value = input
            .get(name)
            .with_context(|| format!("x402 missing parameter {name}"))?;
        query = bind(query, value)?;
    }
    let row = query.fetch_one(pool).await.context("x402 query failed")?;
    let payload: String = row
        .try_get(x402.column.as_str())
        .with_context(|| format!("x402 query has no string column {}", x402.column))?;
    let payload = serde_json::from_str(&payload).context("x402 payload is not JSON")?;
    validate_x402_payload(&payload)?;
    Ok(payload)
}

fn validate_x402_payload(payload: &Value) -> Result<()> {
    let root = payload
        .as_object()
        .context("x402 payload must be an object")?;
    if root.get("x402Version").and_then(Value::as_u64) != Some(2) {
        bail!("x402 payload x402Version must be numeric 2");
    }
    if !root.get("resource").is_some_and(Value::is_object) {
        bail!("x402 payload resource must be an object");
    }
    let accepts = root
        .get("accepts")
        .and_then(Value::as_array)
        .context("x402 payload accepts must be an array")?;
    for accept in accepts {
        let accept = accept
            .as_object()
            .context("x402 accept must be an object")?;
        for field in ["scheme", "network", "amount", "asset", "payTo"] {
            if !accept.get(field).is_some_and(Value::is_string) {
                bail!("x402 accept {field} must be a string");
            }
        }
        if !accept
            .get("maxTimeoutSeconds")
            .is_some_and(|value| value.as_u64().is_some())
        {
            bail!("x402 accept maxTimeoutSeconds must be a nonnegative integer");
        }
    }
    if let Some(extensions) = root.get("extensions") {
        let extensions = extensions
            .as_object()
            .context("x402 extensions must be an object")?;
        for extension in extensions.values() {
            let extension = extension
                .as_object()
                .context("x402 extension must be an object")?;
            for field in ["info", "schema"] {
                if !extension.get(field).is_some_and(Value::is_object) {
                    bail!("x402 extension {field} must be an object");
                }
            }
        }
    }
    Ok(())
}

async fn altcha_challenge(
    State(state): State<Arc<AppState>>,
    ConnectInfo(address): ConnectInfo<SocketAddr>,
) -> Response {
    if let Err(error) = state.challenge_rate_limit.check(address.ip()) {
        return error_response(error);
    }
    let Some(config) = &state.altcha else {
        return StatusCode::NOT_FOUND.into_response();
    };
    match new_altcha_challenge(config, address.ip()) {
        Ok(challenge) => (
            [(CACHE_CONTROL, HeaderValue::from_static("no-store"))],
            Json(challenge),
        )
            .into_response(),
        Err(error) => error_response(error),
    }
}

fn new_altcha_challenge(config: &Altcha, ip: IpAddr) -> Result<Challenge> {
    let data = config
        .bind_ip
        .then(|| BTreeMap::from([("client".into(), Value::String(client_binding(config, ip)))]));
    create_challenge(CreateChallengeOptions {
        algorithm: config.algorithm.clone(),
        cost: config.cost,
        counter: Some(rand::rng().random_range(1..=config.max_number.max(1))),
        expires_at: Some(unix_time().saturating_add(config.expires_seconds)),
        data,
        hmac_signature_secret: Some(config.secret.clone()),
        hmac_key_signature_secret: Some(config.key_secret.clone()),
        ..Default::default()
    })
    .map_err(|error| anyhow::anyhow!(error.to_string()))
}

fn client_binding(config: &Altcha, ip: IpAddr) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(config.secret.as_bytes())
        .expect("HMAC accepts keys of any size");
    mac.update(ip.to_string().as_bytes());
    BASE64.encode(mac.finalize().into_bytes())
}

async fn run_action(state: Arc<AppState>, request: ActionRequest) -> Result<Response> {
    let route = state
        .routes
        .get(&(request.method, request.path))
        .context("route has no action")?;
    let action = state
        .actions
        .get(&route.action)
        .context("action not found")?;
    let mut input = Map::new();

    for (key, value) in request.query_params.into_iter().chain(request.path_params) {
        input.insert(key, Value::String(value));
    }
    if !request.body.is_empty() {
        let value: Value = serde_json::from_slice(&request.body)
            .map_err(|_| ClientError::bad_request("body must be a JSON object"))?;
        let Value::Object(object) = value else {
            return Err(ClientError::bad_request("body must be a JSON object").into());
        };
        input.extend(object);
    }
    if route.altcha && route.altcha_for_authenticated {
        verify_altcha(
            &state,
            input
                .remove("altcha")
                .and_then(|value| value.as_str().map(str::to_owned)),
            request.ip,
        )?;
    }
    let owner = if route.auth.is_empty()
        || (route.auth_optional && !request.headers.contains_key(AUTHORIZATION))
    {
        None
    } else {
        Some(authenticate(&state, &route.auth, &request.headers).await?)
    };
    if route.altcha && !route.altcha_for_authenticated && owner.is_none() {
        verify_altcha(
            &state,
            input
                .remove("altcha")
                .and_then(|value| value.as_str().map(str::to_owned)),
            request.ip,
        )?;
    }
    if let Some(owner) = owner {
        input.insert("$owner".into(), owner);
    }
    if action.params.iter().any(|name| name == "$token") {
        input.insert(
            "$token".into(),
            Value::String(SaltString::generate(&mut OsRng).to_string()),
        );
    }
    let selected_profile = if let Some(field) = action
        .wallets
        .as_ref()
        .and_then(|wallets| wallets.profile.as_ref())
    {
        let name = input
            .get(field)
            .and_then(Value::as_str)
            .ok_or_else(|| ClientError::bad_request(format!("{field} must be a string")))?;
        let profile = state
            .wallets
            .as_ref()
            .and_then(|wallets| wallets.profile(name))
            .ok_or_else(|| ClientError::bad_request(format!("unknown wallet profile {name}")))?;
        input.insert("$profile.name".into(), Value::String(profile.name.clone()));
        input.insert(
            "$profile.caip2".into(),
            Value::String(profile.caip2.clone()),
        );
        input.insert(
            "$profile.max_addresses".into(),
            Value::Number(profile.max_addresses.into()),
        );
        Some(profile)
    } else {
        None
    };
    for name in &action.hash {
        let password = input
            .get(name)
            .and_then(Value::as_str)
            .ok_or_else(|| ClientError::bad_request(format!("{name} must be a string")))?;
        let hash = Argon2::default()
            .hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng))
            .map_err(|error| anyhow::anyhow!("could not hash password: {error}"))?
            .to_string();
        input.insert(name.clone(), Value::String(hash));
    }

    let mut query = sqlx::query(AssertSqlSafe(action.sql.as_str()));
    for name in &action.params {
        let value = input
            .get(name)
            .ok_or_else(|| ClientError::bad_request(format!("missing parameter {name}")))?;
        query = bind(query, value)?;
    }

    let value = if let Some(wallets) = &action.wallets {
        let generator = state
            .wallets
            .as_ref()
            .context("wallet generator is not configured")?;
        let mut transaction = state
            .pool
            .begin()
            .await
            .context("could not start wallet action transaction")?;
        let row = match query.fetch_one(&mut *transaction).await {
            Ok(row) => row,
            Err(error) => {
                transaction
                    .rollback()
                    .await
                    .context("could not roll back wallet action transaction")?;
                return Err(action_error(error, action, &state.pool, &input).await);
            }
        };
        let value = row_to_json(row)?;
        let path_values = wallet_path_values(&wallets.values, &value)?;
        let profiles: Vec<_> = if let Some(profile) = selected_profile {
            vec![profile]
        } else {
            wallets
                .profiles
                .iter()
                .map(|name| {
                    generator
                        .profile(name)
                        .context("validated wallet profile is missing")
                })
                .collect::<Result<_>>()?
        };

        for profile in profiles {
            let generated = generator.derive(profile, &path_values)?;
            let values = wallets
                .params
                .iter()
                .map(|parameter| wallet_parameter(parameter, &value, profile, &generated))
                .collect::<Result<Vec<_>>>()?;
            let mut insert = sqlx::query(AssertSqlSafe(wallets.sql.as_str()));
            for value in &values {
                insert = bind(insert, value)?;
            }
            let result = match insert.execute(&mut *transaction).await {
                Ok(result) => result,
                Err(error) => {
                    transaction
                        .rollback()
                        .await
                        .context("could not roll back wallet action transaction")?;
                    return Err(action_error(error, action, &state.pool, &input).await);
                }
            };
            if result.rows_affected() != 1 {
                bail!("wallet persistence must affect exactly one row");
            }
        }
        transaction
            .commit()
            .await
            .context("could not commit wallet action transaction")?;
        value
    } else {
        match action.result {
            ResultMode::Execute => {
                let result = match query.execute(&state.pool).await {
                    Ok(result) => result,
                    Err(error) => {
                        return Err(action_error(error, action, &state.pool, &input).await);
                    }
                };
                json!({ "rows_affected": result.rows_affected() })
            }
            ResultMode::One => {
                let row = match query.fetch_one(&state.pool).await {
                    Ok(row) => row,
                    Err(error) => {
                        return Err(action_error(error, action, &state.pool, &input).await);
                    }
                };
                row_to_json(row)?
            }
            ResultMode::Optional => {
                let row = match query.fetch_optional(&state.pool).await {
                    Ok(row) => row,
                    Err(error) => {
                        return Err(action_error(error, action, &state.pool, &input).await);
                    }
                };
                row.map(row_to_json).transpose()?.unwrap_or(Value::Null)
            }
            ResultMode::Many => {
                let rows = match query.fetch_all(&state.pool).await {
                    Ok(rows) => rows,
                    Err(error) => {
                        return Err(action_error(error, action, &state.pool, &input).await);
                    }
                };
                Value::Array(
                    rows.into_iter()
                        .map(row_to_json)
                        .collect::<Result<Vec<_>>>()?,
                )
            }
        }
    };
    let status = action
        .status
        .map(StatusCode::from_u16)
        .transpose()?
        .unwrap_or(StatusCode::OK);
    let mut response = (status, Json(value)).into_response();
    if action.no_store {
        response
            .headers_mut()
            .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    }
    Ok(response)
}

fn wallet_path_values(
    expressions: &HashMap<String, String>,
    result: &Value,
) -> Result<HashMap<String, u32>> {
    expressions
        .iter()
        .map(|(name, expression)| {
            let value = if expression.starts_with("$result.") {
                result_value(result, expression)?
                    .as_u64()
                    .and_then(|value| u32::try_from(value).ok())
                    .with_context(|| {
                        format!("wallet path value {name} must be an unsigned integer")
                    })?
            } else {
                expression.parse::<u32>()?
            };
            if value >= 1 << 31 {
                bail!("wallet path value {name} must be less than 2^31");
            }
            Ok((name.clone(), value))
        })
        .collect()
}

fn result_value<'a>(result: &'a Value, reference: &str) -> Result<&'a Value> {
    let column = reference
        .strip_prefix("$result.")
        .context("result reference must start with $result.")?;
    result
        .get(column)
        .with_context(|| format!("action result has no column {column}"))
}

fn wallet_parameter(
    parameter: &str,
    result: &Value,
    profile: &crate::config::WalletProfile,
    generated: &crate::wallet::GeneratedAddress,
) -> Result<Value> {
    Ok(match parameter {
        "$profile.name" => Value::String(profile.name.clone()),
        "$profile.caip2" => Value::String(profile.caip2.clone()),
        "$profile.max_addresses" => Value::Number(profile.max_addresses.into()),
        "$wallet.address" => Value::String(generated.address.clone()),
        "$wallet.derivation_path" => Value::String(generated.derivation_path.clone()),
        parameter if parameter.starts_with("$result.") => result_value(result, parameter)?.clone(),
        _ => bail!("unsupported wallet parameter {parameter}"),
    })
}

fn verify_altcha(state: &AppState, encoded: Option<String>, ip: IpAddr) -> Result<()> {
    let config = state.altcha.as_ref().ok_or(AltchaRejected)?;
    let encoded = encoded.ok_or(AltchaRejected)?;
    let bytes = BASE64.decode(&encoded).map_err(|_| AltchaRejected)?;
    let payload: Payload = serde_json::from_slice(&bytes).map_err(|_| AltchaRejected)?;
    let result = verify_solution(VerifySolutionOptions {
        hmac_key_signature_secret: Some(config.key_secret.clone()),
        ..VerifySolutionOptions::new(&payload.challenge, &payload.solution, &config.secret)
    })
    .map_err(|_| AltchaRejected)?;
    if !result.verified {
        return Err(AltchaRejected.into());
    }
    let expected_binding = client_binding(config, ip);
    if config.bind_ip
        && payload
            .challenge
            .parameters
            .data
            .as_ref()
            .and_then(|data| data.get("client"))
            .and_then(Value::as_str)
            != Some(expected_binding.as_str())
    {
        return Err(AltchaRejected.into());
    }

    let signature = payload.challenge.signature.ok_or(AltchaRejected)?;
    let expires_at = payload
        .challenge
        .parameters
        .expires_at
        .ok_or(AltchaRejected)?;
    let now = unix_time();
    let mut used = state.used_challenges.lock().unwrap();
    used.retain(|_, expires| *expires > now);
    if used.insert(signature, expires_at).is_some() {
        return Err(AltchaRejected.into());
    }
    Ok(())
}

fn unix_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

async fn authenticate(
    state: &AppState,
    allowed: &[AuthMethod],
    headers: &HeaderMap,
) -> Result<Value> {
    let authorization = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or(Unauthorized)?;

    for method in allowed {
        match method {
            AuthMethod::Basic => {
                let Some(encoded) = authorization.strip_prefix("Basic ") else {
                    continue;
                };
                let decoded = BASE64.decode(encoded).map_err(|_| Unauthorized)?;
                let credentials = String::from_utf8(decoded).map_err(|_| Unauthorized)?;
                let (username, password) = credentials.split_once(':').ok_or(Unauthorized)?;
                let config = state
                    .auth
                    .basic
                    .as_ref()
                    .context("Basic auth is not configured")?;
                let Some(row) = sqlx::query(AssertSqlSafe(config.sql.as_str()))
                    .bind(username)
                    .fetch_optional(&state.pool)
                    .await?
                else {
                    return Err(Unauthorized.into());
                };
                let user = row_to_json(row)?;
                let hash = user
                    .get(&config.password)
                    .and_then(Value::as_str)
                    .ok_or(Unauthorized)?;
                if Argon2::default()
                    .verify_password(
                        password.as_bytes(),
                        &PasswordHash::new(hash).map_err(|_| Unauthorized)?,
                    )
                    .is_err()
                {
                    return Err(Unauthorized.into());
                }
                return user
                    .get(&config.owner)
                    .cloned()
                    .ok_or_else(|| Unauthorized.into());
            }
            AuthMethod::Bearer => {
                let Some(token) = authorization.strip_prefix("Bearer ") else {
                    continue;
                };
                let config = state
                    .auth
                    .bearer
                    .as_ref()
                    .context("Bearer auth is not configured")?;
                let Some(row) = sqlx::query(AssertSqlSafe(config.sql.as_str()))
                    .bind(token)
                    .fetch_optional(&state.pool)
                    .await?
                else {
                    return Err(Unauthorized.into());
                };
                let session = row_to_json(row)?;
                return session
                    .get(&config.owner)
                    .cloned()
                    .ok_or_else(|| Unauthorized.into());
            }
        }
    }
    Err(Unauthorized.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use altcha::{SolveChallengeOptions, solve_challenge};
    use sqlx::any::AnyPoolOptions;

    #[tokio::test]
    async fn altcha_proof_can_only_be_used_once() {
        sqlx::any::install_default_drivers();
        let altcha = Altcha {
            secret: "test-secret".into(),
            key_secret: "test-key-secret".into(),
            path: "/challenge".into(),
            algorithm: "SHA-256".into(),
            cost: 1,
            max_number: 10,
            expires_seconds: 60,
            bind_ip: true,
        };
        let ip = "127.0.0.1".parse().unwrap();
        let challenge = new_altcha_challenge(&altcha, ip).unwrap();
        let solution = solve_challenge(SolveChallengeOptions::new(&challenge))
            .unwrap()
            .unwrap();
        let encoded = BASE64.encode(
            serde_json::to_vec(&Payload {
                challenge,
                solution,
            })
            .unwrap(),
        );
        let state = AppState {
            pool: AnyPoolOptions::new()
                .connect_lazy("sqlite::memory:")
                .unwrap(),
            actions: HashMap::new(),
            routes: HashMap::new(),
            auth: Authentication::default(),
            altcha: Some(altcha),
            wallets: None,
            used_challenges: Mutex::new(HashMap::new()),
            challenge_rate_limit: RateLimit {
                requests: 0,
                window: Duration::from_secs(60),
                clients: Mutex::new(HashMap::new()),
            },
        };

        assert!(
            verify_altcha(&state, Some(encoded.clone()), "127.0.0.2".parse().unwrap()).is_err()
        );
        assert!(verify_altcha(&state, Some(encoded.clone()), ip).is_ok());
        assert!(verify_altcha(&state, Some(encoded), ip).is_err());
    }

    #[test]
    fn rate_limit_is_per_ip() {
        let limit = RateLimit {
            requests: 2,
            window: Duration::from_secs(60),
            clients: Mutex::new(HashMap::new()),
        };
        let first = "127.0.0.1".parse().unwrap();
        let second = "127.0.0.2".parse().unwrap();

        assert!(limit.check(first).is_ok());
        assert!(limit.check(first).is_ok());
        assert!(limit.check(first).is_err());
        assert!(limit.check(second).is_ok());
    }

    #[test]
    fn prefix_is_applied_to_every_path() {
        assert_eq!(api_path("v1", "/users").unwrap(), "/v1/users");
        assert_eq!(api_path("/v1/", "/challenge").unwrap(), "/v1/challenge");
        assert_eq!(api_path("", "/users").unwrap(), "/users");
        assert!(api_path("v1", "users").is_err());
    }

    #[test]
    fn rate_limit_response_includes_retry_after() {
        let response = error_response(RateLimited(42).into());
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(response.headers().get(RETRY_AFTER).unwrap(), "42");
    }

    #[tokio::test]
    async fn duplicate_routes_are_rejected_at_startup() {
        sqlx::any::install_default_drivers();
        let config = Config::parse(
            r#"
            [database]
            url = "sqlite::memory:"

            [[endpoints]]
            method = "GET"
            path = "/items"
            action = "items"

            [[endpoints]]
            method = "GET"
            path = "/items"
            action = "items"

            [actions.items]
            sql = "SELECT 1 AS id"
            "#,
        )
        .unwrap();
        let pool = AnyPoolOptions::new()
            .connect_lazy("sqlite::memory:")
            .unwrap();

        assert!(build_router(pool, config).is_err());
    }

    #[tokio::test]
    async fn missing_authentication_configuration_is_rejected_at_startup() {
        sqlx::any::install_default_drivers();
        let config = Config::parse(
            r#"
            [database]
            url = "sqlite::memory:"

            [[endpoints]]
            method = "GET"
            path = "/private"
            action = "private"
            auth = ["bearer"]

            [actions.private]
            sql = "SELECT 1 AS id"
            "#,
        )
        .unwrap();
        let pool = AnyPoolOptions::new()
            .connect_lazy("sqlite::memory:")
            .unwrap();

        assert!(build_router(pool, config).is_err());
    }

    #[tokio::test]
    async fn non_success_action_status_is_rejected_at_startup() {
        sqlx::any::install_default_drivers();
        let config = Config::parse(
            r#"
            [database]
            url = "sqlite::memory:"

            [[endpoints]]
            method = "GET"
            path = "/items"
            action = "items"

            [actions.items]
            sql = "SELECT 1 AS id"
            status = 404
            "#,
        )
        .unwrap();
        let pool = AnyPoolOptions::new()
            .connect_lazy("sqlite::memory:")
            .unwrap();

        let error = build_router(pool, config).err().unwrap();
        assert!(error.to_string().contains("2xx success"));
    }
}
