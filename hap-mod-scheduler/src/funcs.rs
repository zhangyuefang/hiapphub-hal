use hap_common::{hap_fn, HapError};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
#[allow(dead_code)]
struct ScheduledTask {
    id: String,
    name: String,
    task_type: String,
    callback_id: String,
    paused: bool,
    cron_expr: Option<String>,
    interval_ms: Option<u64>,
}

static TASKS: OnceLock<Mutex<HashMap<String, ScheduledTask>>> = OnceLock::new();

fn tasks() -> &'static Mutex<HashMap<String, ScheduledTask>> {
    TASKS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn gen_id() -> String {
    format!("task_{}", NEXT_ID.fetch_add(1, Ordering::Relaxed))
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct CreateCronParams {
    name: String,
    cron_expression: String,
    callback_id: String,
}

hap_fn!(hap_scheduler_create_cron, CreateCronParams, |params| {
    use cron::Schedule;
    use std::str::FromStr;

    let _schedule = Schedule::from_str(&params.cron_expression)
        .map_err(|e| HapError::invalid_param(format!("invalid cron expression: {e}")))?;

    let id = gen_id();
    let task = ScheduledTask {
        id: id.clone(),
        name: params.name.clone(),
        task_type: "cron".to_string(),
        callback_id: params.callback_id.clone(),
        paused: false,
        cron_expr: Some(params.cron_expression.clone()),
        interval_ms: None,
    };
    tasks().lock().unwrap().insert(id.clone(), task);

    Ok(json!({ "task_id": id, "name": params.name, "type": "cron", "cron": params.cron_expression }))
});

#[derive(Deserialize)]
#[allow(dead_code)]
struct CreateIntervalParams {
    name: String,
    interval_ms: i32,
    callback_id: String,
    repeat_count: Option<i32>,
}

hap_fn!(hap_scheduler_create_interval, CreateIntervalParams, |params| {
    if params.interval_ms <= 0 {
        return Err(HapError::invalid_param("interval_ms must be positive"));
    }
    let id = gen_id();
    let task = ScheduledTask {
        id: id.clone(),
        name: params.name.clone(),
        task_type: "interval".to_string(),
        callback_id: params.callback_id.clone(),
        paused: false,
        cron_expr: None,
        interval_ms: Some(params.interval_ms as u64),
    };
    tasks().lock().unwrap().insert(id.clone(), task);

    Ok(json!({ "task_id": id, "name": params.name, "type": "interval", "interval_ms": params.interval_ms }))
});

#[derive(Deserialize)]
#[allow(dead_code)]
struct CreateTimeoutParams {
    name: String,
    delay_ms: i32,
    callback_id: String,
}

hap_fn!(hap_scheduler_create_timeout, CreateTimeoutParams, |params| {
    if params.delay_ms <= 0 {
        return Err(HapError::invalid_param("delay_ms must be positive"));
    }
    let id = gen_id();
    let task = ScheduledTask {
        id: id.clone(),
        name: params.name.clone(),
        task_type: "timeout".to_string(),
        callback_id: params.callback_id.clone(),
        paused: false,
        cron_expr: None,
        interval_ms: None,
    };
    tasks().lock().unwrap().insert(id.clone(), task);

    Ok(json!({ "task_id": id, "name": params.name, "type": "timeout", "delay_ms": params.delay_ms }))
});

#[derive(Deserialize)]
#[allow(dead_code)]
struct TaskIdParams {
    task_id: String,
}

hap_fn!(hap_scheduler_cancel, TaskIdParams, |params| {
    let mut map = tasks().lock().unwrap();
    if map.remove(&params.task_id).is_some() {
        Ok(json!(true))
    } else {
        Err(HapError::invalid_param("task_id not found"))
    }
});

hap_fn!(hap_scheduler_pause, TaskIdParams, |params| {
    let mut map = tasks().lock().unwrap();
    if let Some(task) = map.get_mut(&params.task_id) {
        task.paused = true;
        Ok(json!(true))
    } else {
        Err(HapError::invalid_param("task_id not found"))
    }
});

hap_fn!(hap_scheduler_resume, TaskIdParams, |params| {
    let mut map = tasks().lock().unwrap();
    if let Some(task) = map.get_mut(&params.task_id) {
        task.paused = false;
        Ok(json!(true))
    } else {
        Err(HapError::invalid_param("task_id not found"))
    }
});

#[derive(Deserialize)]
#[allow(dead_code)]
struct EmptyParams {}

hap_fn!(hap_scheduler_list, EmptyParams, |_params| {
    let map = tasks().lock().unwrap();
    let list: Vec<Value> = map.values().map(|t| {
        json!({
            "task_id": t.id,
            "name": t.name,
            "type": t.task_type,
            "paused": t.paused,
        })
    }).collect();
    Ok(json!(list))
});

hap_fn!(hap_scheduler_get_next_run, TaskIdParams, |params| {
    let map = tasks().lock().unwrap();
    let task = map.get(&params.task_id)
        .ok_or_else(|| HapError::invalid_param("task_id not found"))?;

    if let Some(ref expr) = task.cron_expr {
        use cron::Schedule;
        use std::str::FromStr;
        if let Ok(schedule) = Schedule::from_str(expr) {
            if let Some(next) = schedule.upcoming(chrono::Utc).next() {
                return Ok(json!({ "next_run": next.to_rfc3339() }));
            }
        }
    }
    Ok(json!({ "next_run": null }))
});
