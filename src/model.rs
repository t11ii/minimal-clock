//! Platform-independent part: settings file, city table, time formatting and
//! day-offset calculation. Unit-tested on any OS (`cargo test`).

use chrono::{DateTime, NaiveDate, Utc};
use chrono_tz::Tz;

pub const MAX_ROWS: usize = 3;

/// Built-in cities: (region, label, IANA time zone, Japanese name).
/// Labels are IATA city codes. Africa and South America are intentionally left out.
///
/// Only the time zones used here are compiled into the exe: keep the
/// `CHRONO_TZ_TIMEZONE_FILTER` regex in `.cargo/config.toml` in sync
/// (the `all_builtin_cities_resolve` test fails otherwise).
pub const CITIES: &[(&str, &str, &str, &str)] = &[
    ("ヨーロッパ", "LON", "Europe/London", "ロンドン"),
    ("ヨーロッパ", "DUB", "Europe/Dublin", "ダブリン"),
    ("ヨーロッパ", "LIS", "Europe/Lisbon", "リスボン"),
    ("ヨーロッパ", "PAR", "Europe/Paris", "パリ"),
    ("ヨーロッパ", "BRU", "Europe/Brussels", "ブリュッセル"),
    ("ヨーロッパ", "AMS", "Europe/Amsterdam", "アムステルダム"),
    ("ヨーロッパ", "FRA", "Europe/Berlin", "フランクフルト"),
    ("ヨーロッパ", "BER", "Europe/Berlin", "ベルリン"),
    ("ヨーロッパ", "MUC", "Europe/Berlin", "ミュンヘン"),
    ("ヨーロッパ", "ZRH", "Europe/Zurich", "チューリッヒ"),
    ("ヨーロッパ", "MIL", "Europe/Rome", "ミラノ"),
    ("ヨーロッパ", "ROM", "Europe/Rome", "ローマ"),
    ("ヨーロッパ", "MAD", "Europe/Madrid", "マドリード"),
    ("ヨーロッパ", "BCN", "Europe/Madrid", "バルセロナ"),
    ("ヨーロッパ", "VIE", "Europe/Vienna", "ウィーン"),
    ("ヨーロッパ", "PRG", "Europe/Prague", "プラハ"),
    ("ヨーロッパ", "WAW", "Europe/Warsaw", "ワルシャワ"),
    ("ヨーロッパ", "BUD", "Europe/Budapest", "ブダペスト"),
    ("ヨーロッパ", "CPH", "Europe/Copenhagen", "コペンハーゲン"),
    ("ヨーロッパ", "STO", "Europe/Stockholm", "ストックホルム"),
    ("ヨーロッパ", "OSL", "Europe/Oslo", "オスロ"),
    ("ヨーロッパ", "HEL", "Europe/Helsinki", "ヘルシンキ"),
    ("ヨーロッパ", "ATH", "Europe/Athens", "アテネ"),
    ("ヨーロッパ", "IST", "Europe/Istanbul", "イスタンブール"),
    ("ヨーロッパ", "IEV", "Europe/Kyiv", "キーウ"),
    ("ヨーロッパ", "MOW", "Europe/Moscow", "モスクワ"),
    ("中東・南アジア", "DXB", "Asia/Dubai", "ドバイ"),
    ("中東・南アジア", "DOH", "Asia/Qatar", "ドーハ"),
    ("中東・南アジア", "RUH", "Asia/Riyadh", "リヤド"),
    ("中東・南アジア", "TLV", "Asia/Jerusalem", "テルアビブ"),
    ("中東・南アジア", "THR", "Asia/Tehran", "テヘラン"),
    ("中東・南アジア", "KHI", "Asia/Karachi", "カラチ"),
    ("中東・南アジア", "DEL", "Asia/Kolkata", "デリー"),
    ("中東・南アジア", "BOM", "Asia/Kolkata", "ムンバイ"),
    ("中東・南アジア", "BLR", "Asia/Kolkata", "ベンガルール"),
    ("中東・南アジア", "DAC", "Asia/Dhaka", "ダッカ"),
    ("東南アジア・東アジア", "BKK", "Asia/Bangkok", "バンコク"),
    ("東南アジア・東アジア", "HAN", "Asia/Ho_Chi_Minh", "ハノイ"),
    ("東南アジア・東アジア", "SGN", "Asia/Ho_Chi_Minh", "ホーチミン"),
    ("東南アジア・東アジア", "KUL", "Asia/Kuala_Lumpur", "クアラルンプール"),
    ("東南アジア・東アジア", "SIN", "Asia/Singapore", "シンガポール"),
    ("東南アジア・東アジア", "JKT", "Asia/Jakarta", "ジャカルタ"),
    ("東南アジア・東アジア", "MNL", "Asia/Manila", "マニラ"),
    ("東南アジア・東アジア", "HKG", "Asia/Hong_Kong", "香港"),
    ("東南アジア・東アジア", "TPE", "Asia/Taipei", "台北"),
    ("東南アジア・東アジア", "SHA", "Asia/Shanghai", "上海"),
    ("東南アジア・東アジア", "BJS", "Asia/Shanghai", "北京"),
    ("東南アジア・東アジア", "SZX", "Asia/Shanghai", "深圳"),
    ("東南アジア・東アジア", "SEL", "Asia/Seoul", "ソウル"),
    ("東南アジア・東アジア", "TYO", "Asia/Tokyo", "東京"),
    ("オセアニア・太平洋", "PER", "Australia/Perth", "パース"),
    ("オセアニア・太平洋", "ADL", "Australia/Adelaide", "アデレード"),
    ("オセアニア・太平洋", "BNE", "Australia/Brisbane", "ブリスベン"),
    ("オセアニア・太平洋", "SYD", "Australia/Sydney", "シドニー"),
    ("オセアニア・太平洋", "MEL", "Australia/Melbourne", "メルボルン"),
    ("オセアニア・太平洋", "AKL", "Pacific/Auckland", "オークランド"),
    ("オセアニア・太平洋", "GUM", "Pacific/Guam", "グアム"),
    ("オセアニア・太平洋", "HNL", "Pacific/Honolulu", "ホノルル"),
    ("北米", "ANC", "America/Anchorage", "アンカレッジ"),
    ("北米", "SEA", "America/Los_Angeles", "シアトル"),
    ("北米", "SFO", "America/Los_Angeles", "サンフランシスコ"),
    ("北米", "LAX", "America/Los_Angeles", "ロサンゼルス"),
    ("北米", "YVR", "America/Vancouver", "バンクーバー"),
    ("北米", "PHX", "America/Phoenix", "フェニックス"),
    ("北米", "DEN", "America/Denver", "デンバー"),
    ("北米", "DFW", "America/Chicago", "ダラス"),
    ("北米", "HOU", "America/Chicago", "ヒューストン"),
    ("北米", "CHI", "America/Chicago", "シカゴ"),
    ("北米", "MEX", "America/Mexico_City", "メキシコシティ"),
    ("北米", "ATL", "America/New_York", "アトランタ"),
    ("北米", "MIA", "America/New_York", "マイアミ"),
    ("北米", "WAS", "America/New_York", "ワシントンD.C."),
    ("北米", "NYC", "America/New_York", "ニューヨーク"),
    ("北米", "BOS", "America/New_York", "ボストン"),
    ("北米", "YTO", "America/Toronto", "トロント"),
    ("北米", "YMQ", "America/Toronto", "モントリオール"),
    ("その他", "UTC", "Etc/UTC", "協定世界時"),
];

