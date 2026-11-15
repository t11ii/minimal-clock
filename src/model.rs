//! Platform-independent part: settings file, city table, time formatting and
//! day-offset calculation. Unit-tested on any OS (`cargo test`).

use chrono::{DateTime, NaiveDate, Utc};
use chrono_tz::Tz;

pub const MAX_ROWS: usize = 3;

/// Built-in cities: (label, IANA time zone, Japanese name for the settings file).
pub const CITIES: &[(&str, &str, &str)] = &[
    ("STO", "Europe/Stockholm", "ストックホルム"),
    ("LON", "Europe/London", "ロンドン"),
    ("BOS", "America/New_York", "ボストン"),
    ("SIN", "Asia/Singapore", "シンガポール"),
    ("NYC", "America/New_York", "ニューヨーク"),
    ("LAX", "America/Los_Angeles", "ロサンゼルス"),
    ("TYO", "Asia/Tokyo", "東京"),
    ("UTC", "Etc/UTC", "協定世界時"),
];

pub const DEFAULT_CONFIG: &str = "\
; minimal-clock 設定ファイル（保存するとすぐ反映されます）
;
; cities: 表示する都市。上から順に最大3つ、カンマ区切り。
;   使える都市: STO（ストックホルム） LON（ロンドン） BOS（ボストン） SIN（シンガポール）
;               NYC（ニューヨーク） LAX（ロサンゼルス） TYO（東京） UTC（協定世界時）
;   一覧にない都市は「ラベル=IANAタイムゾーン名」で指定できます。例: PAR=Europe/Paris
cities = STO, LON, BOS

; right_margin: タスクバー右端から時計の右端までの距離（100%表示時のpx）。空欄なら自動。
;   時計をドラッグして左右に動かすと、ここが自動で書き換わります。
right_margin =
";

#[derive(Debug, Clone, PartialEq)]
pub struct City {
    pub label: String,
    /// `None` when the time zone name in the settings file is unknown.
    pub tz: Option<Tz>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub cities: Vec<City>,
    pub right_margin: Option<i32>,
}

impl Default for Config {
    fn default() -> Self {
        parse_config(DEFAULT_CONFIG)
    }
}

fn lookup_builtin(label: &str) -> Option<Tz> {
    CITIES
        .iter()
        .find(|(l, _, _)| l.eq_ignore_ascii_case(label))
        .and_then(|(_, tz, _)| tz.parse().ok())
}

fn parse_city(token: &str) -> Option<City> {
    let token = token.trim();
    if token.is_empty() {
        return None;
    }
    if let Some((label, tz)) = token.split_once('=') {
        let label = label.trim();
        let label = if label.is_empty() { "?" } else { label };
        Some(City { label: label.chars().take(4).collect(), tz: tz.trim().parse().ok() })
    } else {
        Some(City { label: token.to_ascii_uppercase().chars().take(4).collect(), tz: lookup_builtin(token) })
    }
}

/// Parses the settings file. Unknown keys are ignored; missing keys keep defaults.
pub fn parse_config(text: &str) -> Config {
    let mut cities: Option<Vec<City>> = None;
    let mut right_margin = None;
    for line in text.trim_start_matches('\u{feff}').lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else { continue };
        let value = value.trim();
        match key.trim().to_ascii_lowercase().as_str() {
            "cities" => {
                let list: Vec<City> = value.split(',').filter_map(parse_city).take(MAX_ROWS).collect();
                if !list.is_empty() {
                    cities = Some(list);
                }
            }
            "right_margin" => right_margin = value.parse::<i32>().ok().filter(|v| *v >= 0),
            _ => {}
        }
    }
    let cities = cities.unwrap_or_else(|| {
        ["STO", "LON", "BOS"].iter().filter_map(|c| parse_city(c)).collect()
    });
    Config { cities, right_margin }
}

/// Replaces (or appends) `key = value` in the settings text, keeping comments
/// and everything else the user wrote.
pub fn set_key(text: &str, key: &str, value: &str) -> String {
    let mut out = Vec::new();
    let mut done = false;
    for line in text.trim_start_matches('\u{feff}').lines() {
        let t = line.trim();
        let is_key = !t.starts_with(';')
            && !t.starts_with('#')
            && t.split_once('=').is_some_and(|(k, _)| k.trim().eq_ignore_ascii_case(key));
        if is_key && !done {
            out.push(format!("{key} = {value}"));
            done = true;
        } else if !is_key {
            out.push(line.to_string());
        }
    }
    if !done {
        out.push(format!("{key} = {value}"));
    }
    let mut s = out.join("\r\n");
    s.push_str("\r\n");
    s
}

/// Calendar-day difference between the city's date and the PC's date at the same instant.
pub fn day_offset(now: DateTime<Utc>, tz: Tz, pc_date: NaiveDate) -> i64 {
    (now.with_timezone(&tz).date_naive() - pc_date).num_days()
}

