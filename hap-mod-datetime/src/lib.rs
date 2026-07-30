use hap_common::{hap_free_string, hap_module_init, hap_fn, HapError};
use chrono::{Datelike, Local, NaiveDate, Offset, TimeZone, Timelike, Utc, Weekday};
use serde::Deserialize;
use serde_json::{json, Value};

hap_module_init!("datetime");
hap_free_string!();

fn resolve_tz(tz: Option<&str>) -> Result<chrono_tz::Tz, HapError> {
    match tz {
        Some(name) => name.parse::<chrono_tz::Tz>()
            .map_err(|_| HapError::invalid_param(format!("unknown timezone: {name}"))),
        None => Ok(chrono_tz::Tz::UTC),
    }
}

fn ms_to_dt(unix_ms: i64, tz: chrono_tz::Tz) -> chrono::DateTime<chrono_tz::Tz> {
    let secs = unix_ms / 1000;
    let nanos = ((unix_ms % 1000) * 1_000_000) as u32;
    tz.timestamp_opt(secs, nanos).single()
        .unwrap_or_else(|| tz.timestamp_opt(secs, 0).unwrap())
}

// ---------- 1. now ----------
#[derive(Deserialize)]
struct NowParams { timezone: Option<String> }
hap_fn!(hap_datetime_now, NowParams, |p| {
    let now = Utc::now();
    let tz = resolve_tz(p.timezone.as_deref())?;
    let dt = now.with_timezone(&tz);
    Ok(json!({
        "iso": dt.to_rfc3339(),
        "unix_ms": now.timestamp_millis(),
        "timezone": tz.name(),
        "offset_min": dt.offset().fix().local_minus_utc() / 60,
    }))
});

