use chrono::{DateTime, Datelike, NaiveDate, TimeZone, Timelike, Utc};

use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct CronSchedule {
    seconds: FieldSpec,
    minutes: FieldSpec,
    hours: FieldSpec,
    day_of_month: FieldSpec,
    month: FieldSpec,
    day_of_week: FieldSpec,
}

#[derive(Debug, Clone)]
enum FieldSpec {
    All,
    Values(Vec<u32>),
}

impl CronSchedule {
    /// Parse a 6-field cron expression: "sec min hour dom month dow"
    pub fn parse(expr: &str) -> Result<Self, AppError> {
        let fields: Vec<&str> = expr.split_whitespace().collect();
        if fields.len() != 6 {
            return Err(AppError::BadRequest(format!(
                "cron expression must have 6 fields (sec min hour dom month dow), got {}",
                fields.len()
            )));
        }
        Ok(Self {
            seconds: parse_field(fields[0], 0, 59)?,
            minutes: parse_field(fields[1], 0, 59)?,
            hours: parse_field(fields[2], 0, 23)?,
            day_of_month: parse_field(fields[3], 1, 31)?,
            month: parse_field(fields[4], 1, 12)?,
            day_of_week: parse_field(fields[5], 0, 6)?,
        })
    }

    /// Find the next datetime after `after` that matches this cron expression.
    pub fn next_after(&self, after: DateTime<Utc>) -> Option<DateTime<Utc>> {
        // Start from the next second
        let mut year = after.year();
        let mut month = after.month();
        let mut day = after.day();
        let mut hour = after.hour();
        let mut minute = after.minute();
        let mut second = after.second() + 1;

        // Carry over
        if second >= 60 {
            second = 0;
            minute += 1;
        }
        if minute >= 60 {
            minute = 0;
            hour += 1;
        }
        if hour >= 24 {
            hour = 0;
            day += 1;
        }

        let max_year = year + 5;

        while year <= max_year {
            // Find next matching month
            let Some(m) = self.month.next_match(month, 1, 12) else {
                year += 1;
                month = 1;
                day = 1;
                hour = 0;
                minute = 0;
                second = 0;
                continue;
            };
            if m > month {
                month = m;
                day = 1;
                hour = 0;
                minute = 0;
                second = 0;
            }

            let days_in_month = days_in_month(year, month);

            // Find next matching day
            let Some(d) = self.day_of_month.next_match(day, 1, days_in_month) else {
                month += 1;
                day = 1;
                hour = 0;
                minute = 0;
                second = 0;
                if month > 12 {
                    month = 1;
                    year += 1;
                }
                continue;
            };

            if d > day {
                day = d;
                hour = 0;
                minute = 0;
                second = 0;
            }

            // Validate day exists
            if day > days_in_month {
                month += 1;
                day = 1;
                hour = 0;
                minute = 0;
                second = 0;
                if month > 12 {
                    month = 1;
                    year += 1;
                }
                continue;
            }

            // Check day of week
            if let Some(date) = NaiveDate::from_ymd_opt(year, month, day) {
                let dow = date.weekday().num_days_from_sunday();
                if !self.day_of_week.matches(dow) {
                    day += 1;
                    hour = 0;
                    minute = 0;
                    second = 0;
                    if day > days_in_month {
                        month += 1;
                        day = 1;
                        if month > 12 {
                            month = 1;
                            year += 1;
                        }
                    }
                    continue;
                }
            } else {
                month += 1;
                day = 1;
                hour = 0;
                minute = 0;
                second = 0;
                if month > 12 {
                    month = 1;
                    year += 1;
                }
                continue;
            }

            // Find next matching hour
            let Some(h) = self.hours.next_match(hour, 0, 23) else {
                day += 1;
                hour = 0;
                minute = 0;
                second = 0;
                if day > days_in_month {
                    month += 1;
                    day = 1;
                    if month > 12 {
                        month = 1;
                        year += 1;
                    }
                }
                continue;
            };
            if h > hour {
                hour = h;
                minute = 0;
                second = 0;
            }

            // Find next matching minute
            let Some(min) = self.minutes.next_match(minute, 0, 59) else {
                hour += 1;
                minute = 0;
                second = 0;
                if hour >= 24 {
                    hour = 0;
                    day += 1;
                    if day > days_in_month {
                        month += 1;
                        day = 1;
                        if month > 12 {
                            month = 1;
                            year += 1;
                        }
                    }
                }
                continue;
            };
            if min > minute {
                minute = min;
                second = 0;
            }

            // Find next matching second
            let Some(s) = self.seconds.next_match(second, 0, 59) else {
                minute += 1;
                second = 0;
                if minute >= 60 {
                    minute = 0;
                    hour += 1;
                    if hour >= 24 {
                        hour = 0;
                        day += 1;
                        if day > days_in_month {
                            month += 1;
                            day = 1;
                            if month > 12 {
                                month = 1;
                                year += 1;
                            }
                        }
                    }
                }
                continue;
            };

            // Build the result
            if let Some(dt) = Utc
                .with_ymd_and_hms(year, month, day, hour, min, s)
                .single()
            {
                return Some(dt);
            }

            // Shouldn't happen, but advance
            second = s + 1;
        }

        None
    }
}

