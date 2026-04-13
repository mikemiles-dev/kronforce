//! Calendar helper functions for weekday-based schedule calculations.

use chrono::{Datelike, NaiveDate};

pub fn parse_weekday(s: &str) -> chrono::Weekday {
    match s.to_lowercase().as_str() {
        "monday" | "mon" => chrono::Weekday::Mon,
        "tuesday" | "tue" => chrono::Weekday::Tue,
        "wednesday" | "wed" => chrono::Weekday::Wed,
        "thursday" | "thu" => chrono::Weekday::Thu,
        "friday" | "fri" => chrono::Weekday::Fri,
        "saturday" | "sat" => chrono::Weekday::Sat,
        "sunday" | "sun" => chrono::Weekday::Sun,
        _ => chrono::Weekday::Mon,
    }
}

pub fn nth_weekday_of_month(
    year: i32,
    month: u32,
    weekday: chrono::Weekday,
    nth: u32,
) -> Option<NaiveDate> {
    let first = NaiveDate::from_ymd_opt(year, month, 1)?;
    let first_wd = first.weekday();
    let days_ahead =
        (weekday.num_days_from_monday() as i32 - first_wd.num_days_from_monday() as i32 + 7) % 7;
    let target = first + chrono::Duration::days(days_ahead as i64 + (nth as i64 - 1) * 7);
    if target.month() == month {
        Some(target)
    } else {
        None
    }
}

pub fn last_weekday_of_month(year: i32, month: u32, weekday: chrono::Weekday) -> Option<NaiveDate> {
    let last_day = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }?
    .pred_opt()?;
    let mut d = last_day;
    while d.weekday() != weekday {
        d = d.pred_opt()?;
    }
    Some(d)
}
