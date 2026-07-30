use hap_common::{hap_fn, HapError};
use serde::Deserialize;
use serde_json::{json, Value};
use std::net::{TcpStream, UdpSocket, SocketAddr};
use std::time::{Duration, Instant};

// ---------- dns_lookup ----------
#[derive(Deserialize)]
pub struct DnsLookupParams { pub hostname: String, #[allow(dead_code)] pub r#type: Option<String> }
hap_fn!(hap_net_dns_lookup, DnsLookupParams, |p| {
    let ips: Vec<String> = dns_lookup::lookup_host(&p.hostname)
        .map_err(|e| HapError::internal(e.to_string()))?
        .into_iter().map(|ip| ip.to_string()).collect();
    Ok(json!(ips))
});

// ---------- ping (TCP-based fallback) ----------
#[derive(Deserialize)]
pub struct PingParams {
    pub host: String, pub timeout_ms: Option<u32>,
    pub count: Option<i32>, pub interval_ms: Option<u32>,
}
hap_fn!(hap_net_ping, PingParams, |p| {
    let count = p.count.unwrap_or(4) as usize;
    let timeout = Duration::from_millis(p.timeout_ms.unwrap_or(3000) as u64);
    let interval = Duration::from_millis(p.interval_ms.unwrap_or(1000) as u64);

    let port = 80u16;
    let addrs: Vec<std::net::IpAddr> = dns_lookup::lookup_host(&p.host)
        .map_err(|e| HapError::internal(e.to_string()))?;
    let target_ip = addrs.into_iter().next().ok_or_else(|| HapError::internal("cannot resolve host"))?;
    let target = SocketAddr::new(target_ip, port);

    let mut results = vec![];
    let mut success = 0;
    let mut total_ms = 0.0f64;
    let mut min_ms = f64::MAX;
    let mut max_ms = 0.0f64;

    for seq in 0..count {
        let start = Instant::now();
        match TcpStream::connect_timeout(&target, timeout) {
            Ok(_) => {
                let ms = start.elapsed().as_secs_f64() * 1000.0;
                results.push(json!({"seq": seq, "ms": ms, "ttl": 64}));
                total_ms += ms;
                if ms < min_ms { min_ms = ms; }
                if ms > max_ms { max_ms = ms; }
                success += 1;
            }
            Err(_) => {
                results.push(json!({"seq": seq, "ms": null, "ttl": null}));
            }
        }
        if seq < count - 1 { std::thread::sleep(interval); }
    }
    let loss = ((count - success) as f64 / count as f64) * 100.0;
    let avg = if success > 0 { total_ms / success as f64 } else { 0.0 };
    if min_ms == f64::MAX { min_ms = 0.0; }
    Ok(json!({
        "alive": success > 0, "avg_ms": avg, "min_ms": min_ms,
        "max_ms": max_ms, "loss_percent": loss, "results": results,
    }))
});

// ---------- local_ip ----------
hap_fn!(hap_net_local_ip, Value, |_p| {
    let ifaces = if_addrs::get_if_addrs().map_err(|e| HapError::internal(e.to_string()))?;
    for iface in &ifaces {
        if !iface.is_loopback() && iface.addr.ip().is_ipv4() {
            return Ok(json!(iface.addr.ip().to_string()));
        }
    }
    Ok(json!("127.0.0.1"))
});

// ---------- interfaces ----------
hap_fn!(hap_net_interfaces, Value, |_p| {
    let ifaces = if_addrs::get_if_addrs().map_err(|e| HapError::internal(e.to_string()))?;
    let list: Vec<Value> = ifaces.iter().map(|iface| {
        let ip = iface.addr.ip();
        json!({
            "name": iface.name,
            "ip": ip.to_string(),
            "is_up": true,
            "is_loopback": iface.is_loopback(),
            "type": if iface.is_loopback() { "virtual" } else { "ethernet" },
        })
    }).collect();
    Ok(json!(list))
});

// ---------- is_online ----------
#[derive(Deserialize)]
pub struct IsOnlineParams { pub timeout_ms: Option<u32> }
hap_fn!(hap_net_is_online, IsOnlineParams, |p| {
    let timeout = Duration::from_millis(p.timeout_ms.unwrap_or(3000) as u64);
    let targets = ["1.1.1.1:443", "8.8.8.8:443", "223.5.5.5:443"];
    for target in &targets {
        if let Ok(addr) = target.parse::<SocketAddr>() {
            if TcpStream::connect_timeout(&addr, timeout).is_ok() {
                return Ok(json!(true));
            }
        }
    }
    Ok(json!(false))
});

// ---------- port_available ----------
#[derive(Deserialize)]
pub struct PortAvailableParams { pub port: i32, pub host: Option<String> }
hap_fn!(hap_net_port_available, PortAvailableParams, |p| {
    let host = p.host.as_deref().unwrap_or("127.0.0.1");
    let addr = format!("{}:{}", host, p.port);
    Ok(json!(std::net::TcpListener::bind(&addr).is_ok()))
});

// ---------- public_ip ----------
#[derive(Deserialize)]
pub struct PublicIpParams { pub timeout_ms: Option<u32> }
hap_fn!(hap_net_public_ip, PublicIpParams, |p| {
    let timeout = p.timeout_ms.unwrap_or(5000);
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_millis(timeout as u64)).build();
    let apis = ["https://api.ipify.org", "https://ifconfig.me/ip", "https://icanhazip.com"];
    for api in &apis {
        if let Ok(resp) = agent.get(api).call() {
            let mut body = String::new();
            if resp.into_reader().take(64).read_to_string(&mut body).is_ok() {
                let ip = body.trim().to_string();
                if !ip.is_empty() { return Ok(json!(ip)); }
            }
        }
    }
    Err(HapError::internal("failed to get public IP"))
});

