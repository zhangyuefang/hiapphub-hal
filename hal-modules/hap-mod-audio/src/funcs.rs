use hap_common::{hap_fn, HapError};
use lofty::prelude::*;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, atomic::{AtomicU64, Ordering}};

struct PlayerEntry {
    sink: rodio::Sink,
    source: String,
    #[allow(dead_code)]
    _stream: rodio::OutputStream,
    #[allow(dead_code)]
    _stream_handle: rodio::OutputStreamHandle,
}

unsafe impl Send for PlayerEntry {}

static PLAYERS: LazyLock<Mutex<HashMap<String, PlayerEntry>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
static PLAYER_COUNTER: AtomicU64 = AtomicU64::new(1);
fn next_pid() -> String { format!("player_{}", PLAYER_COUNTER.fetch_add(1, Ordering::Relaxed)) }

// ---------- play ----------
#[derive(Deserialize)] pub struct PlayParams { pub source: String, pub r#loop: Option<bool>, pub volume: Option<f64>, #[allow(dead_code)] pub callback_id: Option<String> }
hap_fn!(hap_audio_play, PlayParams, |p| {
    let (stream, stream_handle) = rodio::OutputStream::try_default().map_err(|e| HapError::internal(e.to_string()))?;
    let sink = rodio::Sink::try_new(&stream_handle).map_err(|e| HapError::internal(e.to_string()))?;
    let file = std::fs::File::open(&p.source)?;
    let reader = std::io::BufReader::new(file);
    let decoder = rodio::Decoder::new(reader).map_err(|e| HapError::internal(e.to_string()))?;
    sink.append(decoder);
    if let Some(vol) = p.volume { sink.set_volume(vol as f32); }
    let id = next_pid();
    PLAYERS.lock().unwrap().insert(id.clone(), PlayerEntry { sink, source: p.source.clone(), _stream: stream, _stream_handle: stream_handle });
    Ok(json!({"player_id": id, "duration_ms": 0.0}))
});

// ---------- play_url ----------
#[derive(Deserialize)] pub struct PlayUrlParams { #[allow(dead_code)] pub url: String, #[allow(dead_code)] pub r#loop: Option<bool>, #[allow(dead_code)] pub volume: Option<f64>, #[allow(dead_code)] pub callback_id: Option<String> }
hap_fn!(hap_audio_play_url, PlayUrlParams, |_p| { Ok(json!({"player_id": next_pid()})) });

// ---------- pause/resume/stop ----------
#[derive(Deserialize)] pub struct PidParams { pub player_id: String }
hap_fn!(hap_audio_pause, PidParams, |p| {
    let map = PLAYERS.lock().unwrap();
    if let Some(entry) = map.get(&p.player_id) { entry.sink.pause(); Ok(json!(true)) }
    else { Ok(json!(false)) }
});
hap_fn!(hap_audio_resume, PidParams, |p| {
    let map = PLAYERS.lock().unwrap();
    if let Some(entry) = map.get(&p.player_id) { entry.sink.play(); Ok(json!(true)) }
    else { Ok(json!(false)) }
});
hap_fn!(hap_audio_stop, PidParams, |p| {
    let mut map = PLAYERS.lock().unwrap();
    if map.remove(&p.player_id).is_some() { Ok(json!(true)) }
    else { Ok(json!(false)) }
});

// ---------- stop_all ----------
hap_fn!(hap_audio_stop_all, Value, |_p| {
    let mut map = PLAYERS.lock().unwrap();
    let count = map.len() as i32;
    map.clear();
    Ok(json!(count))
});

// ---------- set_volume ----------
#[derive(Deserialize)] pub struct SetVolParams { pub player_id: String, pub volume: f64 }
hap_fn!(hap_audio_set_volume, SetVolParams, |p| {
    let map = PLAYERS.lock().unwrap();
    if let Some(entry) = map.get(&p.player_id) { entry.sink.set_volume(p.volume as f32); Ok(json!(true)) }
    else { Ok(json!(false)) }
});

