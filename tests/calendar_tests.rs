use chrono::{Datelike, NaiveDate, Weekday};
use kronforce::scheduler::{last_weekday_of_month, nth_weekday_of_month, parse_weekday};

#[test]
fn test_parse_weekday() {
    assert_eq!(parse_weekday("monday"), Weekday::Mon);
    assert_eq!(parse_weekday("fri"), Weekday::Fri);
    assert_eq!(parse_weekday("SUNDAY"), Weekday::Sun);
    assert_eq!(parse_weekday("bogus"), Weekday::Mon); // default
}

#[test]
fn test_nth_weekday_first_monday_april_2026() {
    // April 2026: starts on Wednesday
    let d = nth_weekday_of_month(2026, 4, Weekday::Mon, 1).unwrap();
    assert_eq!(d, NaiveDate::from_ymd_opt(2026, 4, 6).unwrap());
}

#[test]
fn test_nth_weekday_second_tuesday_jan_2026() {
    // Jan 2026: starts on Thursday
    let d = nth_weekday_of_month(2026, 1, Weekday::Tue, 2).unwrap();
    assert_eq!(d, NaiveDate::from_ymd_opt(2026, 1, 13).unwrap());
}

#[test]
fn test_nth_weekday_fifth_monday_overflows() {
    // April 2026 has only 4 Mondays — 5th doesn't exist
    assert!(nth_weekday_of_month(2026, 4, Weekday::Mon, 5).is_none());
}

#[test]
fn test_last_weekday_friday_april_2026() {
    // April 2026: 30 days, April 30 is Thursday → last Friday is April 24
    let d = last_weekday_of_month(2026, 4, Weekday::Fri).unwrap();
    assert_eq!(d, NaiveDate::from_ymd_opt(2026, 4, 24).unwrap());
}

#[test]
fn test_last_weekday_sunday_feb_2026() {
    // Feb 2026: 28 days, Feb 28 is Saturday → last Sunday is Feb 22
    let d = last_weekday_of_month(2026, 2, Weekday::Sun).unwrap();
    assert_eq!(d, NaiveDate::from_ymd_opt(2026, 2, 22).unwrap());
}

#[test]
fn test_last_day_of_month() {
    // Use the same logic as the scheduler: last day = first of next month - 1
    let last_apr = NaiveDate::from_ymd_opt(2026, 5, 1)
        .unwrap()
        .pred_opt()
        .unwrap();
    assert_eq!(last_apr, NaiveDate::from_ymd_opt(2026, 4, 30).unwrap());

    let last_feb = NaiveDate::from_ymd_opt(2026, 3, 1)
        .unwrap()
        .pred_opt()
        .unwrap();
    assert_eq!(last_feb, NaiveDate::from_ymd_opt(2026, 2, 28).unwrap());

    // Leap year 2024
    let last_feb_leap = NaiveDate::from_ymd_opt(2024, 3, 1)
        .unwrap()
        .pred_opt()
        .unwrap();
    assert_eq!(last_feb_leap, NaiveDate::from_ymd_opt(2024, 2, 29).unwrap());
}

#[test]
fn test_last_day_offset() {
    // Last day of April 2026 - 2 days = April 28
    let last = NaiveDate::from_ymd_opt(2026, 5, 1)
        .unwrap()
        .pred_opt()
        .unwrap();
    let target = last + chrono::Duration::days(-2);
    assert_eq!(target, NaiveDate::from_ymd_opt(2026, 4, 28).unwrap());
}

#[test]
fn test_skip_weekends() {
    // April 25, 2026 is Saturday
    let d = NaiveDate::from_ymd_opt(2026, 4, 25).unwrap();
    assert_eq!(d.weekday(), Weekday::Sat);

    // April 26, 2026 is Sunday
    let d2 = NaiveDate::from_ymd_opt(2026, 4, 26).unwrap();
    assert_eq!(d2.weekday(), Weekday::Sun);

    // April 27, 2026 is Monday (not a weekend)
    let d3 = NaiveDate::from_ymd_opt(2026, 4, 27).unwrap();
    assert_eq!(d3.weekday(), Weekday::Mon);
}

#[test]
fn test_holiday_matching() {
    let holidays = ["2026-12-25".to_string(), "2026-01-01".to_string()];
    let target = NaiveDate::from_ymd_opt(2026, 12, 25).unwrap();
    let target_str = target.format("%Y-%m-%d").to_string();
    assert!(holidays.contains(&target_str));

    let non_holiday = NaiveDate::from_ymd_opt(2026, 12, 24).unwrap();
    let non_str = non_holiday.format("%Y-%m-%d").to_string();
    assert!(!holidays.contains(&non_str));
}