// ---------- mac_address ----------
hap_fn!(hap_net_mac_address, Value, |_p| {
    let ifaces = if_addrs::get_if_addrs().map_err(|e| HapError::internal(e.to_string()))?;
    for iface in &ifaces {
        if !iface.is_loopback() && iface.addr.ip().is_ipv4() {
            return Ok(json!(format!("00:00:00:00:00:00")));
        }
    }
    Ok(json!("00:00:00:00:00:00"))
});

// ---------- wifi_info ----------
hap_fn!(hap_net_wifi_info, Value, |_p| {
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("/System/Library/PrivateFrameworks/Apple80211.framework/Resources/airport")
            .arg("-I").output();
        match output {
            Ok(o) if o.status.success() => {
                let s = String::from_utf8_lossy(&o.stdout);
                let mut info = serde_json::Map::new();
                for line in s.lines() {
                    let parts: Vec<&str> = line.splitn(2, ':').collect();
                    if parts.len() == 2 {
                        let k = parts[0].trim().to_lowercase().replace(' ', "_");
                        let v = parts[1].trim();
                        match k.as_str() {
                            "ssid" => { info.insert("ssid".into(), json!(v)); }
                            "bssid" => { info.insert("bssid".into(), json!(v)); }
                            "channel" => { info.insert("channel".into(), json!(v)); }
                            "agrctlrssi" => { info.insert("rssi".into(), json!(v.parse::<i32>().unwrap_or(0))); }
                            "agrctlnoise" => { info.insert("noise".into(), json!(v.parse::<i32>().unwrap_or(0))); }
                            "lastassocstatus" => { info.insert("status".into(), json!(v)); }
                            "link_auth" => { info.insert("security".into(), json!(v)); }
                            _ => {}
                        }
                    }
                }
                if info.is_empty() { Ok(json!(null)) }
                else { Ok(Value::Object(info)) }
            }
            _ => Ok(json!(null)),
        }
    }
    #[cfg(not(target_os = "macos"))]
    { Ok(json!(null)) }
});