impl FieldSpec {
    fn matches(&self, value: u32) -> bool {
        match self {
            FieldSpec::All => true,
            FieldSpec::Values(vals) => vals.contains(&value),
        }
    }

    fn next_match(&self, from: u32, min: u32, max: u32) -> Option<u32> {
        match self {
            FieldSpec::All => {
                if from <= max {
                    Some(from)
                } else {
                    None
                }
            }
            FieldSpec::Values(vals) => vals
                .iter()
                .copied()
                .find(|&v| v >= from && v >= min && v <= max),
        }
    }
}

fn parse_field(field: &str, min: u32, max: u32) -> Result<FieldSpec, AppError> {
    if field == "*" {
        return Ok(FieldSpec::All);
    }

    // Handle step: */N or M-N/S
    if let Some((base, step_str)) = field.split_once('/') {
        let step: u32 = step_str
            .parse()
            .map_err(|_| AppError::BadRequest(format!("invalid step value: {step_str}")))?;
        if step == 0 {
            return Err(AppError::BadRequest("step cannot be zero".into()));
        }
        let (start, end) = if base == "*" {
            (min, max)
        } else if let Some((lo, hi)) = base.split_once('-') {
            let lo: u32 = lo
                .parse()
                .map_err(|_| AppError::BadRequest(format!("invalid range start: {lo}")))?;
            let hi: u32 = hi
                .parse()
                .map_err(|_| AppError::BadRequest(format!("invalid range end: {hi}")))?;
            (lo, hi)
        } else {
            let start: u32 = base
                .parse()
                .map_err(|_| AppError::BadRequest(format!("invalid field: {base}")))?;
            (start, max)
        };
        let mut vals = Vec::new();
        let mut v = start;
        while v <= end {
            vals.push(v);
            v += step;
        }
        vals.sort();
        return Ok(FieldSpec::Values(vals));
    }

    // Handle comma-separated values and ranges
    let mut vals = Vec::new();
    for part in field.split(',') {
        if let Some((lo, hi)) = part.split_once('-') {
            let lo: u32 = lo
                .parse()
                .map_err(|_| AppError::BadRequest(format!("invalid range start: {lo}")))?;
            let hi: u32 = hi
                .parse()
                .map_err(|_| AppError::BadRequest(format!("invalid range end: {hi}")))?;
            if lo > hi || lo < min || hi > max {
                return Err(AppError::BadRequest(format!(
                    "range {lo}-{hi} out of bounds ({min}-{max})"
                )));
            }
            for v in lo..=hi {
                vals.push(v);
            }
        } else {
            let v: u32 = part
                .parse()
                .map_err(|_| AppError::BadRequest(format!("invalid value: {part}")))?;
            if v < min || v > max {
                return Err(AppError::BadRequest(format!(
                    "value {v} out of bounds ({min}-{max})"
                )));
            }
            vals.push(v);
        }
    }
    vals.sort();
    vals.dedup();
    Ok(FieldSpec::Values(vals))
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 => 31,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 {
                29
            } else {
                28
            }
        }
        3 => 31,
        4 => 30,
        5 => 31,
        6 => 30,
        7 => 31,
        8 => 31,
        9 => 30,
        10 => 31,
        11 => 30,
        12 => 31,
        _ => 30,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_every_second() {
        let cron = CronSchedule::parse("* * * * * *").unwrap();
        let base = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let next = cron.next_after(base).unwrap();
        assert_eq!(next, Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 1).unwrap());
    }

    #[test]
    fn test_every_minute() {
        let cron = CronSchedule::parse("0 * * * * *").unwrap();
        let base = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 30).unwrap();
        let next = cron.next_after(base).unwrap();
        assert_eq!(next, Utc.with_ymd_and_hms(2024, 1, 1, 0, 1, 0).unwrap());
    }

    #[test]
    fn test_specific_time() {
        let cron = CronSchedule::parse("0 30 14 * * *").unwrap();
        let base = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        let next = cron.next_after(base).unwrap();
        assert_eq!(next, Utc.with_ymd_and_hms(2024, 1, 1, 14, 30, 0).unwrap());
    }

    #[test]
    fn test_step() {
        let cron = CronSchedule::parse("*/15 * * * * *").unwrap();
        let base = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 10).unwrap();
        let next = cron.next_after(base).unwrap();
        assert_eq!(next, Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 15).unwrap());
    }

    #[test]
    fn test_invalid_field_count() {
        assert!(CronSchedule::parse("* * * * *").is_err());
    }
}
