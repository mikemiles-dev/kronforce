use chrono::{TimeZone, Utc};
use kronforce::cron_parser::CronSchedule;

// --- Valid parse tests ---

#[test]
fn test_parse_every_second() {
    assert!(CronSchedule::parse("* * * * * *").is_ok());
}

#[test]
fn test_parse_specific_time() {
    assert!(CronSchedule::parse("0 30 14 * * *").is_ok());
}

#[test]
fn test_parse_range_expression() {
    assert!(CronSchedule::parse("0 0 9-17 * * *").is_ok());
}

#[test]
fn test_parse_step_expression() {
    assert!(CronSchedule::parse("*/5 * * * * *").is_ok());
}

#[test]
fn test_parse_comma_list() {
    assert!(CronSchedule::parse("0 0 0 1,15 * *").is_ok());
}

#[test]
fn test_parse_day_of_week() {
    // 1 = Monday (num_days_from_sunday)
    assert!(CronSchedule::parse("0 0 9 * * 1-5").is_ok());
}

#[test]
fn test_parse_monthly_schedule() {
    assert!(CronSchedule::parse("0 0 0 1 * *").is_ok());
}

#[test]
fn test_parse_specific_month() {
    assert!(CronSchedule::parse("0 0 0 1 6 *").is_ok());
}

#[test]
fn test_parse_range_with_step() {
    assert!(CronSchedule::parse("0 0 8-18/2 * * *").is_ok());
}

// --- Invalid parse tests ---

#[test]
fn test_parse_too_few_fields() {
    let result = CronSchedule::parse("* * * * *");
    assert!(result.is_err());
}

#[test]
fn test_parse_too_many_fields() {
    let result = CronSchedule::parse("* * * * * * *");
    assert!(result.is_err());
}

#[test]
fn test_parse_empty_string() {
    let result = CronSchedule::parse("");
    assert!(result.is_err());
}

#[test]
fn test_parse_out_of_range_second() {
    let result = CronSchedule::parse("60 * * * * *");
    assert!(result.is_err());
}

#[test]
fn test_parse_out_of_range_minute() {
    let result = CronSchedule::parse("0 60 * * * *");
    assert!(result.is_err());
}

#[test]
fn test_parse_out_of_range_hour() {
    let result = CronSchedule::parse("0 0 24 * * *");
    assert!(result.is_err());
}

#[test]
fn test_parse_out_of_range_day_of_month() {
    let result = CronSchedule::parse("0 0 0 32 * *");
    assert!(result.is_err());
}

#[test]
fn test_parse_out_of_range_month() {
    let result = CronSchedule::parse("0 0 0 * 13 *");
    assert!(result.is_err());
}

#[test]
fn test_parse_out_of_range_day_of_week() {
    let result = CronSchedule::parse("0 0 0 * * 7");
    assert!(result.is_err());
}

#[test]
fn test_parse_invalid_value() {
    let result = CronSchedule::parse("abc * * * * *");
    assert!(result.is_err());
}

#[test]
fn test_parse_zero_step() {
    let result = CronSchedule::parse("*/0 * * * * *");
    assert!(result.is_err());
}

#[test]
fn test_parse_inverted_range() {
    let result = CronSchedule::parse("0 0 17-9 * * *");
    assert!(result.is_err());
}

// --- next_after tests ---

#[test]
fn test_next_after_every_second() {
    let cron = CronSchedule::parse("* * * * * *").unwrap();
    let base = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
    let next = cron.next_after(base).unwrap();
    assert_eq!(next, Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 1).unwrap());
}

#[test]
fn test_next_after_every_minute() {
    let cron = CronSchedule::parse("0 * * * * *").unwrap();
    let base = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 30).unwrap();
    let next = cron.next_after(base).unwrap();
    assert_eq!(next, Utc.with_ymd_and_hms(2025, 1, 1, 0, 1, 0).unwrap());
}

#[test]
fn test_next_after_specific_time() {
    // Fire at 14:30:00 every day
    let cron = CronSchedule::parse("0 30 14 * * *").unwrap();
    let base = Utc.with_ymd_and_hms(2025, 3, 15, 10, 0, 0).unwrap();
    let next = cron.next_after(base).unwrap();
    assert_eq!(next, Utc.with_ymd_and_hms(2025, 3, 15, 14, 30, 0).unwrap());
}

#[test]
fn test_next_after_specific_time_already_passed() {
    // Fire at 14:30:00 every day, but we're already past it today
    let cron = CronSchedule::parse("0 30 14 * * *").unwrap();
    let base = Utc.with_ymd_and_hms(2025, 3, 15, 15, 0, 0).unwrap();
    let next = cron.next_after(base).unwrap();
    assert_eq!(next, Utc.with_ymd_and_hms(2025, 3, 16, 14, 30, 0).unwrap());
}

#[test]
fn test_next_after_step_seconds() {
    // Every 15 seconds
    let cron = CronSchedule::parse("*/15 * * * * *").unwrap();
    let base = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 10).unwrap();
    let next = cron.next_after(base).unwrap();
    assert_eq!(next, Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 15).unwrap());
}