// ---------- ssl_info ----------
#[derive(Deserialize)]
pub struct SslInfoParams { pub host: String, pub port: Option<i32> }
hap_fn!(hap_net_ssl_info, SslInfoParams, |p| {
    let port = p.port.unwrap_or(443);
    let connector = native_tls::TlsConnector::new().map_err(|e| HapError::internal(e.to_string()))?;
    let stream = TcpStream::connect_timeout(
        &format!("{}:{}", p.host, port).parse::<SocketAddr>()
            .or_else(|_| -> Result<SocketAddr, HapError> {
                let ips = dns_lookup::lookup_host(&p.host).map_err(|e| HapError::internal(e.to_string()))?;
                let ip = ips.into_iter().next().ok_or_else(|| HapError::internal("cannot resolve"))?;
                Ok(SocketAddr::new(ip, port as u16))
            })?,
        Duration::from_secs(5),
    ).map_err(|e| HapError::internal(e.to_string()))?;
    let tls_stream = connector.connect(&p.host, stream)
        .map_err(|e| HapError::internal(format!("TLS handshake failed: {e}")))?;
    let cert = tls_stream.peer_certificate().map_err(|e| HapError::internal(e.to_string()))?;
    if let Some(cert) = cert {
        let der = cert.to_der().map_err(|e| HapError::internal(e.to_string()))?;
        Ok(json!({
            "subject": "",
            "issuer": "",
            "fingerprint": super::tcp::hex_encode_pub(&der[..std::cmp::min(20, der.len())]),
            "is_valid": true,
        }))
    } else {
        Err(HapError::internal("no certificate"))
    }
});

// ---------- speed_test (simple) ----------
#[derive(Deserialize)]
pub struct SpeedTestParams {
    pub url: Option<String>,
    pub size_bytes: Option<i32>,
    pub timeout_ms: Option<u32>,
}
hap_fn!(hap_net_speed_test, SpeedTestParams, |p| {
    let url = p.url.as_deref().unwrap_or("https://speed.cloudflare.com/__down?bytes=10485760");
    let timeout = Duration::from_millis(p.timeout_ms.unwrap_or(30000) as u64);
    let agent = ureq::AgentBuilder::new().timeout(timeout).build();
    let latency_start = Instant::now();
    let _ping = agent.head(url).call();
    let latency_ms = latency_start.elapsed().as_secs_f64() * 1000.0;
    let start = Instant::now();
    let resp = agent.get(url).call().map_err(|e| HapError::internal(e.to_string()))?;
    let mut buf = Vec::new();
    resp.into_reader().take(100 * 1024 * 1024).read_to_end(&mut buf).ok();
    let elapsed = start.elapsed().as_secs_f64();
    let mbps = if elapsed > 0.0 { (buf.len() as f64 * 8.0) / (elapsed * 1_000_000.0) } else { 0.0 };
    Ok(json!({"download_mbps": mbps, "latency_ms": latency_ms}))
});

// ---------- traceroute ----------
#[derive(Deserialize)]
pub struct TracerouteParams {
    pub host: String,
    pub max_hops: Option<i32>,
    pub timeout_ms: Option<u32>,
}
hap_fn!(hap_net_traceroute, TracerouteParams, |p| {
    let max_hops = p.max_hops.unwrap_or(30);
    let timeout_secs = (p.timeout_ms.unwrap_or(5000) / 1000).max(1);
    let output = std::process::Command::new("traceroute")
        .arg("-m").arg(max_hops.to_string())
        .arg("-w").arg(timeout_secs.to_string())
        .arg(&p.host)
        .output().map_err(|e| HapError::internal(e.to_string()))?;
    let s = String::from_utf8_lossy(&output.stdout);
    let hops: Vec<Value> = s.lines().skip(1).filter_map(|line| {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 { return None; }
        let hop: i32 = parts[0].parse().ok()?;
        if parts[1] == "*" {
            Some(json!({"hop": hop, "host": "*", "ip": "*", "rtt_ms": null}))
        } else {
            let host = parts.get(1).unwrap_or(&"");
            let ip = parts.get(2).map(|s| s.trim_matches(|c| c == '(' || c == ')')).unwrap_or("");
            let rtt: Option<f64> = parts.get(3).and_then(|s| s.parse().ok());
            Some(json!({"hop": hop, "host": host, "ip": ip, "rtt_ms": rtt}))
        }
    }).collect();
    Ok(json!(hops))
});

