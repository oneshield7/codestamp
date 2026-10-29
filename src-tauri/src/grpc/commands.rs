use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use prost_reflect::{DescriptorPool, DynamicMessage, MethodDescriptor};
use serde::Deserialize;
use serde_json::Value;
use tauri::{State, WebviewWindow};
use tonic::metadata::MetadataMap;

use super::client::{self, Address, GrpcReply};
use super::descriptors::{self, ServiceInfo};
use crate::auth::apply::{apply_auth_to, effective_auth};
use crate::auth::models::{ApiKeyLocation, Auth};
use crate::auth::oauth;
use crate::dialogs::pick_file;
use crate::environment::lookup::{resolve_auth, resolve_text};
use crate::environment::vault::Vault;
use crate::error::AppError;
use crate::preferences::network::load_config;
use crate::request::client::HttpClient;
use crate::request::network::NetworkConfig;
use crate::request::rules::{
    MAX_KEY_VALUE_LENGTH, MAX_KEY_VALUE_ROWS, MAX_PATH_LENGTH, MAX_REQUEST_BODY_BYTES,
    MAX_URL_LENGTH,
};
use crate::request::types::NameValue;
use crate::request::uploads::{self, PickedFile};
use crate::storage::db::Database;
use crate::storage::models::GrpcSource;
use crate::storage::validate::check_optional_id;

/// Descriptors are asked for again after this long, or when the user loads services.
const CACHE_TTL: Duration = Duration::from_secs(600);
const MAX_POOLS: usize = 20;

/// Recently loaded descriptors, by server (reflection) or file (proto).
#[derive(Default)]
pub struct GrpcDescriptors {
    pools: Mutex<HashMap<String, (DescriptorPool, Instant)>>,
}