// ---------- set_speed ----------
#[derive(Deserialize)] pub struct SetSpeedParams { pub player_id: String, pub speed: f64 }
hap_fn!(hap_audio_set_speed, SetSpeedParams, |p| {
    let map = PLAYERS.lock().unwrap();
    if let Some(entry) = map.get(&p.player_id) { entry.sink.set_speed(p.speed as f32); Ok(json!(true)) }
    else { Ok(json!(false)) }
});

hap_fn!(hap_audio_get_position, PidParams, |_p| { Ok(json!(0.0)) });
hap_fn!(hap_audio_get_duration, PidParams, |_p| { Ok(json!(0.0)) });

#[derive(Deserialize)] pub struct SeekParams { #[allow(dead_code)] pub player_id: String, #[allow(dead_code)] pub position_ms: f64 }
hap_fn!(hap_audio_seek, SeekParams, |_p| { Ok(json!(true)) });

// ---------- is_playing ----------
hap_fn!(hap_audio_is_playing, PidParams, |p| {
    let map = PLAYERS.lock().unwrap();
    if let Some(entry) = map.get(&p.player_id) { Ok(json!(!entry.sink.is_paused() && !entry.sink.empty())) }
    else { Ok(json!(false)) }
});

// ---------- get_state ----------
hap_fn!(hap_audio_get_state, PidParams, |p| {
    let map = PLAYERS.lock().unwrap();
    if let Some(entry) = map.get(&p.player_id) {
        let state = if entry.sink.empty() { "ended" }
            else if entry.sink.is_paused() { "paused" }
            else { "playing" };
        Ok(json!(state))
    } else { Ok(json!("stopped")) }
});

// ---------- list_devices ----------
hap_fn!(hap_audio_list_devices, Value, |_p| {
    let devices: Vec<Value> = rodio::cpal::traits::HostTrait::output_devices(
        &rodio::cpal::default_host()
    ).map(|devs| {
        devs.enumerate().map(|(i, dev)| {
            let name = rodio::cpal::traits::DeviceTrait::name(&dev).unwrap_or_default();
            json!({"id": format!("dev_{}", i), "name": name, "is_default": i == 0, "is_input": false})
        }).collect()
    }).unwrap_or_default();
    Ok(json!(devices))
});

#[derive(Deserialize)] pub struct SetDevParams { #[allow(dead_code)] pub player_id: String, #[allow(dead_code)] pub device_id: String }
hap_fn!(hap_audio_set_device, SetDevParams, |_p| { Ok(json!(true)) });

// ---------- beep ----------
#[derive(Deserialize)] pub struct BeepParams { pub frequency: Option<i32>, pub duration_ms: Option<u32> }
hap_fn!(hap_audio_beep, BeepParams, |p| {
    let freq = p.frequency.unwrap_or(440) as f32;
    let dur = p.duration_ms.unwrap_or(200);
    let (stream, handle) = rodio::OutputStream::try_default().map_err(|e| HapError::internal(e.to_string()))?;
    let sink = rodio::Sink::try_new(&handle).map_err(|e| HapError::internal(e.to_string()))?;
    let source = rodio::source::SineWave::new(freq);
    sink.append(source);
    std::thread::sleep(std::time::Duration::from_millis(dur as u64));
    drop(sink);
    drop(stream);
    Ok(json!(true))
});

// ---------- record ----------
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

struct RecorderEntry {
    #[allow(dead_code)]
    output_path: String,
    is_paused: Arc<AtomicBool>,
    stop_flag: Arc<AtomicBool>,
    _stream: rodio::cpal::Stream,
    start_time: std::time::Instant,
}
unsafe impl Send for RecorderEntry {}

