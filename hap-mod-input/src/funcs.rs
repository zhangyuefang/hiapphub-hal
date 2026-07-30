use hap_common::{hap_fn, HapError};
use serde::Deserialize;
use serde_json::json;
use enigo::{Enigo, Keyboard, Mouse, Settings, Coordinate, Direction, Button, Key};

fn create_enigo() -> Result<Enigo, HapError> {
    Enigo::new(&Settings::default())
        .map_err(|e| HapError::internal(format!("failed to create input controller: {e}")))
}

fn parse_key(key: &str) -> Key {
    match key.to_lowercase().as_str() {
        "enter" | "return" => Key::Return,
        "tab" => Key::Tab,
        "escape" | "esc" => Key::Escape,
        "backspace" => Key::Backspace,
        "delete" | "del" => Key::Delete,
        "space" => Key::Space,
        "up" => Key::UpArrow,
        "down" => Key::DownArrow,
        "left" => Key::LeftArrow,
        "right" => Key::RightArrow,
        "home" => Key::Home,
        "end" => Key::End,
        "pageup" => Key::PageUp,
        "pagedown" => Key::PageDown,
        "shift" => Key::Shift,
        "ctrl" | "control" => Key::Control,
        "alt" | "option" => Key::Alt,
        "meta" | "cmd" | "command" | "win" | "super" => Key::Meta,
        "capslock" => Key::CapsLock,
        "f1" => Key::F1,
        "f2" => Key::F2,
        "f3" => Key::F3,
        "f4" => Key::F4,
        "f5" => Key::F5,
        "f6" => Key::F6,
        "f7" => Key::F7,
        "f8" => Key::F8,
        "f9" => Key::F9,
        "f10" => Key::F10,
        "f11" => Key::F11,
        "f12" => Key::F12,
        other => {
            if other.len() == 1 {
                Key::Unicode(other.chars().next().unwrap())
            } else {
                Key::Unicode(' ')
            }
        }
    }
}

fn parse_button(btn: &str) -> Button {
    match btn.to_lowercase().as_str() {
        "right" => Button::Right,
        "middle" => Button::Middle,
        _ => Button::Left,
    }
}

#[derive(Deserialize)]
struct KeyPressParams {
    key: String,
    modifiers: Option<Vec<String>>,
}

hap_fn!(hap_input_key_press, KeyPressParams, |params| {
    let mut enigo = create_enigo()?;

    if let Some(mods) = &params.modifiers {
        for m in mods {
            let k = parse_key(m);
            enigo.key(k, Direction::Press).map_err(|e| HapError::internal(format!("{e}")))?;
        }
    }

    let key = parse_key(&params.key);
    enigo.key(key, Direction::Click).map_err(|e| HapError::internal(format!("{e}")))?;

    if let Some(mods) = &params.modifiers {
        for m in mods.iter().rev() {
            let k = parse_key(m);
            enigo.key(k, Direction::Release).map_err(|e| HapError::internal(format!("{e}")))?;
        }
    }

    Ok(json!(true))
});

#[derive(Deserialize)]
struct KeyParams {
    key: String,
}

hap_fn!(hap_input_key_down, KeyParams, |params| {
    let mut enigo = create_enigo()?;
    let key = parse_key(&params.key);
    enigo.key(key, Direction::Press).map_err(|e| HapError::internal(format!("{e}")))?;
    Ok(json!(true))
});

hap_fn!(hap_input_key_up, KeyParams, |params| {
    let mut enigo = create_enigo()?;
    let key = parse_key(&params.key);
    enigo.key(key, Direction::Release).map_err(|e| HapError::internal(format!("{e}")))?;
    Ok(json!(true))
});

#[derive(Deserialize)]
struct TypeTextParams {
    text: String,
    delay_ms: Option<i32>,
}

hap_fn!(hap_input_type_text, TypeTextParams, |params| {
    let mut enigo = create_enigo()?;
    let delay = params.delay_ms.unwrap_or(0).max(0) as u64;

    if delay > 0 {
        for ch in params.text.chars() {
            enigo.text(&ch.to_string()).map_err(|e| HapError::internal(format!("{e}")))?;
            std::thread::sleep(std::time::Duration::from_millis(delay));
        }
    } else {
        enigo.text(&params.text).map_err(|e| HapError::internal(format!("{e}")))?;
    }

    Ok(json!(true))
});

#[derive(Deserialize)]
struct MouseMoveParams {
    x: i32,
    y: i32,
    smooth: Option<bool>,
    duration_ms: Option<i32>,
}

