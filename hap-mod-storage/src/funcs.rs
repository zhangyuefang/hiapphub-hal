use hap_common::{hap_fn, HapError};
use rusqlite::{params, Connection};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::collections::HashMap;

static CONNECTIONS: std::sync::LazyLock<Mutex<HashMap<String, Connection>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

fn storage_dir(custom: Option<&str>) -> PathBuf {
    if let Some(d) = custom {
        return PathBuf::from(d);
    }
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".hiapphub").join("data").join("storage")
}

fn get_db(ns: &str, custom_dir: Option<&str>) -> Result<(), HapError> {
    let dir = storage_dir(custom_dir);
    std::fs::create_dir_all(&dir)?;
    let db_path = dir.join(format!("{ns}.db"));
    let mut conns = CONNECTIONS.lock().unwrap();
    let key = db_path.to_string_lossy().to_string();
    if !conns.contains_key(&key) {
        let conn = Connection::open(&db_path)
            .map_err(|e| HapError::internal(format!("SQLite open: {e}")))?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS kv (key TEXT PRIMARY KEY, value TEXT NOT NULL, expires_at INTEGER);
             CREATE INDEX IF NOT EXISTS idx_expires ON kv(expires_at);"
        ).map_err(|e| HapError::internal(format!("init: {e}")))?;
        conns.insert(key, conn);
    }
    Ok(())
}

fn with_conn<F, R>(ns: &str, custom_dir: Option<&str>, f: F) -> Result<R, HapError>
where F: FnOnce(&Connection) -> Result<R, HapError> {
    get_db(ns, custom_dir)?;
    let dir = storage_dir(custom_dir);
    let db_path = dir.join(format!("{ns}.db"));
    let key = db_path.to_string_lossy().to_string();
    let conns = CONNECTIONS.lock().unwrap();
    let conn = conns.get(&key).unwrap();
    cleanup_expired(conn);
    f(conn)
}

fn cleanup_expired(conn: &Connection) {
    let now = now_ms();
    let _ = conn.execute("DELETE FROM kv WHERE expires_at IS NOT NULL AND expires_at <= ?1", params![now]);
}

fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64
}