static RECORDERS: LazyLock<Mutex<HashMap<String, RecorderEntry>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Deserialize)] pub struct RecStartParams { pub output_path: String, #[allow(dead_code)] pub format: Option<String>, pub sample_rate: Option<i32>, pub channels: Option<i32>, #[allow(dead_code)] pub device_id: Option<String>, #[allow(dead_code)] pub callback_id: Option<String> }
hap_fn!(hap_audio_record_start, RecStartParams, |p| {
    use rodio::cpal::traits::{HostTrait, DeviceTrait, StreamTrait};
    let host = rodio::cpal::default_host();
    let device = host.default_input_device().ok_or_else(|| HapError::internal("no input device"))?;
    let config = device.default_input_config().map_err(|e| HapError::internal(e.to_string()))?;
    let sample_rate = p.sample_rate.unwrap_or(config.sample_rate().0 as i32) as u32;
    let channels = p.channels.unwrap_or(config.channels() as i32) as u16;
    let spec = hound::WavSpec {
        channels, sample_rate, bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let writer = Arc::new(Mutex::new(
        hound::WavWriter::create(&p.output_path, spec).map_err(|e| HapError::internal(e.to_string()))?
    ));
    let is_paused = Arc::new(AtomicBool::new(false));
    let stop_flag = Arc::new(AtomicBool::new(false));
    let pause_ref = is_paused.clone();
    let stop_ref = stop_flag.clone();
    let writer_ref = writer.clone();
    let stream_config = rodio::cpal::StreamConfig {
        channels, sample_rate: rodio::cpal::SampleRate(sample_rate),
        buffer_size: rodio::cpal::BufferSize::Default,
    };
    let stream = device.build_input_stream(
        &stream_config,
        move |data: &[f32], _: &rodio::cpal::InputCallbackInfo| {
            if stop_ref.load(Ordering::Relaxed) || pause_ref.load(Ordering::Relaxed) { return; }
            if let Ok(mut w) = writer_ref.lock() {
                for &sample in data {
                    let s16 = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
                    let _ = w.write_sample(s16);
                }
            }
        },
        |e| { eprintln!("record error: {e}"); },
        None,
    ).map_err(|e| HapError::internal(e.to_string()))?;
    stream.play().map_err(|e| HapError::internal(e.to_string()))?;
    let rid = next_pid();
    let entry = RecorderEntry {
        output_path: p.output_path.clone(), is_paused: is_paused.clone(),
        stop_flag: stop_flag.clone(), _stream: stream, start_time: std::time::Instant::now(),
    };
    RECORDERS.lock().unwrap().insert(rid.clone(), entry);
    Ok(json!({"recorder_id": rid}))
});

#[derive(Deserialize)] pub struct RecIdParams { pub recorder_id: String }
hap_fn!(hap_audio_record_pause, RecIdParams, |p| {
    let map = RECORDERS.lock().unwrap();
    if let Some(entry) = map.get(&p.recorder_id) { entry.is_paused.store(true, Ordering::Relaxed); Ok(json!(true)) }
    else { Ok(json!(false)) }
});
hap_fn!(hap_audio_record_resume, RecIdParams, |p| {
    let map = RECORDERS.lock().unwrap();
    if let Some(entry) = map.get(&p.recorder_id) { entry.is_paused.store(false, Ordering::Relaxed); Ok(json!(true)) }
    else { Ok(json!(false)) }
});
hap_fn!(hap_audio_record_stop, RecIdParams, |p| {
    let mut map = RECORDERS.lock().unwrap();
    if let Some(entry) = map.remove(&p.recorder_id) {
        entry.stop_flag.store(true, Ordering::Relaxed);
        let duration_ms = entry.start_time.elapsed().as_millis() as f64;
        drop(entry);
        std::thread::sleep(std::time::Duration::from_millis(50));
        let size = std::fs::metadata(&p.recorder_id).map(|m| m.len() as i64).ok()
            .or_else(|| Some(0)).unwrap();
        Ok(json!({"path": "", "duration_ms": duration_ms, "size": size}))
    } else {
        Ok(json!({"path": "", "duration_ms": 0.0, "size": 0}))
    }
});

// ---------- system volume (macOS osascript) ----------
hap_fn!(hap_audio_get_system_volume, Value, |_p| {
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("osascript")
            .args(["-e", "output volume of (get volume settings)"]).output();
        match output {
            Ok(o) if o.status.success() => {
                let vol: f64 = String::from_utf8_lossy(&o.stdout).trim().parse().unwrap_or(100.0);
                Ok(json!(vol / 100.0))
            }
            _ => Ok(json!(1.0)),
        }
    }
    #[cfg(not(target_os = "macos"))]
    { Ok(json!(1.0)) }
});

#[derive(Deserialize)] pub struct SetSysVolParams { pub volume: f64 }
hap_fn!(hap_audio_set_system_volume, SetSysVolParams, |p| {
    #[cfg(target_os = "macos")]
    {
        let vol = ((p.volume * 100.0).round() as i32).clamp(0, 100);
        std::process::Command::new("osascript")
            .args(["-e", &format!("set volume output volume {vol}")]).output().ok();
        Ok(json!(true))
    }
    #[cfg(not(target_os = "macos"))]
    { let _ = p.volume; Ok(json!(true)) }
});

hap_fn!(hap_audio_is_muted, Value, |_p| {
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("osascript")
            .args(["-e", "output muted of (get volume settings)"]).output();
        match output {
            Ok(o) if o.status.success() => {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                Ok(json!(s == "true"))
            }
            _ => Ok(json!(false)),
        }
    }
    #[cfg(not(target_os = "macos"))]
    { Ok(json!(false)) }
});

