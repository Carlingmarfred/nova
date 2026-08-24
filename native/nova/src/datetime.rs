//! Slim datetime support (N06c): UTC ISO-8601 text + Unix epoch seconds.
//! Full parse/format arrives with the typed layer; this covers the stdlib
//! surface the 0.2 audience (scripting/CLI) actually needs.

use std::time::{SystemTime, UNIX_EPOCH};

/// Seconds since the Unix epoch (may carry sub-second precision).
pub fn epoch() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

/// Current UTC time as `YYYY-MM-DDTHH:MM:SSZ` (20 chars).
pub fn now_text() -> String {
    let secs = epoch().floor() as i64;
    format_text(secs)
}

/// Formats a Unix-epoch second count as `YYYY-MM-DDTHH:MM:SSZ`.
pub fn format_text(epoch_secs: i64) -> String {
    let days = epoch_secs.div_euclid(86_400);
    let secs_of_day = epoch_secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    let hh = secs_of_day / 3600;
    let mm = (secs_of_day % 3600) / 60;
    let ss = secs_of_day % 60;
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

/// Howard Hinnant's `civil_from_days` algorithm (public-domain shape).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097); // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_epoch_second() {
        // 2026-01-01T00:00:00Z == 1767225600
        assert_eq!(format_text(1_767_225_600), "2026-01-01T00:00:00Z");
    }

    #[test]
    fn known_epoch_leap_year_day() {
        // 2024-02-29T12:30:45Z == 1709212345? compute via known anchor:
        // 2024-03-01T00:00:00Z == 1709251200 -> minus one day minus offsets
        assert_eq!(format_text(1_709_251_200 - 86_400 + 45_145), "2024-02-29T12:32:25Z");
    }

    #[test]
    fn epoch_zero_is_1970() {
        assert_eq!(format_text(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn now_text_shape_is_iso() {
        let t = now_text();
        assert_eq!(t.len(), 20);
        assert!(t.ends_with('Z'));
        assert_eq!(&t[4..5], "-");
        assert_eq!(&t[10..11], "T");
    }
}
