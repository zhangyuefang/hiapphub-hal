use hap_common::{hap_fn, HapError};
use rusqlite::{Connection, OpenFlags};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};

static DB_MAP: LazyLock<Mutex<HashMap<String, Mutex<Connection>>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
static DB_COUNTER: AtomicU64 = AtomicU64::new(1);

fn db_insert(conn: Connection) -> String {
    let id = format!("db_{}", DB_COUNTER.fetch_add(1, Ordering::Relaxed));
    DB_MAP.lock().unwrap().insert(id.clone(), Mutex::new(conn));
    id
}

fn db_remove(id: &str) {
    DB_MAP.lock().unwrap().remove(id);
}

fn with_db<F>(id: &str, f: F) -> Result<Value, HapError>
where F: FnOnce(&Connection) -> Result<Value, HapError> {
    let map = DB_MAP.lock().unwrap();
    let mtx = map.get(id).ok_or_else(|| HapError::invalid_param("invalid db_id"))?;
    let conn = mtx.lock().unwrap();
    f(&conn)
}

fn db_list_ids() -> Vec<String> {
    DB_MAP.lock().unwrap().keys().cloned().collect()
}

fn val_to_rusqlite(v: &Value) -> Box<dyn rusqlite::types::ToSql> {
    match v {
        Value::Null => Box::new(rusqlite::types::Null),
        Value::Bool(b) => Box::new(*b as i32),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() { Box::new(i) }
            else if let Some(f) = n.as_f64() { Box::new(f) }
            else { Box::new(n.to_string()) }
        }
        Value::String(s) => Box::new(s.clone()),
        _ => Box::new(v.to_string()),
    }
}

fn row_to_json(row: &rusqlite::Row, col_count: usize) -> Result<Vec<Value>, rusqlite::Error> {
    let mut vals = Vec::with_capacity(col_count);
    for i in 0..col_count {
        let v: Value = match row.get_ref(i)? {
            rusqlite::types::ValueRef::Null => Value::Null,
            rusqlite::types::ValueRef::Integer(n) => json!(n),
            rusqlite::types::ValueRef::Real(f) => json!(f),
            rusqlite::types::ValueRef::Text(t) => {
                let s = std::str::from_utf8(t).unwrap_or("");
                json!(s)
            }
            rusqlite::types::ValueRef::Blob(b) => json!(base64::Engine::encode(&base64::engine::general_purpose::STANDARD, b)),
        };
        vals.push(v);
    }
    Ok(vals)
}