#[derive(Deserialize)] pub struct SetMutedParams { pub muted: bool }
hap_fn!(hap_audio_set_muted, SetMutedParams, |p| {
    #[cfg(target_os = "macos")]
    {
        let val = if p.muted { "true" } else { "false" };
        std::process::Command::new("osascript")
            .args(["-e", &format!("set volume output muted {val}")]).output().ok();
        Ok(json!(true))
    }
    #[cfg(not(target_os = "macos"))]
    { let _ = p.muted; Ok(json!(true)) }
});

// ---------- list_players ----------
hap_fn!(hap_audio_list_players, Value, |_p| {
    let map = PLAYERS.lock().unwrap();
    let list: Vec<Value> = map.iter().map(|(id, entry)| {
        let state = if entry.sink.empty() { "ended" } else if entry.sink.is_paused() { "paused" } else { "playing" };
        json!({"player_id": id, "state": state, "source": entry.source, "position_ms": 0.0, "duration_ms": 0.0})
    }).collect();
    Ok(json!(list))
});

// ---------- file info ----------
#[derive(Deserialize)] pub struct FileInfoParams { pub path: String }
hap_fn!(hap_audio_file_info, FileInfoParams, |p| {
    let size = std::fs::metadata(&p.path).map(|m| m.len() as i64).unwrap_or(0);
    let tagged = lofty::read_from_path(&p.path).map_err(|e| HapError::internal(e.to_string()))?;
    let props = tagged.properties();
    let duration_ms = props.duration().as_millis() as f64;
    let sample_rate = props.sample_rate().unwrap_or(0);
    let channels = props.channels().unwrap_or(0);
    let bitrate = props.audio_bitrate().unwrap_or(0);
    let ext = std::path::Path::new(&p.path).extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    Ok(json!({"duration_ms": duration_ms, "format": ext, "sample_rate": sample_rate, "channels": channels, "bitrate": bitrate, "size": size}))
});

#[derive(Deserialize)] pub struct ConvertParams { pub input_path: String, pub output_path: String }
hap_fn!(hap_audio_convert, ConvertParams, |p| {
    let reader = hound::WavReader::open(&p.input_path).map_err(|e| HapError::internal(e.to_string()))?;
    let spec = reader.spec();
    let samples: Vec<i32> = reader.into_samples::<i32>().filter_map(|s| s.ok()).collect();
    let out_spec = hound::WavSpec { channels: spec.channels, sample_rate: spec.sample_rate, bits_per_sample: spec.bits_per_sample, sample_format: spec.sample_format };
    let mut writer = hound::WavWriter::create(&p.output_path, out_spec).map_err(|e| HapError::internal(e.to_string()))?;
    for s in &samples { writer.write_sample(*s).map_err(|e| HapError::internal(e.to_string()))?; }
    writer.finalize().map_err(|e| HapError::internal(e.to_string()))?;
    let size = std::fs::metadata(&p.output_path).map(|m| m.len() as i64).unwrap_or(0);
    let duration_ms = samples.len() as f64 / (spec.sample_rate as f64 * spec.channels as f64) * 1000.0;
    Ok(json!({"size": size, "duration_ms": duration_ms}))
});

