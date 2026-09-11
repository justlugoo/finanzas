use chrono::{Local, Datelike, NaiveDate, Duration};
use crate::models::{Period, PeriodRange, PeriodV2};

/// Conversiones de unidades para el esquema v2 — sección 2, principio 1 de
/// schema-v2.md: "la conversión a galones y kilómetros ocurre en un único
/// módulo de presentación". Todo lo demás usa metros/mililitros/COP enteros.
pub const ML_PER_GALLON: f64 = 3785.411784;

pub fn km_per_gallon_to_m_per_l(km_per_gallon: f64) -> i64 {
    (km_per_gallon * 1_000_000.0 / ML_PER_GALLON).round() as i64
}

pub fn m_per_l_to_km_per_gallon(efficiency_m_per_l: i64) -> f64 {
    efficiency_m_per_l as f64 * ML_PER_GALLON / 1_000_000.0
}

pub fn liters_to_ml(l: f64) -> i64 {
    (l * 1000.0).round() as i64
}

pub fn gallons_to_ml(g: f64) -> i64 {
    (g * ML_PER_GALLON).round() as i64
}

pub fn ml_to_liters(ml: i64) -> f64 {
    ml as f64 / 1000.0
}

pub fn gallons_cost_to_ml(total_cop: i64, price_cop_per_gallon: i64) -> i64 {
    (total_cop as f64 / price_cop_per_gallon as f64 * ML_PER_GALLON).round() as i64
}

pub fn km_to_m(km: f64) -> i64 {
    (km * 1000.0).round() as i64
}

pub fn m_to_km(m: i64) -> f64 {
    m as f64 / 1000.0
}

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

/// `Period` nuevo (sección 8 de schema-v2.md). Reemplaza `period_to_dates`
/// + `scale_monthly`: siempre resuelve a un rango cerrado, y si el período
/// pedido todavía no terminó, recorta `end` a hoy y marca `partial = true`
/// — nunca compara un mes completo contra "lo que va del mes" sin decirlo.
pub fn resolve_period_range(period: &PeriodV2) -> PeriodRange {
    let today = Local::now().date_naive();
    let (start, end) = match period {
        PeriodV2::Month { year, month } => (
            NaiveDate::from_ymd_opt(*year, *month, 1).unwrap(),
            NaiveDate::from_ymd_opt(*year, *month, days_in_month(*year, *month)).unwrap(),
        ),
        PeriodV2::Year { year } => (
            NaiveDate::from_ymd_opt(*year, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(*year, 12, 31).unwrap(),
        ),
        PeriodV2::Custom { start, end } => (
            NaiveDate::parse_from_str(start, "%Y-%m-%d").unwrap_or(today),
            NaiveDate::parse_from_str(end, "%Y-%m-%d").unwrap_or(today),
        ),
    };
    let (end, partial) = if end > today { (today, true) } else { (end, false) };
    PeriodRange {
        start: start.format("%Y-%m-%d").to_string(),
        end: end.format("%Y-%m-%d").to_string(),
        partial,
    }
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

#[cfg(test)]
mod period_v2_tests {
    use super::*;

    #[test]
    fn mes_totalmente_pasado_no_es_parcial() {
        let today = Local::now().date_naive();
        // Un mes de hace un año siempre está completamente en el pasado.
        let year = today.year() - 1;
        let range = resolve_period_range(&PeriodV2::Month { year, month: 6 });
        assert_eq!(range.start, format!("{year}-06-01"));
        assert_eq!(range.end, format!("{year}-06-30"));
        assert!(!range.partial);
    }

    #[test]
    fn mes_actual_se_recorta_a_hoy_y_queda_parcial() {
        let today = Local::now().date_naive();
        let range = resolve_period_range(&PeriodV2::Month { year: today.year(), month: today.month() });
        let last_day = NaiveDate::from_ymd_opt(today.year(), today.month(), days_in_month(today.year(), today.month())).unwrap();
        if last_day > today {
            assert_eq!(range.end, today.format("%Y-%m-%d").to_string());
            assert!(range.partial);
        } else {
            assert!(!range.partial);
        }
    }

    #[test]
    fn custom_con_fin_futuro_se_recorta_y_queda_parcial() {
        let today = Local::now().date_naive();
        let future = today + Duration::days(30);
        let range = resolve_period_range(&PeriodV2::Custom {
            start: today.format("%Y-%m-%d").to_string(),
            end: future.format("%Y-%m-%d").to_string(),
        });
        assert_eq!(range.end, today.format("%Y-%m-%d").to_string());
        assert!(range.partial);
    }

    #[test]
    fn año_completamente_pasado_no_es_parcial() {
        let today = Local::now().date_naive();
        let year = today.year() - 1;
        let range = resolve_period_range(&PeriodV2::Year { year });
        assert_eq!(range.start, format!("{year}-01-01"));
        assert_eq!(range.end, format!("{year}-12-31"));
        assert!(!range.partial);
    }
}
