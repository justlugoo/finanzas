use chrono::{Local, Datelike, NaiveDate, Duration};
use crate::models::Period;

pub fn format_cop_simple(n: i64) -> String {
    let s = n.abs().to_string();
    let mut chars: Vec<char> = Vec::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 { chars.push('.'); }
        chars.push(c);
    }
    format!("${}", chars.into_iter().rev().collect::<String>())
}

pub fn send_notification(app: &tauri::AppHandle, title: &str, body: &str) {
    #[cfg(target_os = "linux")]
    {
        let _ = app;
        let _ = std::process::Command::new("notify-send")
            .arg("--app-name=Finanzas")
            .arg(title)
            .arg(body)
            .spawn();
    }
    #[cfg(not(target_os = "linux"))]
    {
        use tauri_plugin_notification::NotificationExt;
        if let Err(e) = app.notification().builder().title(title).body(body).show() {
            eprintln!("[finanzas] notification error: {e}");
        }
    }
}

pub fn days_in_month(year: i32, month: u32) -> u32 {
    let next = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    };
    next.unwrap()
        .signed_duration_since(NaiveDate::from_ymd_opt(year, month, 1).unwrap())
        .num_days() as u32
}

pub fn scale_monthly(monthly: i64, period: &Period) -> i64 {
    if monthly == 0 { return 0; }
    let today = Local::now().date_naive();
    let dim = days_in_month(today.year(), today.month()) as f64;
    match period {
        Period::Daily   => (monthly as f64 / dim).round() as i64,
        Period::Weekly  => (monthly as f64 * 7.0 / dim).round() as i64,
        Period::Monthly => monthly,
        Period::Yearly  => monthly * 12,
        Period::Custom { start, end } => {
            let s = NaiveDate::parse_from_str(start, "%Y-%m-%d").unwrap_or(today);
            let e = NaiveDate::parse_from_str(end, "%Y-%m-%d").unwrap_or(today);
            let days = ((e - s).num_days() + 1).max(1) as f64;
            (monthly as f64 * days / 30.0).round() as i64
        }
    }
}

pub fn period_to_dates(period: &Period) -> (String, String) {
    let today = Local::now().date_naive();
    let (start, end) = match period {
        Period::Daily => (today, today),
        Period::Weekly => {
            let days = today.weekday().num_days_from_monday() as i64;
            (today - Duration::days(days), today)
        }
        Period::Monthly => {
            let first = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
            (first, today)
        }
        Period::Yearly => {
            let first = NaiveDate::from_ymd_opt(today.year(), 1, 1).unwrap();
            (first, today)
        }
        Period::Custom { start, end } => {
            return (start.clone(), end.clone());
        }
    };
    (start.format("%Y-%m-%d").to_string(), end.format("%Y-%m-%d").to_string())
}

pub fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

pub fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if in_quotes => {
                if chars.peek() == Some(&'"') { current.push('"'); chars.next(); }
                else { in_quotes = false; }
            }
            '"' => { in_quotes = true; }
            ',' if !in_quotes => { fields.push(std::mem::take(&mut current)); }
            _ => current.push(c),
        }
    }
    fields.push(current);
    fields
}

pub fn is_valid_date(s: &str) -> bool {
    s.len() == 10 && NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok()
}