// ---------- 2. format ----------
#[derive(Deserialize)]
struct FormatParams { unix_ms: i64, pattern: String, timezone: Option<String>, #[allow(dead_code)] locale: Option<String> }
hap_fn!(hap_datetime_format, FormatParams, |p| {
    let tz = resolve_tz(p.timezone.as_deref())?;
    let dt = ms_to_dt(p.unix_ms, tz);
    Ok(json!(dt.format(&p.pattern).to_string()))
});

// ---------- 3. parse ----------
#[derive(Deserialize)]
struct ParseParams { date_string: String, pattern: Option<String>, timezone: Option<String> }
hap_fn!(hap_datetime_parse, ParseParams, |p| {
    let tz = resolve_tz(p.timezone.as_deref())?;
    if let Some(ref pat) = p.pattern {
        let naive = chrono::NaiveDateTime::parse_from_str(&p.date_string, pat)
            .map_err(|e| HapError::invalid_param(format!("parse failed: {e}")))?;
        let dt = tz.from_local_datetime(&naive).single()
            .ok_or_else(|| HapError::invalid_param("timezone conversion failed"))?;
        Ok(json!({ "unix_ms": dt.timestamp_millis(), "iso": dt.to_rfc3339() }))
    } else {
        let dt = chrono::DateTime::parse_from_rfc3339(&p.date_string)
            .or_else(|_| chrono::DateTime::parse_from_rfc2822(&p.date_string))
            .map_err(|e| HapError::invalid_param(format!("parse failed: {e}")))?;
        Ok(json!({ "unix_ms": dt.timestamp_millis(), "iso": dt.to_rfc3339() }))
    }
});

// ---------- 4. diff ----------
#[derive(Deserialize)]
struct DiffParams { unix_ms_1: i64, unix_ms_2: i64, unit: String }
hap_fn!(hap_datetime_diff, DiffParams, |p| {
    let diff_ms = (p.unix_ms_1 - p.unix_ms_2) as f64;
    let result = match p.unit.as_str() {
        "ms" => diff_ms,
        "s" => diff_ms / 1000.0,
        "m" => diff_ms / 60_000.0,
        "h" => diff_ms / 3_600_000.0,
        "d" => diff_ms / 86_400_000.0,
        "w" => diff_ms / 604_800_000.0,
        "M" => diff_ms / 2_592_000_000.0,
        "y" => diff_ms / 31_536_000_000.0,
        _ => return Err(HapError::invalid_param(format!("unknown unit: {}", p.unit))),
    };
    Ok(json!(result))
});

// ---------- 5. add ----------
#[derive(Deserialize)]
struct AddParams { unix_ms: i64, amount: i64, unit: String }
hap_fn!(hap_datetime_add, AddParams, |p| {
    let delta_ms = match p.unit.as_str() {
        "ms" => p.amount,
        "s" => p.amount * 1000,
        "m" => p.amount * 60_000,
        "h" => p.amount * 3_600_000,
        "d" => p.amount * 86_400_000,
        "w" => p.amount * 604_800_000,
        "M" => {
            let dt = ms_to_dt(p.unix_ms, chrono_tz::Tz::UTC);
            let new_month = dt.month0() as i64 + p.amount;
            let year_delta = new_month.div_euclid(12);
            let month = (new_month.rem_euclid(12) + 1) as u32;
            let year = dt.year() + year_delta as i32;
            let day = dt.day().min(days_in_month_inner(year, month));
            let new_dt = Utc.with_ymd_and_hms(year, month, day, dt.hour(), dt.minute(), dt.second())
                .single().ok_or_else(|| HapError::internal("date calculation overflow"))?;
            return Ok(json!(new_dt.timestamp_millis()));
        }
        "y" => {
            let dt = ms_to_dt(p.unix_ms, chrono_tz::Tz::UTC);
            let year = dt.year() + p.amount as i32;
            let day = dt.day().min(days_in_month_inner(year, dt.month()));
            let new_dt = Utc.with_ymd_and_hms(year, dt.month(), day, dt.hour(), dt.minute(), dt.second())
                .single().ok_or_else(|| HapError::internal("date calculation overflow"))?;
            return Ok(json!(new_dt.timestamp_millis()));
        }
        _ => return Err(HapError::invalid_param(format!("unknown unit: {}", p.unit))),
    };
    Ok(json!(p.unix_ms + delta_ms))
});

fn days_in_month_inner(year: i32, month: u32) -> u32 {
    NaiveDate::from_ymd_opt(
        if month == 12 { year + 1 } else { year },
        if month == 12 { 1 } else { month + 1 },
        1,
    ).unwrap().pred_opt().unwrap().day()
}

// ---------- 6. start_of ----------
#[derive(Deserialize)]
struct StartOfParams { unix_ms: i64, unit: String, timezone: Option<String> }
hap_fn!(hap_datetime_start_of, StartOfParams, |p| {
    let tz = resolve_tz(p.timezone.as_deref())?;
    let dt = ms_to_dt(p.unix_ms, tz);
    let result = match p.unit.as_str() {
        "day" => tz.with_ymd_and_hms(dt.year(), dt.month(), dt.day(), 0, 0, 0).unwrap(),
        "week" => {
            let weekday_offset = dt.weekday().num_days_from_monday();
            let start = dt - chrono::Duration::days(weekday_offset as i64);
            tz.with_ymd_and_hms(start.year(), start.month(), start.day(), 0, 0, 0).unwrap()
        }
        "month" => tz.with_ymd_and_hms(dt.year(), dt.month(), 1, 0, 0, 0).unwrap(),
        "year" => tz.with_ymd_and_hms(dt.year(), 1, 1, 0, 0, 0).unwrap(),
        _ => return Err(HapError::invalid_param(format!("unknown unit: {}", p.unit))),
    };
    Ok(json!(result.timestamp_millis()))
});

// ---------- 7. end_of ----------
#[derive(Deserialize)]
struct EndOfParams { unix_ms: i64, unit: String, timezone: Option<String> }
hap_fn!(hap_datetime_end_of, EndOfParams, |p| {
    let tz = resolve_tz(p.timezone.as_deref())?;
    let dt = ms_to_dt(p.unix_ms, tz);
    let result = match p.unit.as_str() {
        "day" => tz.with_ymd_and_hms(dt.year(), dt.month(), dt.day(), 23, 59, 59).unwrap(),
        "week" => {
            let days_to_sunday = 6 - dt.weekday().num_days_from_monday();
            let end = dt + chrono::Duration::days(days_to_sunday as i64);
            tz.with_ymd_and_hms(end.year(), end.month(), end.day(), 23, 59, 59).unwrap()
        }
        "month" => {
            let last_day = days_in_month_inner(dt.year(), dt.month());
            tz.with_ymd_and_hms(dt.year(), dt.month(), last_day, 23, 59, 59).unwrap()
        }
        "year" => tz.with_ymd_and_hms(dt.year(), 12, 31, 23, 59, 59).unwrap(),
        _ => return Err(HapError::invalid_param(format!("unknown unit: {}", p.unit))),
    };
    Ok(json!(result.timestamp_millis() + 999))
});

// ---------- 8. timezone_list ----------
#[derive(Deserialize)]
struct EmptyParams {}
hap_fn!(hap_datetime_timezone_list, EmptyParams, |_p| {
    let zones: Vec<&str> = chrono_tz::TZ_VARIANTS.iter().map(|tz| tz.name()).collect();
    Ok(json!(zones))
});

// ---------- 9. timezone_offset ----------
#[derive(Deserialize)]
struct TzOffsetParams { timezone: String, unix_ms: Option<i64> }
hap_fn!(hap_datetime_timezone_offset, TzOffsetParams, |p| {
    let tz = resolve_tz(Some(&p.timezone))?;
    let unix_ms = p.unix_ms.unwrap_or_else(|| Utc::now().timestamp_millis());
    let dt = ms_to_dt(unix_ms, tz);
    Ok(json!(dt.offset().fix().local_minus_utc() / 60))
});

// ---------- 10. to_unix ----------
#[derive(Deserialize)]
struct ToUnixParams { iso_string: String }
hap_fn!(hap_datetime_to_unix, ToUnixParams, |p| {
    let dt = chrono::DateTime::parse_from_rfc3339(&p.iso_string)
        .map_err(|e| HapError::invalid_param(format!("ISO parse failed: {e}")))?;
    Ok(json!(dt.timestamp_millis()))
});

// ---------- 11. from_unix ----------
#[derive(Deserialize)]
struct FromUnixParams { unix_ms: i64, timezone: Option<String> }
hap_fn!(hap_datetime_from_unix, FromUnixParams, |p| {
    let tz = resolve_tz(p.timezone.as_deref())?;
    let dt = ms_to_dt(p.unix_ms, tz);
    Ok(json!(dt.to_rfc3339()))
});

// ---------- 12. is_leap_year ----------
#[derive(Deserialize)]
struct LeapYearParams { year: i32 }
hap_fn!(hap_datetime_is_leap_year, LeapYearParams, |p| {
    let leap = (p.year % 4 == 0 && p.year % 100 != 0) || (p.year % 400 == 0);
    Ok(json!(leap))
});

// ---------- 13. days_in_month ----------
#[derive(Deserialize)]
struct DaysInMonthParams { year: i32, month: i32 }
hap_fn!(hap_datetime_days_in_month, DaysInMonthParams, |p| {
    if !(1..=12).contains(&p.month) {
        return Err(HapError::invalid_param("month must be between 1 and 12"));
    }
    Ok(json!(days_in_month_inner(p.year, p.month as u32)))
});

// ---------- 14. day_of_week ----------
#[derive(Deserialize)]
struct DayOfWeekParams { unix_ms: i64 }
hap_fn!(hap_datetime_day_of_week, DayOfWeekParams, |p| {
    let dt = ms_to_dt(p.unix_ms, chrono_tz::Tz::UTC);
    let dow = match dt.weekday() {
        Weekday::Sun => 0,
        Weekday::Mon => 1,
        Weekday::Tue => 2,
        Weekday::Wed => 3,
        Weekday::Thu => 4,
        Weekday::Fri => 5,
        Weekday::Sat => 6,
    };
    Ok(json!(dow))
});

// ---------- 15. is_between ----------
#[derive(Deserialize)]
struct IsBetweenParams { unix_ms: i64, start_ms: i64, end_ms: i64, inclusive: Option<bool> }
hap_fn!(hap_datetime_is_between, IsBetweenParams, |p| {
    let inclusive = p.inclusive.unwrap_or(true);
    let result = if inclusive {
        p.unix_ms >= p.start_ms && p.unix_ms <= p.end_ms
    } else {
        p.unix_ms > p.start_ms && p.unix_ms < p.end_ms
    };
    Ok(json!(result))
});

// ---------- 16. relative ----------
#[derive(Deserialize)]
struct RelativeParams { unix_ms: i64, locale: Option<String> }
hap_fn!(hap_datetime_relative, RelativeParams, |p| {
    let now_ms = Utc::now().timestamp_millis();
    let diff = now_ms - p.unix_ms;
    let abs_diff = diff.unsigned_abs();
    let is_zh = p.locale.as_deref().map_or_else(
        || Local::now().format("%Z").to_string().contains("CST"),
        |l| l.starts_with("zh"),
    );
    let (num, unit_zh, unit_en) = if abs_diff < 60_000 {
        (abs_diff / 1000, "秒", "second")
    } else if abs_diff < 3_600_000 {
        (abs_diff / 60_000, "分钟", "minute")
    } else if abs_diff < 86_400_000 {
        (abs_diff / 3_600_000, "小时", "hour")
    } else if abs_diff < 2_592_000_000 {
        (abs_diff / 86_400_000, "天", "day")
    } else if abs_diff < 31_536_000_000 {
        (abs_diff / 2_592_000_000, "个月", "month")
    } else {
        (abs_diff / 31_536_000_000, "年", "year")
    };
    let text = if is_zh {
        if diff > 0 { format!("{num} {unit_zh}前") } else { format!("{num} {unit_zh}后") }
    } else {
        let plural = if num != 1 { "s" } else { "" };
        if diff > 0 { format!("{num} {unit_en}{plural} ago") } else { format!("in {num} {unit_en}{plural}") }
    };
    Ok(json!(text))
});

// ---------- 17. is_weekend ----------
#[derive(Deserialize)]
struct IsWeekendParams { unix_ms: i64 }
hap_fn!(hap_datetime_is_weekend, IsWeekendParams, |p| {
    let dt = ms_to_dt(p.unix_ms, chrono_tz::Tz::UTC);
    Ok(json!(matches!(dt.weekday(), Weekday::Sat | Weekday::Sun)))
});

// ---------- 18. week_of_year ----------
#[derive(Deserialize)]
struct WeekOfYearParams { unix_ms: i64 }
hap_fn!(hap_datetime_week_of_year, WeekOfYearParams, |p| {
    let dt = ms_to_dt(p.unix_ms, chrono_tz::Tz::UTC);
    Ok(json!(dt.iso_week().week()))
});

// ---------- 19. calendar ----------
#[derive(Deserialize)]
struct CalendarParams { year: i32, month: i32, week_start: Option<i32> }
hap_fn!(hap_datetime_calendar, CalendarParams, |p| {
    if !(1..=12).contains(&p.month) {
        return Err(HapError::invalid_param("month must be between 1 and 12"));
    }
    let week_start = p.week_start.unwrap_or(1); // 1=Monday
    let days = days_in_month_inner(p.year, p.month as u32);
    let first = NaiveDate::from_ymd_opt(p.year, p.month as u32, 1)
        .ok_or_else(|| HapError::invalid_param("invalid date"))?;
    let first_dow = first.weekday().num_days_from_monday() as i32;
    let offset = (first_dow - (if week_start == 0 { 6 } else { week_start - 1 }) + 7) % 7;

    let mut weeks: Vec<Vec<Value>> = Vec::new();
    let mut week: Vec<Value> = vec![Value::Null; offset as usize];

    for day in 1..=days {
        week.push(json!(day));
        if week.len() == 7 {
            weeks.push(week);
            week = Vec::new();
        }
    }
    if !week.is_empty() {
        while week.len() < 7 { week.push(Value::Null); }
        weeks.push(week);
    }
    Ok(json!({ "weeks": weeks }))
});

// ---------- 20. is_valid ----------
#[derive(Deserialize)]
struct IsValidParams { date_string: String, pattern: Option<String> }
hap_fn!(hap_datetime_is_valid, IsValidParams, |p| {
    let valid = if let Some(ref pat) = p.pattern {
        chrono::NaiveDateTime::parse_from_str(&p.date_string, pat).is_ok()
            || chrono::NaiveDate::parse_from_str(&p.date_string, pat).is_ok()
    } else {
        chrono::DateTime::parse_from_rfc3339(&p.date_string).is_ok()
            || chrono::DateTime::parse_from_rfc2822(&p.date_string).is_ok()
            || chrono::NaiveDateTime::parse_from_str(&p.date_string, "%Y-%m-%d %H:%M:%S").is_ok()
            || chrono::NaiveDate::parse_from_str(&p.date_string, "%Y-%m-%d").is_ok()
    };
    Ok(json!(valid))
});

// ---------- hap_module_describe ----------
#[no_mangle]
pub extern "C" fn hap_module_describe() -> *const std::os::raw::c_char {
    hap_common::ffi::str_to_c(include_str!("../manifest.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::{CStr, CString};

    fn call(func: extern "C" fn(*const std::os::raw::c_char) -> *const std::os::raw::c_char, json: &str) -> Value {
        let cs = CString::new(json).unwrap();
        let result = func(cs.as_ptr());
        assert!(!result.is_null());
        let s = unsafe { CStr::from_ptr(result) }.to_str().unwrap().to_string();
        unsafe { hap_free_string(result as *mut _) };
        serde_json::from_str(&s).unwrap()
    }

    #[test]
    fn test_now() {
        let r = call(hap_datetime_now, r#"{}"#);
        assert!(r["unix_ms"].as_i64().unwrap() > 1700000000000);
        assert!(r["iso"].as_str().is_some());
    }

    #[test]
    fn test_format() {
        // 2024-01-15 12:30:45 UTC
        let ms = 1705322445000i64;
        let r = call(hap_datetime_format, &format!(r#"{{"unix_ms":{ms},"pattern":"%Y-%m-%d","timezone":"UTC"}}"#));
        assert_eq!(r.as_str().unwrap(), "2024-01-15");
    }

    #[test]
    fn test_parse_rfc3339() {
        let r = call(hap_datetime_parse, r#"{"date_string":"2024-01-15T12:00:00+00:00"}"#);
        assert!(r["unix_ms"].as_i64().is_some());
    }

    #[test]
    fn test_parse_pattern() {
        let r = call(hap_datetime_parse, r#"{"date_string":"2024-01-15 12:00:00","pattern":"%Y-%m-%d %H:%M:%S","timezone":"UTC"}"#);
        assert!(r["unix_ms"].as_i64().is_some());
    }

    #[test]
    fn test_diff() {
        let r = call(hap_datetime_diff, r#"{"unix_ms_1":86400000,"unix_ms_2":0,"unit":"d"}"#);
        assert!((r.as_f64().unwrap() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_add_days() {
        let r = call(hap_datetime_add, r#"{"unix_ms":0,"amount":1,"unit":"d"}"#);
        assert_eq!(r.as_i64().unwrap(), 86400000);
    }

    #[test]
    fn test_add_months() {
        // 2024-01-31 + 1 month = 2024-02-29 (leap year)
        let jan31 = chrono::Utc.with_ymd_and_hms(2024, 1, 31, 0, 0, 0).unwrap().timestamp_millis();
        let r = call(hap_datetime_add, &format!(r#"{{"unix_ms":{jan31},"amount":1,"unit":"M"}}"#));
        let result_ms = r.as_i64().unwrap();
        let dt = ms_to_dt(result_ms, chrono_tz::Tz::UTC);
        assert_eq!(dt.month(), 2);
        assert_eq!(dt.day(), 29);
    }

    #[test]
    fn test_start_of_day() {
        let ms = 1705322445000i64; // 2024-01-15 12:30:45 UTC
        let r = call(hap_datetime_start_of, &format!(r#"{{"unix_ms":{ms},"unit":"day","timezone":"UTC"}}"#));
        let result = r.as_i64().unwrap();
        let dt = ms_to_dt(result, chrono_tz::Tz::UTC);
        assert_eq!(dt.hour(), 0);
        assert_eq!(dt.minute(), 0);
    }

    #[test]
    fn test_end_of_month() {
        // 2024-02-15 -> end of Feb 2024 (leap year, 29 days)
        let ms = chrono::Utc.with_ymd_and_hms(2024, 2, 15, 10, 0, 0).unwrap().timestamp_millis();
        let r = call(hap_datetime_end_of, &format!(r#"{{"unix_ms":{ms},"unit":"month","timezone":"UTC"}}"#));
        let result = r.as_i64().unwrap();
        let dt = ms_to_dt(result, chrono_tz::Tz::UTC);
        assert_eq!(dt.day(), 29);
        assert_eq!(dt.hour(), 23);
    }

    #[test]
    fn test_timezone_list() {
        let r = call(hap_datetime_timezone_list, r#"{}"#);
        let arr = r.as_array().unwrap();
        assert!(arr.len() > 400);
    }

    #[test]
    fn test_timezone_offset() {
        let r = call(hap_datetime_timezone_offset, r#"{"timezone":"Asia/Shanghai"}"#);
        assert_eq!(r.as_i64().unwrap(), 480);
    }

    #[test]
    fn test_to_unix_from_unix() {
        let r = call(hap_datetime_to_unix, r#"{"iso_string":"2024-01-01T00:00:00+00:00"}"#);
        let ms = r.as_i64().unwrap();
        let r2 = call(hap_datetime_from_unix, &format!(r#"{{"unix_ms":{ms},"timezone":"UTC"}}"#));
        assert!(r2.as_str().unwrap().starts_with("2024-01-01"));
    }

    #[test]
    fn test_is_leap_year() {
        assert_eq!(call(hap_datetime_is_leap_year, r#"{"year":2024}"#), json!(true));
        assert_eq!(call(hap_datetime_is_leap_year, r#"{"year":2023}"#), json!(false));
        assert_eq!(call(hap_datetime_is_leap_year, r#"{"year":2000}"#), json!(true));
        assert_eq!(call(hap_datetime_is_leap_year, r#"{"year":1900}"#), json!(false));
    }

    #[test]
    fn test_days_in_month() {
        assert_eq!(call(hap_datetime_days_in_month, r#"{"year":2024,"month":2}"#), json!(29));
        assert_eq!(call(hap_datetime_days_in_month, r#"{"year":2023,"month":2}"#), json!(28));
        assert_eq!(call(hap_datetime_days_in_month, r#"{"year":2024,"month":1}"#), json!(31));
    }

    #[test]
    fn test_day_of_week() {
        // 2024-01-01 is Monday
        let ms = chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap().timestamp_millis();
        let r = call(hap_datetime_day_of_week, &format!(r#"{{"unix_ms":{ms}}}"#));
        assert_eq!(r.as_i64().unwrap(), 1); // Monday=1
    }

    #[test]
    fn test_is_between() {
        assert_eq!(call(hap_datetime_is_between, r#"{"unix_ms":5,"start_ms":1,"end_ms":10}"#), json!(true));
        assert_eq!(call(hap_datetime_is_between, r#"{"unix_ms":1,"start_ms":1,"end_ms":10,"inclusive":false}"#), json!(false));
        assert_eq!(call(hap_datetime_is_between, r#"{"unix_ms":1,"start_ms":1,"end_ms":10,"inclusive":true}"#), json!(true));
    }

    #[test]
    fn test_is_weekend() {
        // 2024-01-06 is Saturday
        let sat = chrono::Utc.with_ymd_and_hms(2024, 1, 6, 12, 0, 0).unwrap().timestamp_millis();
        assert_eq!(call(hap_datetime_is_weekend, &format!(r#"{{"unix_ms":{sat}}}"#)), json!(true));
        // 2024-01-08 is Monday
        let mon = chrono::Utc.with_ymd_and_hms(2024, 1, 8, 12, 0, 0).unwrap().timestamp_millis();
        assert_eq!(call(hap_datetime_is_weekend, &format!(r#"{{"unix_ms":{mon}}}"#)), json!(false));
    }

    #[test]
    fn test_week_of_year() {
        let ms = chrono::Utc.with_ymd_and_hms(2024, 1, 15, 0, 0, 0).unwrap().timestamp_millis();
        let r = call(hap_datetime_week_of_year, &format!(r#"{{"unix_ms":{ms}}}"#));
        assert_eq!(r.as_i64().unwrap(), 3);
    }

    #[test]
    fn test_calendar() {
        let r = call(hap_datetime_calendar, r#"{"year":2024,"month":2}"#);
        let weeks = r["weeks"].as_array().unwrap();
        assert!(!weeks.is_empty());
        // Feb 2024 starts on Thursday (week_start=1/Monday), so first week has 3 nulls
        let first_week = weeks[0].as_array().unwrap();
        assert_eq!(first_week[0], Value::Null); // Mon
        assert_eq!(first_week[3], json!(1)); // Thu = day 1
    }

    #[test]
    fn test_is_valid() {
        assert_eq!(call(hap_datetime_is_valid, r#"{"date_string":"2024-01-15T12:00:00+00:00"}"#), json!(true));
        assert_eq!(call(hap_datetime_is_valid, r#"{"date_string":"not-a-date"}"#), json!(false));
        assert_eq!(call(hap_datetime_is_valid, r#"{"date_string":"2024-01-15","pattern":"%Y-%m-%d"}"#), json!(true));
    }

    #[test]
    fn test_describe() {
        let ptr = hap_module_describe();
        let s = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap();
        let v: Value = serde_json::from_str(s).unwrap();
        assert_eq!(v["name"], "datetime");
        assert_eq!(v["functions"].as_array().unwrap().len(), 20);
        unsafe { hap_free_string(ptr as *mut _) };
    }
}