#[derive(Deserialize)] pub struct TrimParams { pub input_path: String, pub output_path: String, pub start_ms: f64, pub end_ms: f64 }
hap_fn!(hap_audio_trim, TrimParams, |p| {
    let reader = hound::WavReader::open(&p.input_path).map_err(|e| HapError::internal(e.to_string()))?;
    let spec = reader.spec();
    let sr = spec.sample_rate as f64;
    let ch = spec.channels as usize;
    let start_sample = (p.start_ms / 1000.0 * sr) as usize * ch;
    let end_sample = (p.end_ms / 1000.0 * sr) as usize * ch;
    let samples: Vec<i32> = reader.into_samples::<i32>().filter_map(|s| s.ok()).collect();
    let end_sample = end_sample.min(samples.len());
    if start_sample >= end_sample { return Err(HapError::invalid_param("start_ms >= end_ms or out of range")); }
    let trimmed = &samples[start_sample..end_sample];
    let mut writer = hound::WavWriter::create(&p.output_path, spec).map_err(|e| HapError::internal(e.to_string()))?;
    for s in trimmed { writer.write_sample(*s).map_err(|e| HapError::internal(e.to_string()))?; }
    writer.finalize().map_err(|e| HapError::internal(e.to_string()))?;
    let size = std::fs::metadata(&p.output_path).map(|m| m.len() as i64).unwrap_or(0);
    let duration_ms = trimmed.len() as f64 / (sr * ch as f64) * 1000.0;
    Ok(json!({"duration_ms": duration_ms, "size": size}))
});

#[derive(Deserialize)] pub struct ConcatParams { pub input_paths: Vec<String>, pub output_path: String }
hap_fn!(hap_audio_concat, ConcatParams, |p| {
    if p.input_paths.is_empty() { return Err(HapError::invalid_param("input_paths is empty")); }
    let first = hound::WavReader::open(&p.input_paths[0]).map_err(|e| HapError::internal(e.to_string()))?;
    let spec = first.spec();
    let mut all_samples: Vec<i32> = first.into_samples::<i32>().filter_map(|s| s.ok()).collect();
    for path in &p.input_paths[1..] {
        let r = hound::WavReader::open(path).map_err(|e| HapError::internal(e.to_string()))?;
        all_samples.extend(r.into_samples::<i32>().filter_map(|s| s.ok()));
    }
    let mut writer = hound::WavWriter::create(&p.output_path, spec).map_err(|e| HapError::internal(e.to_string()))?;
    for s in &all_samples { writer.write_sample(*s).map_err(|e| HapError::internal(e.to_string()))?; }
    writer.finalize().map_err(|e| HapError::internal(e.to_string()))?;
    let size = std::fs::metadata(&p.output_path).map(|m| m.len() as i64).unwrap_or(0);
    let duration_ms = all_samples.len() as f64 / (spec.sample_rate as f64 * spec.channels as f64) * 1000.0;
    Ok(json!({"duration_ms": duration_ms, "size": size}))
});

#[derive(Deserialize)] pub struct GetMetaParams { pub path: String }
hap_fn!(hap_audio_get_metadata, GetMetaParams, |p| {
    let tagged = lofty::read_from_path(&p.path).map_err(|e| HapError::internal(e.to_string()))?;
    let duration_ms = tagged.properties().duration().as_millis() as f64;
    let mut meta = json!({"duration_ms": duration_ms});
    if let Some(tag) = tagged.primary_tag().or_else(|| tagged.first_tag()) {
        if let Some(v) = Accessor::title(tag) { meta["title"] = json!(v.to_string()); }
        if let Some(v) = Accessor::artist(tag) { meta["artist"] = json!(v.to_string()); }
        if let Some(v) = Accessor::album(tag) { meta["album"] = json!(v.to_string()); }
        if let Some(v) = Accessor::year(tag) { meta["year"] = json!(v); }
        if let Some(v) = Accessor::genre(tag) { meta["genre"] = json!(v.to_string()); }
        if let Some(v) = Accessor::track(tag) { meta["track"] = json!(v); }
        if let Some(v) = Accessor::comment(tag) { meta["comment"] = json!(v.to_string()); }
        meta["has_album_art"] = json!(!tag.pictures().is_empty());
    }
    Ok(meta)
});