/// Text of a new settings file, including the list of built-in cities.
pub fn default_config() -> String {
    let mut s = String::from(
        "; minimal-clock 設定ファイル（保存するとすぐ反映されます）\n\
         ;\n\
         ; cities: 表示する都市。上から順に最大3つ、カンマ区切りで書きます。\n\
         ;   表示名を変えたいときは「表示名=都市」と書きます。例: OSA=TYO\n\
         ;\n\
         ;   使える都市:\n",
    );
    let mut region = "";
    let mut line: Vec<String> = Vec::new();
    for (r, label, _, ja) in CITIES {
        if *r != region || line.len() == 5 {
            if !line.is_empty() {
                s.push_str(&format!(";       {}\n", line.join(" / ")));
                line.clear();
            }
            if *r != region {
                s.push_str(&format!(";     {r}\n"));
                region = r;
            }
        }
        line.push(format!("{label} {ja}"));
    }
    if !line.is_empty() {
        s.push_str(&format!(";       {}\n", line.join(" / ")));
    }
    s.push_str(
        "cities = STO, LON, BOS\n\
         \n\
         ; right_margin: タスクバー右端から時計の右端までの距離（100%表示時のpx）。空欄なら自動。\n\
         ;   時計をドラッグして左右に動かすと、ここが自動で書き換わります。\n\
         right_margin =\n",
    );
    s
}

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
        parse_config(&default_config())
    }
}

fn lookup_builtin(label: &str) -> Option<Tz> {
    CITIES
        .iter()
        .find(|(_, l, _, _)| l.eq_ignore_ascii_case(label))
        .and_then(|(_, _, tz, _)| tz.parse().ok())
}

