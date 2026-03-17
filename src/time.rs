use chrono::{DateTime, Duration, Utc};

pub fn parse_time(input: &str) -> Result<DateTime<Utc>, String> {
    if input == "now" {
        return Ok(Utc::now());
    }

    // Relative time: -<number><unit>
    if let Some(rest) = input.strip_prefix('-') {
        if let Some((num_str, unit)) = split_relative(rest) {
            let num: i64 = num_str
                .parse()
                .map_err(|_| format!("Invalid relative time: {input}"))?;
            let duration = match unit {
                "s" => Duration::seconds(num),
                "m" => Duration::minutes(num),
                "h" => Duration::hours(num),
                "d" => Duration::days(num),
                "w" => Duration::weeks(num),
                _ => return Err(format!("Unknown time unit '{unit}' in '{input}'. Use s, m, h, d, or w.")),
            };
            return Ok(Utc::now() - duration);
        }
    }

    // ISO 8601
    input
        .parse::<DateTime<Utc>>()
        .map_err(|e| format!("Cannot parse time '{input}': {e}"))
}

fn split_relative(s: &str) -> Option<(&str, &str)> {
    let unit_pos = s.find(|c: char| c.is_alphabetic())?;
    let (num, unit) = s.split_at(unit_pos);
    if num.is_empty() || unit.is_empty() {
        return None;
    }
    Some((num, unit))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_now() {
        let t = parse_time("now").unwrap();
        let diff = (Utc::now() - t).num_seconds().abs();
        assert!(diff < 2);
    }

    #[test]
    fn test_parse_relative_minutes() {
        let t = parse_time("-15m").unwrap();
        let diff = (Utc::now() - t).num_minutes();
        assert!((14..=16).contains(&diff));
    }

    #[test]
    fn test_parse_relative_hours() {
        let t = parse_time("-24h").unwrap();
        let diff = (Utc::now() - t).num_hours();
        assert!((23..=25).contains(&diff));
    }

    #[test]
    fn test_parse_relative_weeks() {
        let t = parse_time("-2w").unwrap();
        let diff = (Utc::now() - t).num_days();
        assert!((13..=15).contains(&diff));
    }

    #[test]
    fn test_parse_invalid() {
        assert!(parse_time("garbage").is_err());
    }

    #[test]
    fn test_parse_unknown_unit() {
        assert!(parse_time("-5x").is_err());
    }
}