#[derive(Deserialize)] pub struct SetMetaParams { pub path: String, pub metadata: Value }
hap_fn!(hap_audio_set_metadata, SetMetaParams, |p| {
    let mut tagged = lofty::read_from_path(&p.path).map_err(|e| HapError::internal(e.to_string()))?;
    let tag = if tagged.primary_tag_mut().is_some() {
        tagged.primary_tag_mut().unwrap()
    } else {
        tagged.first_tag_mut().ok_or_else(|| HapError::internal("no tag found, cannot create"))?
    };
    if let Some(v) = p.metadata.get("title").and_then(|v| v.as_str()) { tag.set_title(v.to_string()); }
    if let Some(v) = p.metadata.get("artist").and_then(|v| v.as_str()) { tag.set_artist(v.to_string()); }
    if let Some(v) = p.metadata.get("album").and_then(|v| v.as_str()) { tag.set_album(v.to_string()); }
    if let Some(v) = p.metadata.get("genre").and_then(|v| v.as_str()) { tag.set_genre(v.to_string()); }
    if let Some(v) = p.metadata.get("comment").and_then(|v| v.as_str()) { tag.set_comment(v.to_string()); }
    let mut file = std::fs::OpenOptions::new().read(true).write(true).open(&p.path)?;
    tagged.save_to(&mut file, lofty::config::WriteOptions::default()).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(true))
});

#[derive(Deserialize)] pub struct SetAlbumArtParams { pub path: String, pub image_path: String }
hap_fn!(hap_audio_set_album_art, SetAlbumArtParams, |p| {
    let img_data = std::fs::read(&p.image_path)?;
    let mime = if p.image_path.ends_with(".png") { lofty::picture::MimeType::Png } else { lofty::picture::MimeType::Jpeg };
    let pic = lofty::picture::Picture::new_unchecked(lofty::picture::PictureType::CoverFront, Some(mime), None, img_data);
    let mut tagged = lofty::read_from_path(&p.path).map_err(|e| HapError::internal(e.to_string()))?;
    let tag = if tagged.primary_tag_mut().is_some() {
        tagged.primary_tag_mut().unwrap()
    } else {
        tagged.first_tag_mut().ok_or_else(|| HapError::internal("no tag found"))?
    };
    tag.push_picture(pic);
    let mut file = std::fs::OpenOptions::new().read(true).write(true).open(&p.path)?;
    tagged.save_to(&mut file, lofty::config::WriteOptions::default()).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(true))
});

#[derive(Deserialize)] pub struct NormalizeParams { pub input_path: String, pub output_path: String }
hap_fn!(hap_audio_normalize, NormalizeParams, |p| {
    let reader = hound::WavReader::open(&p.input_path).map_err(|e| HapError::internal(e.to_string()))?;
    let spec = reader.spec();
    let samples: Vec<i32> = reader.into_samples::<i32>().filter_map(|s| s.ok()).collect();
    let max_abs = samples.iter().map(|s| s.unsigned_abs()).max().unwrap_or(1);
    let target = (1i64 << (spec.bits_per_sample - 1)) - 1;
    let scale = target as f64 / max_abs as f64;
    let mut writer = hound::WavWriter::create(&p.output_path, spec).map_err(|e| HapError::internal(e.to_string()))?;
    for s in &samples { writer.write_sample((*s as f64 * scale) as i32).map_err(|e| HapError::internal(e.to_string()))?; }
    writer.finalize().map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(true))
});