#[test]
fn test_next_after_step_minutes() {
    // Every 5 minutes at second 0
    let cron = CronSchedule::parse("0 */5 * * * *").unwrap();
    let base = Utc.with_ymd_and_hms(2025, 1, 1, 0, 7, 0).unwrap();
    let next = cron.next_after(base).unwrap();
    assert_eq!(next, Utc.with_ymd_and_hms(2025, 1, 1, 0, 10, 0).unwrap());
}

#[test]
fn test_next_after_range_hours() {
    // Every hour from 9-17 at minute 0, second 0
    let cron = CronSchedule::parse("0 0 9-17 * * *").unwrap();
    let base = Utc.with_ymd_and_hms(2025, 1, 1, 8, 0, 0).unwrap();
    let next = cron.next_after(base).unwrap();
    assert_eq!(next, Utc.with_ymd_and_hms(2025, 1, 1, 9, 0, 0).unwrap());
}

#[test]
fn test_next_after_range_hours_within() {
    // Within the 9-17 range, should go to next hour
    let cron = CronSchedule::parse("0 0 9-17 * * *").unwrap();
    let base = Utc.with_ymd_and_hms(2025, 1, 1, 12, 30, 0).unwrap();
    let next = cron.next_after(base).unwrap();
    assert_eq!(next, Utc.with_ymd_and_hms(2025, 1, 1, 13, 0, 0).unwrap());
}

#[test]
fn test_next_after_range_hours_past_end() {
    // After 17:00, should go to next day at 9:00
    let cron = CronSchedule::parse("0 0 9-17 * * *").unwrap();
    let base = Utc.with_ymd_and_hms(2025, 1, 1, 18, 0, 0).unwrap();
    let next = cron.next_after(base).unwrap();
    assert_eq!(next, Utc.with_ymd_and_hms(2025, 1, 2, 9, 0, 0).unwrap());
}

#[test]
fn test_next_after_weekday_only() {
    // Monday through Friday (1-5), at midnight
    let cron = CronSchedule::parse("0 0 0 * * 1-5").unwrap();
    // 2025-01-04 is Saturday (dow=6)
    let base = Utc.with_ymd_and_hms(2025, 1, 4, 0, 0, 0).unwrap();
    let next = cron.next_after(base).unwrap();
    // Should skip to Monday 2025-01-06
    assert_eq!(next, Utc.with_ymd_and_hms(2025, 1, 6, 0, 0, 0).unwrap());
}

#[test]
fn test_next_after_sunday_only() {
    // Sunday only (0)
    let cron = CronSchedule::parse("0 0 12 * * 0").unwrap();
    // 2025-01-06 is Monday
    let base = Utc.with_ymd_and_hms(2025, 1, 6, 0, 0, 0).unwrap();
    let next = cron.next_after(base).unwrap();
    // Next Sunday is 2025-01-12
    assert_eq!(next, Utc.with_ymd_and_hms(2025, 1, 12, 12, 0, 0).unwrap());
}

#[test]
fn test_next_after_monthly_first_day() {
    // First of every month at midnight
    let cron = CronSchedule::parse("0 0 0 1 * *").unwrap();
    let base = Utc.with_ymd_and_hms(2025, 3, 15, 0, 0, 0).unwrap();
    let next = cron.next_after(base).unwrap();
    assert_eq!(next, Utc.with_ymd_and_hms(2025, 4, 1, 0, 0, 0).unwrap());
}

#[test]
fn test_next_after_specific_month_and_day() {
    // June 15th at noon
    let cron = CronSchedule::parse("0 0 12 15 6 *").unwrap();
    let base = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
    let next = cron.next_after(base).unwrap();
    assert_eq!(next, Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap());
}

#[test]
fn test_next_after_year_rollover() {
    // December 31 already passed, next fire is Jan 1
    let cron = CronSchedule::parse("0 0 0 1 1 *").unwrap();
    let base = Utc.with_ymd_and_hms(2025, 6, 1, 0, 0, 0).unwrap();
    let next = cron.next_after(base).unwrap();
    assert_eq!(next, Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap());
}

#[test]
fn test_next_after_leap_year_feb29() {
    // 29th of February -- should find 2028 (next leap year after 2025)
    let cron = CronSchedule::parse("0 0 0 29 2 *").unwrap();
    let base = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
    let next = cron.next_after(base).unwrap();
    assert_eq!(next, Utc.with_ymd_and_hms(2028, 2, 29, 0, 0, 0).unwrap());
}

#[test]
fn test_next_after_end_of_day_rollover() {
    // At 23:59:59, every-second cron should roll to next day
    let cron = CronSchedule::parse("* * * * * *").unwrap();
    let base = Utc.with_ymd_and_hms(2025, 1, 1, 23, 59, 59).unwrap();
    let next = cron.next_after(base).unwrap();
    assert_eq!(next, Utc.with_ymd_and_hms(2025, 1, 2, 0, 0, 0).unwrap());
}

#[test]
fn test_next_after_comma_values() {
    // Fire at minutes 0, 15, 30, 45
    let cron = CronSchedule::parse("0 0,15,30,45 * * * *").unwrap();
    let base = Utc.with_ymd_and_hms(2025, 1, 1, 0, 10, 0).unwrap();
    let next = cron.next_after(base).unwrap();
    assert_eq!(next, Utc.with_ymd_and_hms(2025, 1, 1, 0, 15, 0).unwrap());
}