// ---------- open ----------
#[derive(Deserialize)]
pub struct OpenParams { pub path: String, #[allow(dead_code)] pub password: Option<String>, pub readonly: Option<bool>, pub create: Option<bool> }
hap_fn!(hap_sqlite_open, OpenParams, |p| {
    if let Some(parent) = std::path::Path::new(&p.path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut flags = OpenFlags::SQLITE_OPEN_NO_MUTEX;
    if p.readonly.unwrap_or(false) {
        flags |= OpenFlags::SQLITE_OPEN_READ_ONLY;
    } else {
        flags |= OpenFlags::SQLITE_OPEN_READ_WRITE;
        if p.create.unwrap_or(true) { flags |= OpenFlags::SQLITE_OPEN_CREATE; }
    }
    let conn = Connection::open_with_flags(&p.path, flags)
        .map_err(|e| HapError::internal(format!("SQLite open: {e}")))?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
        .map_err(|e| HapError::internal(e.to_string()))?;
    let db_id = db_insert(conn);
    Ok(json!({"db_id": db_id}))
});

// ---------- close ----------
#[derive(Deserialize)]
pub struct DbIdParam { pub db_id: String }
hap_fn!(hap_sqlite_close, DbIdParam, |p| {
    db_remove(&p.db_id);
    Ok(json!(true))
});

// ---------- execute ----------
#[derive(Deserialize)]
pub struct ExecuteParams { pub db_id: String, pub sql: String, pub params: Option<Vec<Value>> }
hap_fn!(hap_sqlite_execute, ExecuteParams, |p| {
    with_db(&p.db_id, |conn| {
        let params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = p.params.as_deref().unwrap_or(&[]).iter().map(val_to_rusqlite).collect();
        let refs: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();
        let changes = conn.execute(&p.sql, refs.as_slice())
            .map_err(|e| HapError::internal(e.to_string()))?;
        let rowid = conn.last_insert_rowid();
        Ok(json!({"changes": changes as i32, "last_insert_rowid": rowid}))
    })
});

// ---------- batch_execute ----------
#[derive(Deserialize)]
pub struct BatchExecParams { pub db_id: String, pub statements: Vec<StmtItem> }
#[derive(Deserialize)]
pub struct StmtItem { pub sql: String, pub params: Option<Vec<Value>> }
hap_fn!(hap_sqlite_batch_execute, BatchExecParams, |p| {
    with_db(&p.db_id, |conn| {
        conn.execute_batch("BEGIN").map_err(|e| HapError::internal(e.to_string()))?;
        let mut total = 0i32;
        for stmt in &p.statements {
            let pv: Vec<Box<dyn rusqlite::types::ToSql>> = stmt.params.as_deref().unwrap_or(&[]).iter().map(val_to_rusqlite).collect();
            let refs: Vec<&dyn rusqlite::types::ToSql> = pv.iter().map(|b| b.as_ref()).collect();
            let n = conn.execute(&stmt.sql, refs.as_slice()).map_err(|e| {
                let _ = conn.execute_batch("ROLLBACK");
                HapError::internal(e.to_string())
            })?;
            total += n as i32;
        }
        conn.execute_batch("COMMIT").map_err(|e| HapError::internal(e.to_string()))?;
        Ok(json!({"total_changes": total}))
    })
});

// ---------- query ----------
#[derive(Deserialize)]
pub struct QueryParams { pub db_id: String, pub sql: String, pub params: Option<Vec<Value>> }
hap_fn!(hap_sqlite_query, QueryParams, |p| {
    with_db(&p.db_id, |conn| {
        let pv: Vec<Box<dyn rusqlite::types::ToSql>> = p.params.as_deref().unwrap_or(&[]).iter().map(val_to_rusqlite).collect();
        let refs: Vec<&dyn rusqlite::types::ToSql> = pv.iter().map(|b| b.as_ref()).collect();
        let mut stmt = conn.prepare(&p.sql).map_err(|e| HapError::internal(e.to_string()))?;
        let col_count = stmt.column_count();
        let columns: Vec<String> = stmt.column_names().into_iter().map(|s| s.to_string()).collect();
        let rows: Vec<Vec<Value>> = stmt.query_map(refs.as_slice(), |row| {
            Ok(row_to_json(row, col_count).unwrap_or_default())
        }).map_err(|e| HapError::internal(e.to_string()))?.filter_map(|r| r.ok()).collect();
        Ok(json!({"columns": columns, "rows": rows}))
    })
});

// ---------- query_one ----------
hap_fn!(hap_sqlite_query_one, QueryParams, |p| {
    with_db(&p.db_id, |conn| {
        let pv: Vec<Box<dyn rusqlite::types::ToSql>> = p.params.as_deref().unwrap_or(&[]).iter().map(val_to_rusqlite).collect();
        let refs: Vec<&dyn rusqlite::types::ToSql> = pv.iter().map(|b| b.as_ref()).collect();
        let mut stmt = conn.prepare(&p.sql).map_err(|e| HapError::internal(e.to_string()))?;
        let col_count = stmt.column_count();
        let columns: Vec<String> = stmt.column_names().into_iter().map(|s| s.to_string()).collect();
        let result = stmt.query_row(refs.as_slice(), |row| {
            let vals = row_to_json(row, col_count).unwrap_or_default();
            let mut obj = Map::new();
            for (i, col) in columns.iter().enumerate() {
                obj.insert(col.clone(), vals.get(i).cloned().unwrap_or(Value::Null));
            }
            Ok(Value::Object(obj))
        });
        match result {
            Ok(v) => Ok(v),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(Value::Null),
            Err(e) => Err(HapError::internal(e.to_string())),
        }
    })
});

// ---------- query_objects ----------
hap_fn!(hap_sqlite_query_objects, QueryParams, |p| {
    with_db(&p.db_id, |conn| {
        let pv: Vec<Box<dyn rusqlite::types::ToSql>> = p.params.as_deref().unwrap_or(&[]).iter().map(val_to_rusqlite).collect();
        let refs: Vec<&dyn rusqlite::types::ToSql> = pv.iter().map(|b| b.as_ref()).collect();
        let mut stmt = conn.prepare(&p.sql).map_err(|e| HapError::internal(e.to_string()))?;
        let col_count = stmt.column_count();
        let columns: Vec<String> = stmt.column_names().into_iter().map(|s| s.to_string()).collect();
        let rows: Vec<Value> = stmt.query_map(refs.as_slice(), |row| {
            let vals = row_to_json(row, col_count).unwrap_or_default();
            let mut obj = Map::new();
            for (i, col) in columns.iter().enumerate() {
                obj.insert(col.clone(), vals.get(i).cloned().unwrap_or(Value::Null));
            }
            Ok(Value::Object(obj))
        }).map_err(|e| HapError::internal(e.to_string()))?.filter_map(|r| r.ok()).collect();
        Ok(json!(rows))
    })
});

// ---------- begin/commit/rollback ----------
#[derive(Deserialize)]
pub struct BeginParams { pub db_id: String, #[serde(rename = "type")] pub tx_type: Option<String> }
hap_fn!(hap_sqlite_begin, BeginParams, |p| {
    with_db(&p.db_id, |conn| {
        let t = p.tx_type.as_deref().unwrap_or("deferred").to_uppercase();
        conn.execute_batch(&format!("BEGIN {t}")).map_err(|e| HapError::internal(e.to_string()))?;
        Ok(json!(true))
    })
});

hap_fn!(hap_sqlite_commit, DbIdParam, |p| {
    with_db(&p.db_id, |conn| {
        conn.execute_batch("COMMIT").map_err(|e| HapError::internal(e.to_string()))?;
        Ok(json!(true))
    })
});

hap_fn!(hap_sqlite_rollback, DbIdParam, |p| {
    with_db(&p.db_id, |conn| {
        conn.execute_batch("ROLLBACK").map_err(|e| HapError::internal(e.to_string()))?;
        Ok(json!(true))
    })
});

// ---------- vacuum ----------
hap_fn!(hap_sqlite_vacuum, DbIdParam, |p| {
    with_db(&p.db_id, |conn| {
        conn.execute_batch("VACUUM").map_err(|e| HapError::internal(e.to_string()))?;
        Ok(json!(true))
    })
});

// ---------- backup ----------
#[derive(Deserialize)]
pub struct BackupParams { pub db_id: String, pub dest_path: String }
hap_fn!(hap_sqlite_backup, BackupParams, |p| {
    with_db(&p.db_id, |conn| {
        let mut dest = Connection::open(&p.dest_path).map_err(|e| HapError::internal(e.to_string()))?;
        let backup = rusqlite::backup::Backup::new(conn, &mut dest).map_err(|e| HapError::internal(e.to_string()))?;
        backup.run_to_completion(100, std::time::Duration::from_millis(50), None)
            .map_err(|e| HapError::internal(e.to_string()))?;
        let size = std::fs::metadata(&p.dest_path)?.len() as i64;
        Ok(json!({"size": size}))
    })
});

// ---------- table_list ----------
hap_fn!(hap_sqlite_table_list, DbIdParam, |p| {
    with_db(&p.db_id, |conn| {
        let mut stmt = conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
            .map_err(|e| HapError::internal(e.to_string()))?;
        let names: Vec<String> = stmt.query_map([], |row| row.get(0))
            .map_err(|e| HapError::internal(e.to_string()))?.filter_map(|r| r.ok()).collect();
        Ok(json!(names))
    })
});

// ---------- table_info ----------
#[derive(Deserialize)]
pub struct TableInfoParams { pub db_id: String, pub table_name: String }
hap_fn!(hap_sqlite_table_info, TableInfoParams, |p| {
    with_db(&p.db_id, |conn| {
        let mut stmt = conn.prepare(&format!("PRAGMA table_info(\"{}\")", p.table_name.replace('"', "\"\"")))
            .map_err(|e| HapError::internal(e.to_string()))?;
        let infos: Vec<Value> = stmt.query_map([], |row| {
            Ok(json!({
                "cid": row.get::<_, i32>(0)?,
                "name": row.get::<_, String>(1)?,
                "type": row.get::<_, String>(2)?,
                "notnull": row.get::<_, i32>(3)? != 0,
                "dflt_value": row.get::<_, Option<String>>(4)?,
                "pk": row.get::<_, i32>(5)? != 0,
            }))
        }).map_err(|e| HapError::internal(e.to_string()))?.filter_map(|r| r.ok()).collect();
        Ok(json!(infos))
    })
});

// ---------- pragma ----------
#[derive(Deserialize)]
pub struct PragmaParams { pub db_id: String, pub name: String, pub value: Option<String> }
hap_fn!(hap_sqlite_pragma, PragmaParams, |p| {
    with_db(&p.db_id, |conn| {
        let sql = match &p.value {
            Some(v) => format!("PRAGMA {} = {}", p.name, v),
            None => format!("PRAGMA {}", p.name),
        };
        let result: Option<String> = conn.query_row(&sql, [], |row| row.get(0)).ok();
        Ok(json!(result.unwrap_or_default()))
    })
});

// ---------- is_open ----------
hap_fn!(hap_sqlite_is_open, DbIdParam, |p| {
    Ok(json!(DB_MAP.lock().unwrap().contains_key(&p.db_id)))
});

// ---------- db_size ----------
hap_fn!(hap_sqlite_db_size, DbIdParam, |p| {
    with_db(&p.db_id, |conn| {
        let path: String = conn.query_row("PRAGMA database_list", [], |row| row.get(2))
            .map_err(|e| HapError::internal(e.to_string()))?;
        let size = std::fs::metadata(&path).map(|m| m.len() as i64).unwrap_or(0);
        Ok(json!(size))
    })
});

// ---------- count ----------
#[derive(Deserialize)]
pub struct CountParams { pub db_id: String, pub table_name: String, pub where_clause: Option<String>, pub params: Option<Vec<Value>> }
hap_fn!(hap_sqlite_count, CountParams, |p| {
    with_db(&p.db_id, |conn| {
        let sql = match &p.where_clause {
            Some(w) => format!("SELECT COUNT(*) FROM \"{}\" WHERE {}", p.table_name.replace('"', "\"\""), w),
            None => format!("SELECT COUNT(*) FROM \"{}\"", p.table_name.replace('"', "\"\"")),
        };
        let pv: Vec<Box<dyn rusqlite::types::ToSql>> = p.params.as_deref().unwrap_or(&[]).iter().map(val_to_rusqlite).collect();
        let refs: Vec<&dyn rusqlite::types::ToSql> = pv.iter().map(|b| b.as_ref()).collect();
        let count: i64 = conn.query_row(&sql, refs.as_slice(), |row| row.get(0))
            .map_err(|e| HapError::internal(e.to_string()))?;
        Ok(json!(count))
    })
});

// ---------- index_list ----------
hap_fn!(hap_sqlite_index_list, TableInfoParams, |p| {
    with_db(&p.db_id, |conn| {
        let mut stmt = conn.prepare(&format!("PRAGMA index_list(\"{}\")", p.table_name.replace('"', "\"\"")))
            .map_err(|e| HapError::internal(e.to_string()))?;
        let indices: Vec<Value> = stmt.query_map([], |row| {
            Ok(json!({
                "name": row.get::<_, String>(1)?,
                "unique": row.get::<_, i32>(2)? != 0,
            }))
        }).map_err(|e| HapError::internal(e.to_string()))?.filter_map(|r| r.ok()).collect();
        Ok(json!(indices))
    })
});

// ---------- export_csv ----------
#[derive(Deserialize)]
pub struct ExportCsvParams { pub db_id: String, pub table_or_query: String, pub output_path: String, pub delimiter: Option<String>, pub with_header: Option<bool> }
hap_fn!(hap_sqlite_export_csv, ExportCsvParams, |p| {
    with_db(&p.db_id, |conn| {
        let sql = if p.table_or_query.trim().to_uppercase().starts_with("SELECT") {
            p.table_or_query.clone()
        } else {
            format!("SELECT * FROM \"{}\"", p.table_or_query.replace('"', "\"\""))
        };
        let mut stmt = conn.prepare(&sql).map_err(|e| HapError::internal(e.to_string()))?;
        let col_count = stmt.column_count();
        let columns: Vec<String> = stmt.column_names().into_iter().map(|s| s.to_string()).collect();
        let delim = p.delimiter.as_deref().unwrap_or(",").as_bytes()[0];
        let mut wtr = csv::WriterBuilder::new().delimiter(delim).from_path(&p.output_path)
            .map_err(|e| HapError::internal(e.to_string()))?;
        if p.with_header.unwrap_or(true) {
            wtr.write_record(&columns).map_err(|e| HapError::internal(e.to_string()))?;
        }
        let mut row_count = 0i32;
        let mut rows = stmt.query([]).map_err(|e| HapError::internal(e.to_string()))?;
        while let Some(row) = rows.next().map_err(|e| HapError::internal(e.to_string()))? {
            let vals = row_to_json(row, col_count).unwrap_or_default();
            let strs: Vec<String> = vals.iter().map(|v| match v {
                Value::Null => "".to_string(),
                Value::String(s) => s.clone(),
                _ => v.to_string(),
            }).collect();
            wtr.write_record(&strs).map_err(|e| HapError::internal(e.to_string()))?;
            row_count += 1;
        }
        wtr.flush().map_err(|e| HapError::internal(e.to_string()))?;
        Ok(json!({"rows": row_count}))
    })
});

// ---------- import_csv ----------
#[derive(Deserialize)]
pub struct ImportCsvParams { pub db_id: String, pub table_name: String, pub csv_path: String, pub delimiter: Option<String>, pub has_header: Option<bool>, pub on_conflict: Option<String> }
hap_fn!(hap_sqlite_import_csv, ImportCsvParams, |p| {
    with_db(&p.db_id, |conn| {
        let delim = p.delimiter.as_deref().unwrap_or(",").as_bytes()[0];
        let mut rdr = csv::ReaderBuilder::new().delimiter(delim).has_headers(p.has_header.unwrap_or(true))
            .from_path(&p.csv_path).map_err(|e| HapError::internal(e.to_string()))?;
        let headers: Vec<String> = if p.has_header.unwrap_or(true) {
            rdr.headers().map_err(|e| HapError::internal(e.to_string()))?.iter().map(|s| s.to_string()).collect()
        } else {
            let mut stmt = conn.prepare(&format!("PRAGMA table_info(\"{}\")", p.table_name.replace('"', "\"\"")))
                .map_err(|e| HapError::internal(e.to_string()))?;
            let info: Vec<String> = stmt.query_map([], |row| row.get::<_, String>(1))
                .map_err(|e| HapError::internal(e.to_string()))?.filter_map(|r| r.ok()).collect();
            info
        };
        let conflict = match p.on_conflict.as_deref() {
            Some("ignore") => "OR IGNORE",
            Some("replace") => "OR REPLACE",
            _ => "",
        };
        let placeholders: Vec<String> = (0..headers.len()).map(|_| "?".to_string()).collect();
        let cols: Vec<String> = headers.iter().map(|h| format!("\"{}\"", h.replace('"', "\"\""))).collect();
        let sql = format!("INSERT {} INTO \"{}\" ({}) VALUES ({})",
            conflict, p.table_name.replace('"', "\"\""), cols.join(","), placeholders.join(","));
        let mut imported = 0i32;
        let mut skipped = 0i32;
        for result in rdr.records() {
            let record = result.map_err(|e| HapError::internal(e.to_string()))?;
            let values: Vec<String> = record.iter().map(|s| s.to_string()).collect();
            let params: Vec<&dyn rusqlite::types::ToSql> = values.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
            match conn.execute(&sql, params.as_slice()) {
                Ok(_) => imported += 1,
                Err(_) => skipped += 1,
            }
        }
        Ok(json!({"imported": imported, "skipped": skipped}))
    })
});

// ---------- wal_checkpoint ----------
#[derive(Deserialize)]
pub struct WalCheckpointParams { pub db_id: String, #[allow(dead_code)] pub mode: Option<String> }
hap_fn!(hap_sqlite_wal_checkpoint, WalCheckpointParams, |p| {
    with_db(&p.db_id, |conn| {
        let result: i32 = conn.query_row("PRAGMA wal_checkpoint(PASSIVE)", [], |row| row.get(0)).unwrap_or(0);
        Ok(json!({"pages_checkpointed": result}))
    })
});

// ---------- attach/detach ----------
#[derive(Deserialize)]
pub struct AttachParams { pub db_id: String, pub path: String, pub alias: String, #[allow(dead_code)] pub password: Option<String> }
hap_fn!(hap_sqlite_attach, AttachParams, |p| {
    with_db(&p.db_id, |conn| {
        conn.execute_batch(&format!("ATTACH DATABASE '{}' AS \"{}\"", p.path.replace('\'', "''"), p.alias.replace('"', "\"\"")))
            .map_err(|e| HapError::internal(e.to_string()))?;
        Ok(json!(true))
    })
});

#[derive(Deserialize)]
pub struct DetachParams { pub db_id: String, pub alias: String }
hap_fn!(hap_sqlite_detach, DetachParams, |p| {
    with_db(&p.db_id, |conn| {
        conn.execute_batch(&format!("DETACH DATABASE \"{}\"", p.alias.replace('"', "\"\"")))
            .map_err(|e| HapError::internal(e.to_string()))?;
        Ok(json!(true))
    })
});

// ---------- set_busy_timeout ----------
#[derive(Deserialize)]
pub struct BusyTimeoutParams { pub db_id: String, pub timeout_ms: i32 }
hap_fn!(hap_sqlite_set_busy_timeout, BusyTimeoutParams, |p| {
    with_db(&p.db_id, |conn| {
        conn.busy_timeout(std::time::Duration::from_millis(p.timeout_ms as u64))
            .map_err(|e| HapError::internal(e.to_string()))?;
        Ok(json!(true))
    })
});

// ---------- interrupt ----------
hap_fn!(hap_sqlite_interrupt, DbIdParam, |p| {
    with_db(&p.db_id, |conn| {
        conn.get_interrupt_handle().interrupt();
        Ok(json!(true))
    })
});

// ---------- change_password ----------
#[derive(Deserialize)]
pub struct ChangePasswordParams { #[allow(dead_code)] pub db_id: String, #[allow(dead_code)] pub new_password: String }
hap_fn!(hap_sqlite_change_password, ChangePasswordParams, |_p| {
    Err(HapError::internal("SQLCipher not enabled, change_password unavailable"))
});

// ---------- create_function / remove_function ----------
#[derive(Deserialize)]
pub struct CreateFuncParams { #[allow(dead_code)] pub db_id: String, #[allow(dead_code)] pub name: String, #[allow(dead_code)] pub num_args: i32, #[allow(dead_code)] pub callback_id: String }
hap_fn!(hap_sqlite_create_function, CreateFuncParams, |_p| {
    Err(HapError::internal("create_function requires Bridge callback channel"))
});

#[derive(Deserialize)]
pub struct RemoveFuncParams { #[allow(dead_code)] pub db_id: String, #[allow(dead_code)] pub name: String }
hap_fn!(hap_sqlite_remove_function, RemoveFuncParams, |_p| {
    Err(HapError::internal("remove_function requires Bridge callback channel"))
});

// ---------- list_open ----------
#[derive(Deserialize)]
pub struct EmptyParams {}
hap_fn!(hap_sqlite_list_open, EmptyParams, |_p| {
    let ids = db_list_ids();
    let mut list = vec![];
    for id in &ids {
        if let Ok(path) = with_db(id, |conn| {
            let p: String = conn.query_row("PRAGMA database_list", [], |row| row.get(2)).unwrap_or_default();
            Ok(json!(p))
        }) {
            list.push(json!({"db_id": id, "path": path, "readonly": false}));
        }
    }
    Ok(json!(list))
});