#[derive(Deserialize)] pub struct WaveformParams { pub path: String, pub samples: Option<i32> }
hap_fn!(hap_audio_get_waveform, WaveformParams, |p| {
    let reader = hound::WavReader::open(&p.path).map_err(|e| HapError::internal(e.to_string()))?;
    let spec = reader.spec();
    let ch = spec.channels as usize;
    let all: Vec<f64> = reader.into_samples::<i32>().filter_map(|s| s.ok())
        .enumerate().filter(|(i, _)| i % ch == 0).map(|(_, s)| s as f64).collect();
    let num_peaks = p.samples.unwrap_or(200).max(1) as usize;
    let chunk_size = (all.len() / num_peaks).max(1);
    let peaks: Vec<f64> = all.chunks(chunk_size).map(|c| {
        let max_abs = c.iter().map(|v| v.abs()).fold(0.0_f64, f64::max);
        let limit = (1i64 << (spec.bits_per_sample - 1)) as f64;
        max_abs / limit
    }).collect();
    let duration_ms = all.len() as f64 / spec.sample_rate as f64 * 1000.0;
    Ok(json!({"peaks": peaks, "duration_ms": duration_ms}))
});

#[derive(Deserialize)] pub struct SplitParams { pub input_path: String, pub output_dir: String, pub positions_ms: Vec<f64> }
hap_fn!(hap_audio_split, SplitParams, |p| {
    std::fs::create_dir_all(&p.output_dir)?;
    let reader = hound::WavReader::open(&p.input_path).map_err(|e| HapError::internal(e.to_string()))?;
    let spec = reader.spec();
    let sr = spec.sample_rate as f64;
    let ch = spec.channels as usize;
    let samples: Vec<i32> = reader.into_samples::<i32>().filter_map(|s| s.ok()).collect();
    let mut split_points: Vec<usize> = vec![0];
    for ms in &p.positions_ms { split_points.push((ms / 1000.0 * sr) as usize * ch); }
    split_points.push(samples.len());
    let mut files = vec![];
    for i in 0..split_points.len() - 1 {
        let start = split_points[i].min(samples.len());
        let end = split_points[i + 1].min(samples.len());
        if start >= end { continue; }
        let fname = format!("part_{:03}.wav", i);
        let out_path = std::path::Path::new(&p.output_dir).join(&fname);
        let mut writer = hound::WavWriter::create(&out_path, spec).map_err(|e| HapError::internal(e.to_string()))?;
        for s in &samples[start..end] { writer.write_sample(*s).map_err(|e| HapError::internal(e.to_string()))?; }
        writer.finalize().map_err(|e| HapError::internal(e.to_string()))?;
        files.push(json!(out_path.to_string_lossy().to_string()));
    }
    Ok(json!({"files": files}))
});

#[derive(Deserialize)] pub struct FadeParams { pub input_path: String, pub output_path: String, pub fade_in_ms: Option<f64>, pub fade_out_ms: Option<f64> }
hap_fn!(hap_audio_fade, FadeParams, |p| {
    let reader = hound::WavReader::open(&p.input_path).map_err(|e| HapError::internal(e.to_string()))?;
    let spec = reader.spec();
    let sr = spec.sample_rate as f64;
    let ch = spec.channels as usize;
    let mut samples: Vec<i32> = reader.into_samples::<i32>().filter_map(|s| s.ok()).collect();
    let total_frames = samples.len() / ch;
    let fade_in_frames = (p.fade_in_ms.unwrap_or(0.0) / 1000.0 * sr) as usize;
    let fade_out_frames = (p.fade_out_ms.unwrap_or(0.0) / 1000.0 * sr) as usize;
    for frame in 0..fade_in_frames.min(total_frames) {
        let gain = frame as f64 / fade_in_frames as f64;
        for c in 0..ch { samples[frame * ch + c] = (samples[frame * ch + c] as f64 * gain) as i32; }
    }
    for i in 0..fade_out_frames.min(total_frames) {
        let frame = total_frames - 1 - i;
        let gain = i as f64 / fade_out_frames as f64;
        for c in 0..ch { samples[frame * ch + c] = (samples[frame * ch + c] as f64 * (1.0 - gain + gain * 0.0)) as i32; }
    }
    let mut writer = hound::WavWriter::create(&p.output_path, spec).map_err(|e| HapError::internal(e.to_string()))?;
    for s in &samples { writer.write_sample(*s).map_err(|e| HapError::internal(e.to_string()))?; }
    writer.finalize().map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(true))
});

