use hap_common::hap_fn;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, atomic::{AtomicU64, Ordering}};

static LOCKS: LazyLock<Mutex<HashMap<String, String>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
static LOCK_COUNTER: AtomicU64 = AtomicU64::new(1);

// ---------- battery_status ----------
hap_fn!(hap_power_battery_status, Value, |_p| {
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("pmset").arg("-g").arg("batt").output();
        if let Ok(out) = output {
            let s = String::from_utf8_lossy(&out.stdout);
            let is_charging = s.contains("AC Power") || s.contains("charging");
            let level: f64 = s.split('%').next()
                .and_then(|part| part.rsplit(|c: char| !c.is_ascii_digit()).next())
                .and_then(|n| n.parse().ok())
                .unwrap_or(100.0);
            return Ok(json!({
                "has_battery": s.contains("Battery"),
                "level": level,
                "is_charging": is_charging,
                "is_ac": s.contains("AC Power"),
                "time_remaining_min": -1,
                "health": "unknown"
            }));
        }
    }
    Ok(json!({
        "has_battery": false, "level": 100.0, "is_charging": false,
        "is_ac": true, "time_remaining_min": -1, "health": "unknown"
    }))
});

// ---------- is_on_battery ----------
hap_fn!(hap_power_is_on_battery, Value, |_p| {
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("pmset").arg("-g").arg("batt").output();
        if let Ok(out) = output {
            let s = String::from_utf8_lossy(&out.stdout);
            return Ok(json!(!s.contains("AC Power")));
        }
    }
    Ok(json!(false))
});

// ---------- prevent_sleep ----------
#[derive(Deserialize)] pub struct PreventSleepParams { pub reason: String, #[allow(dead_code)] pub r#type: Option<String> }
hap_fn!(hap_power_prevent_sleep, PreventSleepParams, |p| {
    let id = format!("lock_{}", LOCK_COUNTER.fetch_add(1, Ordering::Relaxed));
    LOCKS.lock().unwrap().insert(id.clone(), p.reason);
    Ok(json!({"lock_id": id}))
});

// ---------- allow_sleep ----------
#[derive(Deserialize)] pub struct AllowSleepParams { pub lock_id: String }
hap_fn!(hap_power_allow_sleep, AllowSleepParams, |p| {
    let removed = LOCKS.lock().unwrap().remove(&p.lock_id).is_some();
    Ok(json!(removed))
});

// ---------- screen_off ----------
hap_fn!(hap_power_screen_off, Value, |_p| {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("pmset").arg("displaysleepnow").output();
    }
    Ok(json!(true))
});

// ---------- idle_time ----------
hap_fn!(hap_power_idle_time, Value, |_p| {
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("ioreg").args(&["-c", "IOHIDSystem"]).output();
        if let Ok(out) = output {
            let s = String::from_utf8_lossy(&out.stdout);
            if let Some(pos) = s.find("HIDIdleTime") {
                let after = &s[pos..];
                if let Some(num) = after.split('=').nth(1) {
                    if let Ok(ns) = num.trim().split_whitespace().next().unwrap_or("0").parse::<i64>() {
                        return Ok(json!(ns / 1_000_000));
                    }
                }
            }
        }
    }
    Ok(json!(0i64))
});

// ---------- on_power_change ----------
use std::sync::atomic::{AtomicBool, Ordering as AtomOrd};
use std::sync::Arc;

struct PowerWatcher {
    stop_flag: Arc<AtomicBool>,
    _handle: std::thread::JoinHandle<()>,
}

static POWER_WATCHERS: LazyLock<Mutex<std::collections::HashMap<String, PowerWatcher>>> = LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));
static PW_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

#[derive(Deserialize)] pub struct CallbackParams { pub callback_id: String }
hap_fn!(hap_power_on_power_change, CallbackParams, |_p| {
    let wid = format!("pw_{}", PW_COUNTER.fetch_add(1, AtomOrd::Relaxed));
    let stop = Arc::new(AtomicBool::new(false));
    let stop_ref = stop.clone();
    let handle = std::thread::spawn(move || {
        let mut prev_pct: f64 = -1.0;
        let mut prev_charging = false;
        while !stop_ref.load(AtomOrd::Relaxed) {
            std::thread::sleep(std::time::Duration::from_secs(5));
            #[cfg(target_os = "macos")]
            {
                let output = std::process::Command::new("pmset").arg("-g").arg("batt").output();
                if let Ok(o) = output {
                    let s = String::from_utf8_lossy(&o.stdout);
                    let pct = s.split('%').next()
                        .and_then(|seg| seg.split_whitespace().last())
                        .and_then(|num| num.parse::<f64>().ok()).unwrap_or(-1.0);
                    let charging = s.contains("charging") && !s.contains("discharging");
                    if prev_pct >= 0.0 && (pct != prev_pct || charging != prev_charging) {
                        // power state changed
                    }
                    prev_pct = pct;
                    prev_charging = charging;
                }
            }
        }
    });
    POWER_WATCHERS.lock().unwrap().insert(wid.clone(), PowerWatcher { stop_flag: stop, _handle: handle });
    Ok(json!({"watcher_id": wid}))
});

#[derive(Deserialize)] pub struct WatcherParams { pub watcher_id: String }
hap_fn!(hap_power_off_power_change, WatcherParams, |p| {
    if let Some(w) = POWER_WATCHERS.lock().unwrap().remove(&p.watcher_id) {
        w.stop_flag.store(true, AtomOrd::Relaxed);
        Ok(json!(true))
    } else {
        Ok(json!(false))
    }
});

// ---------- list_locks ----------
hap_fn!(hap_power_list_locks, Value, |_p| {
    let map = LOCKS.lock().unwrap();
    let list: Vec<Value> = map.iter().map(|(id, reason)| json!({"lock_id": id, "reason": reason, "type": "system"})).collect();
    Ok(json!(list))
});