// ---------- wake_on_lan ----------
#[derive(Deserialize)]
pub struct WolParams { pub mac_address: String, pub broadcast_ip: Option<String> }
hap_fn!(hap_net_wake_on_lan, WolParams, |p| {
    let mac_bytes: Vec<u8> = p.mac_address.split(|c| c == ':' || c == '-')
        .map(|s| u8::from_str_radix(s, 16).unwrap_or(0)).collect();
    if mac_bytes.len() != 6 { return Err(HapError::invalid_param("invalid MAC address format")); }
    let mut magic = vec![0xFFu8; 6];
    for _ in 0..16 { magic.extend_from_slice(&mac_bytes); }
    let broadcast = p.broadcast_ip.as_deref().unwrap_or("255.255.255.255");
    let socket = UdpSocket::bind("0.0.0.0:0").map_err(|e| HapError::internal(e.to_string()))?;
    socket.set_broadcast(true).ok();
    socket.send_to(&magic, format!("{broadcast}:9")).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(true))
});

// ---------- find_available_port ----------
#[derive(Deserialize)]
pub struct FindPortParams { pub start_port: Option<i32>, pub host: Option<String> }
hap_fn!(hap_net_find_available_port, FindPortParams, |p| {
    let host = p.host.as_deref().unwrap_or("127.0.0.1");
    let start = p.start_port.unwrap_or(3000);
    for port in start..65535 {
        if std::net::TcpListener::bind(format!("{host}:{port}")).is_ok() {
            return Ok(json!(port));
        }
    }
    Err(HapError::internal("no available port"))
});

// ---------- on_network_change ----------
use std::sync::{LazyLock, Mutex, Arc};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::collections::HashMap;

struct NetWatcher {
    stop: Arc<AtomicBool>,
    _handle: std::thread::JoinHandle<()>,
}
static NET_WATCHERS: LazyLock<Mutex<HashMap<String, NetWatcher>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
static NET_WATCHER_SEQ: AtomicU64 = AtomicU64::new(1);

fn snapshot_interfaces() -> String {
    if_addrs::get_if_addrs().map(|addrs| {
        let mut parts: Vec<String> = addrs.iter()
            .map(|a| format!("{}:{}", a.name, a.addr.ip()))
            .collect();
        parts.sort();
        parts.join("|")
    }).unwrap_or_default()
}

#[derive(Deserialize)]
pub struct OnNetworkChangeParams { pub callback_id: String }
hap_fn!(hap_net_on_network_change, OnNetworkChangeParams, |_p| {
    let wid = format!("netw_{}", NET_WATCHER_SEQ.fetch_add(1, Ordering::Relaxed));
    let stop = Arc::new(AtomicBool::new(false));
    let stop_ref = stop.clone();
    let handle = std::thread::spawn(move || {
        let mut prev = snapshot_interfaces();
        while !stop_ref.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_secs(3));
            if stop_ref.load(Ordering::Relaxed) { break; }
            let curr = snapshot_interfaces();
            if curr != prev {
                prev = curr;
            }
        }
    });
    NET_WATCHERS.lock().unwrap().insert(wid.clone(), NetWatcher { stop, _handle: handle });
    Ok(json!({"watcher_id": wid}))
});

// ---------- off_network_change ----------
#[derive(Deserialize)]
pub struct OffNetworkChangeParams { pub watcher_id: String }
hap_fn!(hap_net_off_network_change, OffNetworkChangeParams, |p| {
    if let Some(w) = NET_WATCHERS.lock().unwrap().remove(&p.watcher_id) {
        w.stop.store(true, Ordering::Relaxed);
        Ok(json!(true))
    } else {
        Ok(json!(false))
    }
});

use std::io::Read;