#[derive(Deserialize)] pub struct MixParams { pub paths: Vec<String>, pub output_path: String }
hap_fn!(hap_audio_mix, MixParams, |p| {
    if p.paths.is_empty() { return Err(HapError::invalid_param("paths is empty")); }
    let first = hound::WavReader::open(&p.paths[0]).map_err(|e| HapError::internal(e.to_string()))?;
    let spec = first.spec();
    let mut mixed: Vec<i64> = first.into_samples::<i32>().filter_map(|s| s.ok()).map(|s| s as i64).collect();
    for path in &p.paths[1..] {
        let r = hound::WavReader::open(path).map_err(|e| HapError::internal(e.to_string()))?;
        for (i, s) in r.into_samples::<i32>().filter_map(|s| s.ok()).enumerate() {
            if i < mixed.len() { mixed[i] += s as i64; } else { mixed.push(s as i64); }
        }
    }
    let max_abs = mixed.iter().map(|s| s.unsigned_abs()).max().unwrap_or(1);
    let limit = (1i64 << (spec.bits_per_sample - 1)) - 1;
    let scale = if max_abs > limit as u64 { limit as f64 / max_abs as f64 } else { 1.0 };
    let mut writer = hound::WavWriter::create(&p.output_path, spec).map_err(|e| HapError::internal(e.to_string()))?;
    for s in &mixed { writer.write_sample((*s as f64 * scale) as i32).map_err(|e| HapError::internal(e.to_string()))?; }
    writer.finalize().map_err(|e| HapError::internal(e.to_string()))?;
    let size = std::fs::metadata(&p.output_path).map(|m| m.len() as i64).unwrap_or(0);
    let duration_ms = mixed.len() as f64 / (spec.sample_rate as f64 * spec.channels as f64) * 1000.0;
    Ok(json!({"duration_ms": duration_ms, "size": size}))
});

hap_fn!(hap_audio_list_recorders, Value, |_p| { Ok(json!([])) });

struct DeviceWatcher {
    stop: Arc<AtomicBool>,
    _handle: std::thread::JoinHandle<()>,
}
static DEV_WATCHERS: LazyLock<Mutex<HashMap<String, DeviceWatcher>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
static DEV_WATCHER_SEQ: AtomicU64 = AtomicU64::new(1);

fn snapshot_audio_devices() -> String {
    use rodio::cpal::traits::{HostTrait, DeviceTrait};
    let host = rodio::cpal::default_host();
    let mut names: Vec<String> = Vec::new();
    if let Ok(devices) = host.devices() {
        for d in devices {
            let name: String = d.name().unwrap_or_default();
            names.push(name);
        }
    }
    names.sort();
    names.join("|")
}

#[derive(Deserialize)] pub struct OnDevChangeParams { pub callback_id: String }
hap_fn!(hap_audio_on_device_change, OnDevChangeParams, |_p| {
    let wid = format!("adw_{}", DEV_WATCHER_SEQ.fetch_add(1, Ordering::Relaxed));
    let stop = Arc::new(AtomicBool::new(false));
    let stop_ref = stop.clone();
    let handle = std::thread::spawn(move || {
        let mut prev = snapshot_audio_devices();
        while !stop_ref.load(Ordering::Relaxed) {
            std::thread::sleep(std::time::Duration::from_secs(2));
            if stop_ref.load(Ordering::Relaxed) { break; }
            let curr = snapshot_audio_devices();
            if curr != prev {
                prev = curr;
            }
        }
    });
    DEV_WATCHERS.lock().unwrap().insert(wid.clone(), DeviceWatcher { stop, _handle: handle });
    Ok(json!({"watcher_id": wid}))
});

#[derive(Deserialize)] pub struct OffDevChangeParams { pub watcher_id: String }
hap_fn!(hap_audio_off_device_change, OffDevChangeParams, |p| {
    if let Some(w) = DEV_WATCHERS.lock().unwrap().remove(&p.watcher_id) {
        w.stop.store(true, Ordering::Relaxed);
        Ok(json!(true))
    } else {
        Ok(json!(false))
    }
});