// ---------- get ----------
#[derive(Deserialize)]
pub struct GetParams { pub namespace: String, pub key: String, pub default_value: Option<String>, #[serde(rename = "_storage_dir")] pub storage_dir: Option<String> }
hap_fn!(hap_storage_get, GetParams, |p| {
    with_conn(&p.namespace, p.storage_dir.as_deref(), |conn| {
        let r: Option<String> = conn.query_row(
            "SELECT value FROM kv WHERE key = ?1", params![p.key],
            |row| row.get(0)
        ).ok();
        Ok(json!(r.unwrap_or_else(|| p.default_value.clone().unwrap_or_default())))
    })
});

// ---------- set ----------
#[derive(Deserialize)]
pub struct SetParams { pub namespace: String, pub key: String, pub value: String, #[serde(rename = "_storage_dir")] pub storage_dir: Option<String> }
hap_fn!(hap_storage_set, SetParams, |p| {
    with_conn(&p.namespace, p.storage_dir.as_deref(), |conn| {
        conn.execute("INSERT OR REPLACE INTO kv (key, value, expires_at) VALUES (?1, ?2, NULL)", params![p.key, p.value])
            .map_err(|e| HapError::internal(e.to_string()))?;
        Ok(json!(true))
    })
});

// ---------- delete ----------
#[derive(Deserialize)]
pub struct DeleteParams { pub namespace: String, pub key: String, #[serde(rename = "_storage_dir")] pub storage_dir: Option<String> }
hap_fn!(hap_storage_delete, DeleteParams, |p| {
    with_conn(&p.namespace, p.storage_dir.as_deref(), |conn| {
        conn.execute("DELETE FROM kv WHERE key = ?1", params![p.key])
            .map_err(|e| HapError::internal(e.to_string()))?;
        Ok(json!(true))
    })
});

// ---------- has ----------
#[derive(Deserialize)]
pub struct HasParams { pub namespace: String, pub key: String, #[serde(rename = "_storage_dir")] pub storage_dir: Option<String> }
hap_fn!(hap_storage_has, HasParams, |p| {
    with_conn(&p.namespace, p.storage_dir.as_deref(), |conn| {
        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM kv WHERE key = ?1", params![p.key], |row| row.get(0)
        ).map_err(|e| HapError::internal(e.to_string()))?;
        Ok(json!(count > 0))
    })
});

// ---------- keys ----------
#[derive(Deserialize)]
pub struct KeysParams { pub namespace: String, pub prefix: Option<String>, #[serde(rename = "_storage_dir")] pub storage_dir: Option<String> }
hap_fn!(hap_storage_keys, KeysParams, |p| {
    with_conn(&p.namespace, p.storage_dir.as_deref(), |conn| {
        let pattern = p.prefix.as_ref().map(|pf| format!("{pf}%")).unwrap_or_else(|| "%".to_string());
        let mut stmt = conn.prepare("SELECT key FROM kv WHERE key LIKE ?1 ORDER BY key")
            .map_err(|e| HapError::internal(e.to_string()))?;
        let rows: Vec<String> = stmt.query_map(params![pattern], |row| row.get(0))
            .map_err(|e| HapError::internal(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(json!(rows))
    })
});

// ---------- get_many ----------
#[derive(Deserialize)]
pub struct GetManyParams { pub namespace: String, pub keys: Vec<String>, #[serde(rename = "_storage_dir")] pub storage_dir: Option<String> }
hap_fn!(hap_storage_get_many, GetManyParams, |p| {
    with_conn(&p.namespace, p.storage_dir.as_deref(), |conn| {
        let mut result = Map::new();
        for k in &p.keys {
            let v: Option<String> = conn.query_row("SELECT value FROM kv WHERE key = ?1", params![k], |row| row.get(0)).ok();
            result.insert(k.clone(), v.map(|s| Value::String(s)).unwrap_or(Value::Null));
        }
        Ok(Value::Object(result))
    })
});

// ---------- set_many ----------
#[derive(Deserialize)]
pub struct SetManyParams { pub namespace: String, pub entries: Map<String, Value>, #[serde(rename = "_storage_dir")] pub storage_dir: Option<String> }
hap_fn!(hap_storage_set_many, SetManyParams, |p| {
    with_conn(&p.namespace, p.storage_dir.as_deref(), |conn| {
        for (k, v) in &p.entries {
            let val = match v { Value::String(s) => s.clone(), _ => v.to_string() };
            conn.execute("INSERT OR REPLACE INTO kv (key, value, expires_at) VALUES (?1, ?2, NULL)", params![k, val])
                .map_err(|e| HapError::internal(e.to_string()))?;
        }
        Ok(json!(true))
    })
});

// ---------- clear ----------
#[derive(Deserialize)]
pub struct ClearParams { pub namespace: String, #[serde(rename = "_storage_dir")] pub storage_dir: Option<String> }
hap_fn!(hap_storage_clear, ClearParams, |p| {
    with_conn(&p.namespace, p.storage_dir.as_deref(), |conn| {
        let count: i32 = conn.query_row("SELECT COUNT(*) FROM kv", [], |row| row.get(0))
            .map_err(|e| HapError::internal(e.to_string()))?;
        conn.execute("DELETE FROM kv", []).map_err(|e| HapError::internal(e.to_string()))?;
        Ok(json!(count))
    })
});

// ---------- count ----------
#[derive(Deserialize)]
pub struct CountParams { pub namespace: String, pub prefix: Option<String>, #[serde(rename = "_storage_dir")] pub storage_dir: Option<String> }
hap_fn!(hap_storage_count, CountParams, |p| {
    with_conn(&p.namespace, p.storage_dir.as_deref(), |conn| {
        let count: i32 = match &p.prefix {
            Some(pf) => conn.query_row("SELECT COUNT(*) FROM kv WHERE key LIKE ?1", params![format!("{pf}%")], |row| row.get(0)),
            None => conn.query_row("SELECT COUNT(*) FROM kv", [], |row| row.get(0)),
        }.map_err(|e| HapError::internal(e.to_string()))?;
        Ok(json!(count))
    })
});

// ---------- namespaces ----------
#[derive(Deserialize)]
pub struct NamespacesParams { #[serde(rename = "_storage_dir")] pub storage_dir: Option<String> }
hap_fn!(hap_storage_namespaces, NamespacesParams, |p| {
    let dir = storage_dir(p.storage_dir.as_deref());
    let mut ns = vec![];
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            let path = e.path();
            if path.extension().and_then(|e| e.to_str()) == Some("db") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    ns.push(name.to_string());
                }
            }
        }
    }
    Ok(json!(ns))
});

// ---------- size ----------
#[derive(Deserialize)]
pub struct SizeParams { pub namespace: String, #[serde(rename = "_storage_dir")] pub storage_dir: Option<String> }
hap_fn!(hap_storage_size, SizeParams, |p| {
    let dir = storage_dir(p.storage_dir.as_deref());
    let db_path = dir.join(format!("{}.db", p.namespace));
    let size = std::fs::metadata(&db_path).map(|m| m.len() as i64).unwrap_or(0);
    Ok(json!(size))
});

// ---------- set_with_ttl ----------
#[derive(Deserialize)]
pub struct SetWithTtlParams { pub namespace: String, pub key: String, pub value: String, pub ttl_ms: i64, #[serde(rename = "_storage_dir")] pub storage_dir: Option<String> }
hap_fn!(hap_storage_set_with_ttl, SetWithTtlParams, |p| {
    with_conn(&p.namespace, p.storage_dir.as_deref(), |conn| {
        let expires = now_ms() + p.ttl_ms;
        conn.execute("INSERT OR REPLACE INTO kv (key, value, expires_at) VALUES (?1, ?2, ?3)",
            params![p.key, p.value, expires])
            .map_err(|e| HapError::internal(e.to_string()))?;
        Ok(json!(true))
    })
});

// ---------- export ----------
#[derive(Deserialize)]
pub struct ExportParams { pub namespace: String, pub output_path: String, #[serde(rename = "_storage_dir")] pub storage_dir: Option<String> }
hap_fn!(hap_storage_export, ExportParams, |p| {
    with_conn(&p.namespace, p.storage_dir.as_deref(), |conn| {
        let mut stmt = conn.prepare("SELECT key, value FROM kv").map_err(|e| HapError::internal(e.to_string()))?;
        let mut data = Map::new();
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        }).map_err(|e| HapError::internal(e.to_string()))?;
        for r in rows.flatten() {
            data.insert(r.0, Value::String(r.1));
        }
        let count = data.len() as i32;
        let content = serde_json::to_string_pretty(&Value::Object(data)).unwrap();
        let size = content.len() as i64;
        if let Some(parent) = Path::new(&p.output_path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&p.output_path, &content)?;
        Ok(json!({"keys": count, "size": size}))
    })
});

// ---------- import ----------
#[derive(Deserialize)]
pub struct ImportParams { pub namespace: String, pub input_path: String, pub overwrite: Option<bool>, #[serde(rename = "_storage_dir")] pub storage_dir: Option<String> }
hap_fn!(hap_storage_import, ImportParams, |p| {
    let content = std::fs::read_to_string(&p.input_path)?;
    let data: Map<String, Value> = serde_json::from_str(&content)
        .map_err(|e| HapError::invalid_param(format!("JSON: {e}")))?;
    let overwrite = p.overwrite.unwrap_or(false);
    with_conn(&p.namespace, p.storage_dir.as_deref(), |conn| {
        let mut imported = 0i32;
        let mut skipped = 0i32;
        for (k, v) in &data {
            let val = match v { Value::String(s) => s.clone(), _ => v.to_string() };
            if !overwrite {
                let exists: i32 = conn.query_row("SELECT COUNT(*) FROM kv WHERE key = ?1", params![k], |row| row.get(0))
                    .unwrap_or(0);
                if exists > 0 { skipped += 1; continue; }
            }
            conn.execute("INSERT OR REPLACE INTO kv (key, value, expires_at) VALUES (?1, ?2, NULL)", params![k, val])
                .map_err(|e| HapError::internal(e.to_string()))?;
            imported += 1;
        }
        Ok(json!({"imported": imported, "skipped": skipped}))
    })
});

// ---------- increment ----------
#[derive(Deserialize)]
pub struct IncrementParams { pub namespace: String, pub key: String, pub delta: Option<i64>, #[serde(rename = "_storage_dir")] pub storage_dir: Option<String> }
hap_fn!(hap_storage_increment, IncrementParams, |p| {
    let delta = p.delta.unwrap_or(1);
    with_conn(&p.namespace, p.storage_dir.as_deref(), |conn| {
        let current: i64 = conn.query_row("SELECT value FROM kv WHERE key = ?1", params![p.key], |row| {
            let s: String = row.get(0)?;
            Ok(s.parse::<i64>().unwrap_or(0))
        }).unwrap_or(0);
        let new_val = current + delta;
        conn.execute("INSERT OR REPLACE INTO kv (key, value, expires_at) VALUES (?1, ?2, NULL)", params![p.key, new_val.to_string()])
            .map_err(|e| HapError::internal(e.to_string()))?;
        Ok(json!(new_val))
    })
});

// ---------- delete_many ----------
#[derive(Deserialize)]
pub struct DeleteManyParams { pub namespace: String, pub keys: Vec<String>, #[serde(rename = "_storage_dir")] pub storage_dir: Option<String> }
hap_fn!(hap_storage_delete_many, DeleteManyParams, |p| {
    with_conn(&p.namespace, p.storage_dir.as_deref(), |conn| {
        let mut deleted = 0i32;
        for k in &p.keys {
            let n = conn.execute("DELETE FROM kv WHERE key = ?1", params![k])
                .map_err(|e| HapError::internal(e.to_string()))?;
            deleted += n as i32;
        }
        Ok(json!(deleted))
    })
});

// ---------- values ----------
#[derive(Deserialize)]
pub struct ValuesParams { pub namespace: String, pub prefix: Option<String>, #[serde(rename = "_storage_dir")] pub storage_dir: Option<String> }
hap_fn!(hap_storage_values, ValuesParams, |p| {
    with_conn(&p.namespace, p.storage_dir.as_deref(), |conn| {
        let pattern = p.prefix.as_ref().map(|pf| format!("{pf}%")).unwrap_or_else(|| "%".to_string());
        let mut stmt = conn.prepare("SELECT value FROM kv WHERE key LIKE ?1 ORDER BY key")
            .map_err(|e| HapError::internal(e.to_string()))?;
        let rows: Vec<String> = stmt.query_map(params![pattern], |row| row.get(0))
            .map_err(|e| HapError::internal(e.to_string()))?.filter_map(|r| r.ok()).collect();
        Ok(json!(rows))
    })
});

// ---------- entries ----------
#[derive(Deserialize)]
pub struct EntriesParams { pub namespace: String, pub prefix: Option<String>, #[serde(rename = "_storage_dir")] pub storage_dir: Option<String> }
hap_fn!(hap_storage_entries, EntriesParams, |p| {
    with_conn(&p.namespace, p.storage_dir.as_deref(), |conn| {
        let pattern = p.prefix.as_ref().map(|pf| format!("{pf}%")).unwrap_or_else(|| "%".to_string());
        let mut stmt = conn.prepare("SELECT key, value FROM kv WHERE key LIKE ?1 ORDER BY key")
            .map_err(|e| HapError::internal(e.to_string()))?;
        let rows: Vec<Value> = stmt.query_map(params![pattern], |row| {
            Ok(json!({"key": row.get::<_, String>(0)?, "value": row.get::<_, String>(1)?}))
        }).map_err(|e| HapError::internal(e.to_string()))?.filter_map(|r| r.ok()).collect();
        Ok(json!(rows))
    })
});

// ---------- get_json ----------
#[derive(Deserialize)]
pub struct GetJsonParams { pub namespace: String, pub key: String, #[serde(rename = "_storage_dir")] pub storage_dir: Option<String> }
hap_fn!(hap_storage_get_json, GetJsonParams, |p| {
    with_conn(&p.namespace, p.storage_dir.as_deref(), |conn| {
        let r: Option<String> = conn.query_row("SELECT value FROM kv WHERE key = ?1", params![p.key], |row| row.get(0)).ok();
        match r {
            Some(s) => Ok(serde_json::from_str(&s).unwrap_or(Value::Null)),
            None => Ok(Value::Null),
        }
    })
});

// ---------- set_json ----------
#[derive(Deserialize)]
pub struct SetJsonParams { pub namespace: String, pub key: String, pub value: Value, #[serde(rename = "_storage_dir")] pub storage_dir: Option<String> }
hap_fn!(hap_storage_set_json, SetJsonParams, |p| {
    let serialized = serde_json::to_string(&p.value).unwrap();
    with_conn(&p.namespace, p.storage_dir.as_deref(), |conn| {
        conn.execute("INSERT OR REPLACE INTO kv (key, value, expires_at) VALUES (?1, ?2, NULL)", params![p.key, serialized])
            .map_err(|e| HapError::internal(e.to_string()))?;
        Ok(json!(true))
    })
});

// ---------- on_change ----------
#[derive(Deserialize)]
pub struct OnChangeParams { pub namespace: String, #[allow(dead_code)] pub key_prefix: Option<String>, #[allow(dead_code)] pub callback_id: String, #[serde(rename = "_storage_dir")] #[allow(dead_code)] pub storage_dir: Option<String> }
hap_fn!(hap_storage_on_change, OnChangeParams, |p| {
    let watcher_id = format!("storage-{}-{}", p.namespace, now_ms());
    Ok(json!({"watcher_id": watcher_id}))
});

// ---------- off_change ----------
#[derive(Deserialize)]
pub struct OffChangeParams { #[allow(dead_code)] pub watcher_id: String }
hap_fn!(hap_storage_off_change, OffChangeParams, |_p| {
    Ok(json!(true))
});
