//! USD amounts as **integer minor units** with fixed scale **k = 15** (1 USD = 10^15 minor).
//! Stored in SQLite as decimal **strings** (no floating point in persistence).

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

/// Exponent k: USD × 10^k = minor integer.
pub const USD_MINOR_EXP: u32 = 15;

/// 10^15 — fits in i128 for arithmetic with USD-sized values.
pub fn usd_scale() -> Decimal {
    Decimal::from(10_i128.pow(USD_MINOR_EXP))
}

/// Convert a USD `Decimal` to minor units (round to nearest minor unit).
pub fn usd_to_minor(usd: Decimal) -> i128 {
    let scaled = (usd * usd_scale()).round();
    scaled.to_i128().unwrap_or_else(|| {
        if scaled.is_sign_positive() {
            i128::MAX
        } else {
            i128::MIN
        }
    })
}

pub fn minor_to_usd(minor: i128) -> Decimal {
    Decimal::from_i128_with_scale(minor, USD_MINOR_EXP)
}

pub fn minor_to_string(minor: i128) -> String {
    format!("{}", minor_to_usd(minor).normalize())
}

/// Serialize minor to SQLite TEXT (signed integer string, no decimals).
pub fn minor_to_db(minor: i128) -> String {
    minor.to_string()
}

pub fn minor_from_db(s: &str) -> Result<i128, std::num::ParseIntError> {
    s.trim().parse::<i128>()
}

/// Convert legacy cent amount to k=15 minor (1 cent = 10^13 minor).
pub fn cents_to_minor(cents: i64) -> i128 {
    (cents as i128).saturating_mul(10_i128.pow(USD_MINOR_EXP - 2))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    #[test]
    fn usd_one_round_trips_through_minor() {
        let usd = Decimal::ONE;
        let m = usd_to_minor(usd);
        assert_eq!(minor_to_usd(m), usd);
        assert_eq!(m, 10_i128.pow(USD_MINOR_EXP));
    }

    #[test]
    fn minor_db_round_trip() {
        let m = 12345_i128;
        let s = minor_to_db(m);
        assert_eq!(minor_from_db(&s).unwrap(), m);
    }

    #[test]
    fn cents_to_minor_one_dollar() {
        assert_eq!(cents_to_minor(100), usd_to_minor(Decimal::ONE));
    }

    #[test]
    fn minor_to_string_normalizes() {
        let m = usd_to_minor(Decimal::from_str("0.01").unwrap());
        assert_eq!(minor_to_string(m), "0.01");
    }
}
