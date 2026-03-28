use chrono::{DateTime, Datelike, NaiveDate, TimeZone, Timelike, Utc};

use crate::error::AppError;

/// A parsed 6-field cron expression (sec min hour dom month dow) that can compute next fire times.
#[derive(Debug, Clone)]
pub struct CronSchedule {
    seconds: FieldSpec,
    minutes: FieldSpec,
    hours: FieldSpec,
    day_of_month: FieldSpec,
    month: FieldSpec,
    day_of_week: FieldSpec,
}

/// Represents a single cron field: either all values (`*`) or a specific set.
#[derive(Debug, Clone)]
enum FieldSpec {
    All,
    Values(Vec<u32>),
}

/// Mutable cursor tracking the current position while searching for a cron match.
struct CronCursor {
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
}

impl CronCursor {
    fn from_after(after: &DateTime<Utc>) -> Self {
        let mut c = Self {
            year: after.year(),
            month: after.month(),
            day: after.day(),
            hour: after.hour(),
            minute: after.minute(),
            second: after.second() + 1,
        };
        // Carry over
        if c.second >= 60 { c.second = 0; c.minute += 1; }
        if c.minute >= 60 { c.minute = 0; c.hour += 1; }
        if c.hour >= 24 { c.hour = 0; c.day += 1; }
        c
    }

    fn reset_to_day(&mut self) { self.hour = 0; self.minute = 0; self.second = 0; }
    fn reset_to_hour(&mut self) { self.minute = 0; self.second = 0; }
    fn reset_to_minute(&mut self) { self.second = 0; }

    fn advance_month(&mut self) {
        self.month += 1;
        self.day = 1;
        self.reset_to_day();
        if self.month > 12 { self.month = 1; self.year += 1; }
    }

    fn advance_day(&mut self, days_in_month: u32) {
        self.day += 1;
        self.reset_to_day();
        if self.day > days_in_month { self.advance_month(); }
    }

    fn advance_hour(&mut self, days_in_month: u32) {
        self.hour += 1;
        self.reset_to_hour();
        if self.hour >= 24 { self.hour = 0; self.advance_day(days_in_month); }
    }

    fn advance_minute(&mut self, days_in_month: u32) {
        self.minute += 1;
        self.reset_to_minute();
        if self.minute >= 60 { self.minute = 0; self.advance_hour(days_in_month); }
    }

    fn to_datetime(&self, hour: u32, minute: u32, second: u32) -> Option<DateTime<Utc>> {
        Utc.with_ymd_and_hms(self.year, self.month, self.day, hour, minute, second)
            .single()
    }
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
        let mut c = CronCursor::from_after(&after);
        let max_year = c.year + 5;

        while c.year <= max_year {
            if !self.advance_to_matching_date(&mut c, max_year) {
                return None;
            }
            if c.year > max_year {
                return None;
            }

            let dim = days_in_month(c.year, c.month);

            if let Some(dt) = self.find_matching_time(&mut c, dim) {
                return Some(dt);
            }
        }

        None
    }

    /// Advances the cursor to the next date (year/month/day) that matches the cron fields.
    /// Returns false if no match is possible within the year limit.
    fn advance_to_matching_date(&self, c: &mut CronCursor, max_year: i32) -> bool {
        loop {
            if c.year > max_year { return false; }

            // Find next matching month
            let Some(m) = self.month.next_match(c.month, 1, 12) else {
                c.year += 1; c.month = 1; c.day = 1; c.reset_to_day();
                continue;
            };
            if m > c.month { c.month = m; c.day = 1; c.reset_to_day(); }

            let dim = days_in_month(c.year, c.month);

            // Find next matching day of month
            let Some(d) = self.day_of_month.next_match(c.day, 1, dim) else {
                c.advance_month();
                continue;
            };
            if d > c.day { c.day = d; c.reset_to_day(); }

            if c.day > dim { c.advance_month(); continue; }

            // Check day of week
            let Some(date) = NaiveDate::from_ymd_opt(c.year, c.month, c.day) else {
                c.advance_month();
                continue;
            };
            if !self.day_of_week.matches(date.weekday().num_days_from_sunday()) {
                c.advance_day(dim);
                continue;
            }

            return true;
        }
    }

    /// Given a cursor on a valid date, finds the next matching hour:minute:second.
    /// Returns None and advances the cursor if no time matches on this date.
    fn find_matching_time(&self, c: &mut CronCursor, dim: u32) -> Option<DateTime<Utc>> {
        let Some(h) = self.hours.next_match(c.hour, 0, 23) else {
            c.advance_day(dim);
            return None;
        };
        if h > c.hour { c.hour = h; c.reset_to_hour(); }

        let Some(min) = self.minutes.next_match(c.minute, 0, 59) else {
            c.advance_hour(dim);
            return None;
        };
        if min > c.minute { c.minute = min; c.reset_to_minute(); }

        let Some(s) = self.seconds.next_match(c.second, 0, 59) else {
            c.advance_minute(dim);
            return None;
        };

        if let Some(dt) = c.to_datetime(h, min, s) {
            return Some(dt);
        }

        // Shouldn't happen, but advance
        c.second = s + 1;
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

/// Parses a single cron field string into a `FieldSpec`, handling `*`, ranges, steps, and lists.
fn parse_field(field: &str, min: u32, max: u32) -> Result<FieldSpec, AppError> {
    if field == "*" {
        return Ok(FieldSpec::All);
    }
    if let Some(pos) = field.find('/') {
        return parse_step_field(field, pos, min, max);
    }
    parse_range_or_list(field, min, max)
}

/// Parses a step field like `*/N` or `M-N/S`.
fn parse_step_field(field: &str, slash_pos: usize, min: u32, max: u32) -> Result<FieldSpec, AppError> {
    let base = &field[..slash_pos];
    let step_str = &field[slash_pos + 1..];
    let step: u32 = step_str
        .parse()
        .map_err(|_| AppError::BadRequest(format!("invalid step value: {step_str}")))?;
    if step == 0 {
        return Err(AppError::BadRequest("step cannot be zero".into()));
    }
    let (start, end) = if base == "*" {
        (min, max)
    } else if let Some((lo, hi)) = base.split_once('-') {
        (parse_u32(lo, "range start")?, parse_u32(hi, "range end")?)
    } else {
        (parse_u32(base, "field")?, max)
    };
    let mut vals = Vec::new();
    let mut v = start;
    while v <= end {
        vals.push(v);
        v += step;
    }
    vals.sort();
    Ok(FieldSpec::Values(vals))
}

/// Parses a comma-separated list of values and/or ranges like `1,3,5` or `1-5,10`.
fn parse_range_or_list(field: &str, min: u32, max: u32) -> Result<FieldSpec, AppError> {
    let mut vals = Vec::new();
    for part in field.split(',') {
        if let Some((lo, hi)) = part.split_once('-') {
            let lo = parse_u32(lo, "range start")?;
            let hi = parse_u32(hi, "range end")?;
            if lo > hi || lo < min || hi > max {
                return Err(AppError::BadRequest(format!(
                    "range {lo}-{hi} out of bounds ({min}-{max})"
                )));
            }
            vals.extend(lo..=hi);
        } else {
            let v = parse_u32(part, "value")?;
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

/// Parses a string as u32 with a descriptive error label.
fn parse_u32(s: &str, label: &str) -> Result<u32, AppError> {
    s.parse()
        .map_err(|_| AppError::BadRequest(format!("invalid {label}: {s}")))
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