hap_fn!(hap_input_mouse_move, MouseMoveParams, |params| {
    let mut enigo = create_enigo()?;

    if params.smooth.unwrap_or(false) && params.duration_ms.unwrap_or(0) > 0 {
        let duration = params.duration_ms.unwrap_or(300) as u64;
        let steps = (duration / 16).max(1);
        let (loc_x, loc_y) = enigo.location().map_err(|e| HapError::internal(format!("{e}")))?;
        let dx = (params.x - loc_x) as f64 / steps as f64;
        let dy = (params.y - loc_y) as f64 / steps as f64;
        for i in 1..=steps {
            let nx = loc_x + (dx * i as f64) as i32;
            let ny = loc_y + (dy * i as f64) as i32;
            enigo.move_mouse(nx, ny, Coordinate::Abs).map_err(|e| HapError::internal(format!("{e}")))?;
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
    } else {
        enigo.move_mouse(params.x, params.y, Coordinate::Abs)
            .map_err(|e| HapError::internal(format!("{e}")))?;
    }

    Ok(json!(true))
});

#[derive(Deserialize)]
struct MouseClickParams {
    x: Option<i32>,
    y: Option<i32>,
    button: Option<String>,
    clicks: Option<i32>,
}

hap_fn!(hap_input_mouse_click, MouseClickParams, |params| {
    let mut enigo = create_enigo()?;

    if let (Some(x), Some(y)) = (params.x, params.y) {
        enigo.move_mouse(x, y, Coordinate::Abs)
            .map_err(|e| HapError::internal(format!("{e}")))?;
    }

    let btn = parse_button(&params.button.unwrap_or_else(|| "left".into()));
    let clicks = params.clicks.unwrap_or(1);

    for _ in 0..clicks {
        enigo.button(btn, Direction::Click).map_err(|e| HapError::internal(format!("{e}")))?;
    }

    Ok(json!(true))
});

#[derive(Deserialize)]
struct MouseButtonParams {
    button: Option<String>,
}

hap_fn!(hap_input_mouse_down, MouseButtonParams, |params| {
    let mut enigo = create_enigo()?;
    let btn = parse_button(&params.button.unwrap_or_else(|| "left".into()));
    enigo.button(btn, Direction::Press).map_err(|e| HapError::internal(format!("{e}")))?;
    Ok(json!(true))
});

hap_fn!(hap_input_mouse_up, MouseButtonParams, |params| {
    let mut enigo = create_enigo()?;
    let btn = parse_button(&params.button.unwrap_or_else(|| "left".into()));
    enigo.button(btn, Direction::Release).map_err(|e| HapError::internal(format!("{e}")))?;
    Ok(json!(true))
});

#[derive(Deserialize)]
struct MouseDragParams {
    from_x: i32,
    from_y: i32,
    to_x: i32,
    to_y: i32,
    duration_ms: Option<i32>,
}

hap_fn!(hap_input_mouse_drag, MouseDragParams, |params| {
    let mut enigo = create_enigo()?;
    let duration = params.duration_ms.unwrap_or(300) as u64;
    let steps = (duration / 16).max(1);

    enigo.move_mouse(params.from_x, params.from_y, Coordinate::Abs)
        .map_err(|e| HapError::internal(format!("{e}")))?;
    enigo.button(Button::Left, Direction::Press)
        .map_err(|e| HapError::internal(format!("{e}")))?;

    let dx = (params.to_x - params.from_x) as f64 / steps as f64;
    let dy = (params.to_y - params.from_y) as f64 / steps as f64;
    for i in 1..=steps {
        let nx = params.from_x + (dx * i as f64) as i32;
        let ny = params.from_y + (dy * i as f64) as i32;
        enigo.move_mouse(nx, ny, Coordinate::Abs).map_err(|e| HapError::internal(format!("{e}")))?;
        std::thread::sleep(std::time::Duration::from_millis(16));
    }

    enigo.button(Button::Left, Direction::Release)
        .map_err(|e| HapError::internal(format!("{e}")))?;
    Ok(json!(true))
});

#[derive(Deserialize)]
struct ScrollParams {
    delta_x: Option<i32>,
    delta_y: i32,
}

hap_fn!(hap_input_scroll, ScrollParams, |params| {
    let mut enigo = create_enigo()?;
    let dx = params.delta_x.unwrap_or(0);
    if dx != 0 {
        enigo.scroll(dx, enigo::Axis::Horizontal).map_err(|e| HapError::internal(format!("{e}")))?;
    }
    if params.delta_y != 0 {
        enigo.scroll(params.delta_y, enigo::Axis::Vertical).map_err(|e| HapError::internal(format!("{e}")))?;
    }
    Ok(json!(true))
});

#[derive(Deserialize)]
struct EmptyParams {}

hap_fn!(hap_input_get_mouse_position, EmptyParams, |_params| {
    let enigo = create_enigo()?;
    let (x, y) = enigo.location().map_err(|e| HapError::internal(format!("{e}")))?;
    Ok(json!({ "x": x, "y": y }))
});

#[derive(Deserialize)]
struct HotkeyParams {
    keys: Vec<String>,
}

hap_fn!(hap_input_hotkey, HotkeyParams, |params| {
    if params.keys.is_empty() {
        return Err(HapError::invalid_param("keys cannot be empty"));
    }
    let mut enigo = create_enigo()?;

    for k in &params.keys {
        let key = parse_key(k);
        enigo.key(key, Direction::Press).map_err(|e| HapError::internal(format!("{e}")))?;
    }
    for k in params.keys.iter().rev() {
        let key = parse_key(k);
        enigo.key(key, Direction::Release).map_err(|e| HapError::internal(format!("{e}")))?;
    }

    Ok(json!(true))
});