impl GrpcDescriptors {
    fn lock(&self) -> MutexGuard<'_, HashMap<String, (DescriptorPool, Instant)>> {
        self.pools.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Where to call and how: the address, where the descriptors come from, metadata and auth.
/// Values may contain `{{variables}}`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GrpcTarget {
    pub url: String,
    pub source: GrpcSource,
    #[serde(default)]
    pub proto_file: String,
    #[serde(default)]
    pub metadata: Vec<NameValue>,
    #[serde(default)]
    pub auth: Option<Auth>,
    #[serde(default)]
    pub collection_id: Option<String>,
    #[serde(default)]
    pub request_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GrpcInvoke {
    pub target: GrpcTarget,
    /// `package.Service/Method`.
    pub method: String,
    /// JSON: an object, or an array of objects for methods that stream from the client.
    pub message: String,
}

struct Prepared {
    address: Address,
    config: NetworkConfig,
    metadata: MetadataMap,
    key: String,
    proto: Option<PathBuf>,
}

/// Lists the services and methods, asking again even if they were loaded before.
#[tauri::command]
pub async fn grpc_describe(
    client: State<'_, HttpClient>,
    db: State<'_, Database>,
    vault: State<'_, Vault>,
    cache: State<'_, GrpcDescriptors>,
    target: GrpcTarget,
) -> Result<Vec<ServiceInfo>, AppError> {
    let prepared = prepare(&client, &db, &vault, &target).await?;
    let pool = load(&cache, &prepared, true).await?;
    Ok(descriptors::services(&pool))
}

#[tauri::command]
pub async fn grpc_invoke(
    client: State<'_, HttpClient>,
    db: State<'_, Database>,
    vault: State<'_, Vault>,
    cache: State<'_, GrpcDescriptors>,
    input: GrpcInvoke,
) -> Result<GrpcReply, AppError> {
    if input.message.len() > MAX_REQUEST_BODY_BYTES {
        return Err(AppError::invalid("The message is larger than 10 MB"));
    }
    let prepared = prepare(&client, &db, &vault, &input.target).await?;
    let pool = load(&cache, &prepared, false).await?;
    let method = find_method(&pool, &input.method)?;
    let text = resolve_text(
        &db,
        &vault,
        input.message,
        input.target.collection_id.as_deref(),
    )?;
    let messages = parse_messages(&method, &text)?;
    let channel =
        client::connect(&prepared.address, &prepared.config, prepared.config.timeout).await?;
    client::invoke(channel, &method, messages, prepared.metadata).await
}

/// Lets the user pick a `.proto` file; only picked files (and the imports beside them) are read.
#[tauri::command]
pub async fn pick_proto_file(
    window: WebviewWindow,
    db: State<'_, Database>,
) -> Result<Option<PickedFile>, AppError> {
    let Some(path) = pick_file(
        &window,
        "Choose a .proto file",
        "Protocol Buffers",
        &["proto"],
    )
    .await
    else {
        return Ok(None);
    };
    db.with(|conn| uploads::grant(conn, &path)).map(Some)
}

async fn prepare(
    client: &HttpClient,
    db: &Database,
    vault: &Vault,
    target: &GrpcTarget,
) -> Result<Prepared, AppError> {
    check_optional_id(target.collection_id.as_deref())?;
    check_optional_id(target.request_id.as_deref())?;
    let too_long = target.url.len() > MAX_URL_LENGTH
        || target.proto_file.len() > MAX_PATH_LENGTH
        || target.metadata.len() > MAX_KEY_VALUE_ROWS
        || target.metadata.iter().any(|row| {
            row.name.len() > MAX_KEY_VALUE_LENGTH || row.value.len() > MAX_KEY_VALUE_LENGTH
        });
    if too_long {
        return Err(AppError::invalid(
            "The address, metadata or file path is too long",
        ));
    }
    let collection_id = target.collection_id.as_deref();
    let url = resolve_text(db, vault, target.url.clone(), collection_id)?;
    let address = client::address(&url)?;
    let mut rows = Vec::with_capacity(target.metadata.len());
    for row in target
        .metadata
        .iter()
        .filter(|row| !row.name.trim().is_empty())
    {
        rows.push(NameValue {
            name: row.name.trim().to_owned(),
            value: resolve_text(db, vault, row.value.clone(), collection_id)?,
        });
    }
    add_auth(client, db, vault, target, &mut rows).await?;
    let proto = match target.source {
        GrpcSource::Reflection => None,
        GrpcSource::Proto => {
            let path = target.proto_file.trim();
            if path.is_empty() {
                return Err(AppError::invalid(
                    "Choose the .proto file that describes the service",
                ));
            }
            if !db.with(|conn| uploads::is_granted(conn, path))? {
                return Err(AppError::invalid(
                    "Choose the .proto file again: only files picked on this device are read",
                ));
            }
            Some(PathBuf::from(path))
        }
    };
    let key = match &proto {
        Some(path) => format!("proto {}", path.display()),
        None => format!("reflection {url}"),
    };
    Ok(Prepared {
        address,
        config: db.with(|conn| load_config(conn, vault))?,
        metadata: client::metadata(&rows)?,
        key,
        proto,
    })
}

/// The request's auth becomes metadata, the way it becomes headers for HTTP.
async fn add_auth(
    client: &HttpClient,
    db: &Database,
    vault: &Vault,
    target: &GrpcTarget,
    rows: &mut Vec<NameValue>,
) -> Result<(), AppError> {
    let collection_id = target.collection_id.as_deref();
    let owner = oauth::token_owner(
        target.auth.as_ref(),
        target.request_id.as_deref(),
        collection_id,
    );
    let auth = effective_auth(db, target.auth.clone(), collection_id)?;
    let auth = match resolve_auth(db, vault, auth, collection_id)? {
        Auth::OAuth2(settings) => {
            let token = oauth::access_token(client, db, vault, owner.as_deref(), &settings).await?;
            oauth::bearer(&settings, token)
        }
        Auth::Digest { .. } | Auth::AwsSigV4 { .. } => {
            return Err(AppError::invalid(
                "gRPC requests support Bearer, Basic, API key (header) and OAuth 2.0 auth",
            ));
        }
        Auth::ApiKey {
            location: ApiKeyLocation::Query,
            ..
        } => {
            return Err(AppError::invalid(
                "gRPC has no query string; send the API key in a header",
            ));
        }
        other => other,
    };
    let mut ignored = Vec::new();
    apply_auth_to(rows, &mut ignored, &auth)
}

async fn load(
    cache: &GrpcDescriptors,
    prepared: &Prepared,
    refresh: bool,
) -> Result<DescriptorPool, AppError> {
    if !refresh
        && let Some((pool, loaded)) = cache.lock().get(&prepared.key)
        && loaded.elapsed() < CACHE_TTL
    {
        return Ok(pool.clone());
    }
    let pool = match &prepared.proto {
        Some(path) => {
            let path = path.clone();
            tauri::async_runtime::spawn_blocking(move || descriptors::from_proto_file(&path))
                .await
                .map_err(|_| AppError::invalid("The .proto file could not be read"))??
        }
        None => {
            let channel =
                client::connect(&prepared.address, &prepared.config, prepared.config.timeout)
                    .await?;
            descriptors::from_reflection(channel, &prepared.metadata).await?
        }
    };
    let mut pools = cache.lock();
    if pools.len() >= MAX_POOLS
        && let Some(oldest) = pools
            .iter()
            .min_by_key(|(_, (_, loaded))| *loaded)
            .map(|(key, _)| key.clone())
    {
        pools.remove(&oldest);
    }
    pools.insert(prepared.key.clone(), (pool.clone(), Instant::now()));
    Ok(pool)
}

fn find_method(pool: &DescriptorPool, full_name: &str) -> Result<MethodDescriptor, AppError> {
    let (service, method) = full_name
        .trim()
        .split_once('/')
        .ok_or_else(|| AppError::invalid("Choose a method"))?;
    pool.get_service_by_name(service)
        .and_then(|service| {
            service
                .methods()
                .find(|candidate| candidate.name() == method)
        })
        .ok_or_else(|| {
            AppError::InvalidInput(format!(
                "The server has no method {full_name}; load the services again"
            ))
        })
}

/// One message, or one per element for methods that stream from the client.
fn parse_messages(method: &MethodDescriptor, text: &str) -> Result<Vec<DynamicMessage>, AppError> {
    let value: Value = if text.trim().is_empty() {
        Value::Object(serde_json::Map::new())
    } else {
        serde_json::from_str(text)
            .map_err(|_| AppError::invalid("The message is not valid JSON"))?
    };
    let items = match (method.is_client_streaming(), value) {
        (true, Value::Array(items)) => items,
        (true, _) => {
            return Err(AppError::invalid(
                "This method streams from the client: write a JSON array of messages",
            ));
        }
        (false, value) => vec![value],
    };
    let input = method.input();
    items
        .into_iter()
        .map(|item| {
            DynamicMessage::deserialize(input.clone(), item).map_err(|error| {
                AppError::InvalidInput(format!(
                    "The message does not fit {}: {error}",
                    input.name()
                ))
            })
        })
        .collect()
}