/// "+1", "−1" (U+2212 minus sign) or "" when the dates are equal.
pub fn marker(offset: i64) -> String {
    match offset {
        0 => String::new(),
        n if n > 0 => format!("+{n}"),
        n => format!("\u{2212}{}", -n),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub label: String,
    pub time: String,
    pub marker: String,
}

pub fn rows(cfg: &Config, now: DateTime<Utc>, pc_date: NaiveDate) -> Vec<Row> {
    cfg.cities
        .iter()
        .map(|c| match c.tz {
            Some(tz) => Row {
                label: c.label.clone(),
                time: now.with_timezone(&tz).format("%H:%M").to_string(),
                marker: marker(day_offset(now, tz, pc_date)),
            },
            None => Row { label: c.label.clone(), time: "--:--".into(), marker: String::new() },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn utc(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, mo, d, h, mi, 0).unwrap()
    }
    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn default_config_is_sto_lon_bos() {
        let c = Config::default();
        let labels: Vec<_> = c.cities.iter().map(|c| c.label.as_str()).collect();
        assert_eq!(labels, ["STO", "LON", "BOS"]);
        assert!(c.cities.iter().all(|c| c.tz.is_some()));
        assert_eq!(c.right_margin, None);
    }

    #[test]
    fn all_builtin_cities_resolve() {
        for (label, _, _) in CITIES {
            assert!(lookup_builtin(label).is_some(), "{label}");
        }
    }

    #[test]
    fn parse_custom_and_limits() {
        let c = parse_config("\u{feff}cities = sin, PAR=Europe/Paris, XXX, LAX, TYO\nright_margin = 120\n");
        let labels: Vec<_> = c.cities.iter().map(|c| c.label.as_str()).collect();
        assert_eq!(labels, ["SIN", "PAR", "XXX"]);
        assert_eq!(c.cities[1].tz, Some(chrono_tz::Europe::Paris));
        assert_eq!(c.cities[2].tz, None);
        assert_eq!(c.right_margin, Some(120));
    }

    #[test]
    fn set_key_replaces_and_keeps_comments() {
        let s = set_key(DEFAULT_CONFIG, "right_margin", "200");
        assert!(s.contains("right_margin = 200"));
        assert!(s.contains("; cities:"));
        assert_eq!(parse_config(&s).right_margin, Some(200));
        let s2 = set_key("cities = SIN\n", "right_margin", "5");
        assert_eq!(parse_config(&s2).right_margin, Some(5));
    }

    #[test]
    fn markers() {
        assert_eq!(marker(0), "");
        assert_eq!(marker(1), "+1");
        assert_eq!(marker(-1), "\u{2212}1");
    }

    // PC in Tokyo, 2026-11-14 09:05 JST = 2026-11-14 00:05 UTC.
    #[test]
    fn tokyo_pc_morning() {
        let now = utc(2026, 11, 14, 0, 5);
        let pc = date(2026, 11, 14);
        let r = rows(&Config::default(), now, pc);
        assert_eq!(r[0], Row { label: "STO".into(), time: "01:05".into(), marker: String::new() });
        assert_eq!(r[1], Row { label: "LON".into(), time: "00:05".into(), marker: String::new() });
        assert_eq!(r[2], Row { label: "BOS".into(), time: "19:05".into(), marker: "\u{2212}1".into() });
    }

    // PC in Boston, 2026-11-13 19:05 EST = 2026-11-14 00:05 UTC → Stockholm and London are +1.
    #[test]
    fn boston_pc_evening() {
        let now = utc(2026, 11, 14, 0, 5);
        let r = rows(&Config::default(), now, date(2026, 11, 13));
        assert_eq!(r[0].marker, "+1");
        assert_eq!(r[1].marker, "+1");
        assert_eq!(r[2].marker, "");
    }

    // DST: Europe and US switch on different dates. 2026-03-20 12:00 UTC:
    // Boston is already on EDT (UTC-4) but Stockholm is still CET (UTC+1).
    #[test]
    fn dst_gap_weeks() {
        let now = utc(2026, 3, 20, 12, 0);
        let r = rows(&Config::default(), now, date(2026, 3, 20));
        assert_eq!(r[0].time, "13:00");
        assert_eq!(r[1].time, "12:00");
        assert_eq!(r[2].time, "08:00");
        // Summer: Stockholm CEST (UTC+2), London BST (UTC+1).
        let r = rows(&Config::default(), utc(2026, 7, 1, 12, 0), date(2026, 7, 1));
        assert_eq!((r[0].time.as_str(), r[1].time.as_str(), r[2].time.as_str()), ("14:00", "13:00", "08:00"));
    }

    // The largest real difference is 2 days (Kiritimati UTC+14 vs Pago Pago UTC-11).
    #[test]
    fn two_day_offset_is_possible() {
        let now = utc(2026, 11, 14, 11, 30); // Pago Pago 00:30 on the 14th, Kiritimati 01:30 on the 15th
        let pc = date(2026, 11, 14);
        assert_eq!(day_offset(now, chrono_tz::Pacific::Kiritimati, pc), 1);
        let pc_pago = now.with_timezone(&chrono_tz::Pacific::Pago_Pago).date_naive();
        assert_eq!(day_offset(now, chrono_tz::Pacific::Kiritimati, pc_pago), 1);
        let now = utc(2026, 11, 14, 10, 30); // Pago Pago 23:30 on the 13th, Kiritimati 00:30 on the 15th
        let pc_pago = now.with_timezone(&chrono_tz::Pacific::Pago_Pago).date_naive();
        assert_eq!(day_offset(now, chrono_tz::Pacific::Kiritimati, pc_pago), 2);
        assert_eq!(marker(-2), "\u{2212}2");
    }
}