fn parse_city(token: &str) -> Option<City> {
    let token = token.trim();
    if token.is_empty() {
        return None;
    }
    if let Some((label, target)) = token.split_once('=') {
        // "OSA=TYO" (another built-in city) or "OSA=Asia/Tokyo" (a compiled-in IANA name).
        let label = label.trim();
        let label = if label.is_empty() { "?" } else { label };
        let target = target.trim();
        let tz = lookup_builtin(target).or_else(|| target.parse().ok());
        Some(City { label: label.chars().take(4).collect(), tz })
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

    // Every built-in city's zone must be compiled in (see CHRONO_TZ_TIMEZONE_FILTER).
    #[test]
    fn all_builtin_cities_resolve() {
        for (_, label, tz, _) in CITIES {
            assert!(lookup_builtin(label).is_some(), "{label} -> {tz} is missing from CHRONO_TZ_TIMEZONE_FILTER");
        }
    }

    #[test]
    fn labels_are_unique_and_listed_in_settings_file() {
        let text = default_config();
        let mut seen = std::collections::HashSet::new();
        for (_, label, _, ja) in CITIES {
            assert!(seen.insert(*label), "duplicate label {label}");
            assert!(text.contains(&format!("{label} {ja}")), "{label} not listed");
        }
    }

    // The exe only carries the zones it needs; Africa and South America are left out.
    #[test]
    fn unused_zones_are_not_compiled_in() {
        for name in ["Africa/Cairo", "Africa/Johannesburg", "America/Sao_Paulo", "America/Argentina/Buenos_Aires"] {
            assert!(name.parse::<Tz>().is_err(), "{name} should be filtered out");
        }
    }

    #[test]
    fn parse_custom_and_limits() {
        let c = parse_config("\u{feff}cities = sin, OSA=TYO, XXX, LAX, TYO\nright_margin = 120\n");
        let labels: Vec<_> = c.cities.iter().map(|c| c.label.as_str()).collect();
        assert_eq!(labels, ["SIN", "OSA", "XXX"]);
        assert_eq!(c.cities[1].tz, Some(chrono_tz::Asia::Tokyo));
        assert_eq!(c.cities[2].tz, None);
        assert_eq!(c.right_margin, Some(120));
        let c = parse_config("cities = Lon=Europe/London");
        assert_eq!(c.cities[0], City { label: "Lon".into(), tz: Some(chrono_tz::Europe::London) });
    }

    // Spot checks across regions, including non-hour offsets and zones without DST.
    #[test]
    fn region_spot_checks() {
        let c = parse_config("cities = DEL, THR, ADL");
        let r = rows(&c, utc(2026, 1, 15, 0, 0), date(2026, 1, 15));
        assert_eq!(r[0].time, "05:30"); // India UTC+5:30
        assert_eq!(r[1].time, "03:30"); // Iran UTC+3:30
        assert_eq!(r[2].time, "10:30"); // Adelaide summer UTC+10:30
        let c = parse_config("cities = PHX, HNL, AKL");
        let r = rows(&c, utc(2026, 7, 1, 0, 0), date(2026, 7, 1));
        assert_eq!((r[0].time.as_str(), r[0].marker.as_str()), ("17:00", "\u{2212}1")); // Arizona: no DST, UTC-7
        assert_eq!((r[1].time.as_str(), r[1].marker.as_str()), ("14:00", "\u{2212}1")); // Hawaii UTC-10
        assert_eq!((r[2].time.as_str(), r[2].marker.as_str()), ("12:00", "")); // Auckland winter UTC+12
    }

    #[test]
    fn set_key_replaces_and_keeps_comments() {
        let s = set_key(&default_config(), "right_margin", "200");
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

    // Widest pair in the list: Auckland (UTC+13 in January) vs Honolulu (UTC-10).
    #[test]
    fn auckland_vs_honolulu() {
        let akl = chrono_tz::Pacific::Auckland;
        let hnl = chrono_tz::Pacific::Honolulu;
        let now = utc(2026, 1, 15, 10, 30); // Auckland 23:30 on the 15th, Honolulu 00:30 on the 15th
        assert_eq!(day_offset(now, akl, now.with_timezone(&hnl).date_naive()), 0);
        let now = utc(2026, 1, 15, 11, 30); // Auckland 00:30 on the 16th, Honolulu 01:30 on the 15th
        assert_eq!(day_offset(now, akl, now.with_timezone(&hnl).date_naive()), 1);
        assert_eq!(day_offset(now, hnl, now.with_timezone(&akl).date_naive()), -1);
        assert_eq!(marker(-2), "\u{2212}2");
    }
}
