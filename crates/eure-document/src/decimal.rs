//! Arbitrary-precision decimal number.

use core::cmp::Ordering;
use core::str::FromStr;

use num_bigint::{BigInt, Sign};

use crate::prelude_internal::*;

/// An exact decimal number represented as `mantissa * 10^(-scale)`.
///
/// Unsuffixed fractional literals such as `1.10` or `1e3` are parsed into this type
/// so that no precision is lost before a schema decides how to interpret them.
///
/// The scale written in the source is preserved (`1.10` keeps scale 2), but equality,
/// ordering and hashing are numeric: `1.10 == 1.1`.
#[derive(Debug, Clone)]
pub struct Decimal {
    mantissa: BigInt,
    scale: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DecimalParseError {
    #[error("empty decimal literal")]
    Empty,
    #[error("invalid character `{0}` in decimal literal")]
    InvalidCharacter(char),
    #[error("missing digits in decimal literal")]
    MissingDigits,
    #[error("decimal exponent is out of range")]
    ExponentOutOfRange,
}

impl Decimal {
    /// Constructs an exact decimal. Every `i64` scale is supported.
    pub fn new(mantissa: BigInt, scale: i64) -> Self {
        Self { mantissa, scale }
    }

    pub fn mantissa(&self) -> &BigInt {
        &self.mantissa
    }

    /// Number of digits after the decimal point. Negative means trailing zeros (`1e3` has scale -3).
    pub fn scale(&self) -> i64 {
        self.scale
    }

    pub fn is_zero(&self) -> bool {
        self.mantissa.sign() == Sign::NoSign
    }

    /// Returns the numerically equal value with the smallest representable scale.
    pub fn normalized(&self) -> Self {
        if self.is_zero() {
            return Self::new(BigInt::ZERO, 0);
        }
        let ten = BigInt::from(10u8);
        let mut mantissa = self.mantissa.clone();
        let mut scale = self.scale;
        while scale > i64::MIN && (&mantissa % &ten).sign() == Sign::NoSign {
            mantissa /= &ten;
            scale -= 1;
        }
        Self::new(mantissa, scale)
    }

    /// Returns the value as an integer if it has no fractional part and the
    /// required power of ten fits in `u32`. This explicitly materializes the
    /// integer, so memory usage grows with its number of digits.
    pub fn to_integer(&self) -> Option<BigInt> {
        let normalized = self.normalized();
        if normalized.scale > 0 {
            return None;
        }
        let exp = u32::try_from(normalized.scale.unsigned_abs()).ok()?;
        Some(normalized.mantissa * BigInt::from(10u8).pow(exp))
    }

    /// Returns true if `self` is an exact integer multiple of `divisor`.
    ///
    /// Only zero is a multiple of zero.
    pub fn is_multiple_of(&self, divisor: &Self) -> bool {
        if divisor.is_zero() {
            return self.is_zero();
        }
        if self.is_zero() {
            return true;
        }
        let shift = i128::from(divisor.scale) - i128::from(self.scale);
        let modulus = BigInt::from(divisor.mantissa.magnitude().clone());
        if shift >= 0 {
            // Compute the remainder without allocating 10^shift.
            let power = BigInt::from(10u8).modpow(&BigInt::from(shift), &modulus);
            (&self.mantissa * power % modulus).sign() == Sign::NoSign
        } else {
            // The dividend must contain all the extra factors of ten. This loop is
            // bounded by its mantissa's digits, even for an enormous scale gap.
            let ten = BigInt::from(10u8);
            let mut value = self.mantissa.clone();
            let mut remaining = -shift;
            while remaining > 0 {
                if (&value % &ten).sign() != Sign::NoSign {
                    return false;
                }
                value /= &ten;
                remaining -= 1;
            }
            (value % modulus).sign() == Sign::NoSign
        }
    }

    /// Converts a finite `f64` to the shortest decimal that round-trips to it.
    ///
    /// Integral values get one fractional digit (`100.0`) so they stay fractional literals.
    pub fn from_f64(value: f64) -> Option<Self> {
        if !value.is_finite() {
            return None;
        }
        // `{:e}` yields the shortest round-tripping digits, e.g. `1.5e2`.
        let decimal: Self = alloc::format!("{value:e}")
            .parse()
            .expect("f64 exponent formatting is a valid decimal literal");
        let normalized = decimal.normalized();
        // Keep moderately sized integral values in plain notation (`100.0`, not `1e2`).
        if normalized.scale <= 0 && normalized.scale >= -20 {
            let exp = u32::try_from(1 - normalized.scale).expect("bounded exponent");
            return Some(Self::new(
                normalized.mantissa * BigInt::from(10u8).pow(exp),
                1,
            ));
        }
        Some(normalized)
    }

    /// Converts to `f64` only if no precision is lost.
    pub fn to_f64_exact(&self) -> Option<f64> {
        let value = self.to_f64();
        (Self::from_f64(value).as_ref() == Some(self)).then_some(value)
    }

    /// Converts to the nearest `f64`.
    pub fn to_f64(&self) -> f64 {
        // Rust's float parser is correctly rounded, and `Display` always produces a valid float literal.
        self.to_string()
            .parse()
            .expect("Decimal display is always a valid float literal")
    }

    /// Converts to the nearest `f32`.
    pub fn to_f32(&self) -> f32 {
        self.to_string()
            .parse()
            .expect("Decimal display is always a valid float literal")
    }
}

impl From<BigInt> for Decimal {
    fn from(value: BigInt) -> Self {
        Self::new(value, 0)
    }
}

impl PartialEq for Decimal {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Decimal {}

impl PartialOrd for Decimal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Decimal {
    fn cmp(&self, other: &Self) -> Ordering {
        let sign = self.mantissa.sign().cmp(&other.mantissa.sign());
        if sign != Ordering::Equal {
            return sign;
        }
        if self.is_zero() {
            return Ordering::Equal;
        }
        let a = self.mantissa.magnitude().to_string();
        let b = other.mantissa.magnitude().to_string();
        let a_exponent = a.len() as i128 - i128::from(self.scale);
        let b_exponent = b.len() as i128 - i128::from(other.scale);
        let magnitude = a_exponent.cmp(&b_exponent).then_with(|| {
            // Compare significands with virtual trailing zeros, never 10^scale.
            let width = a.len().max(b.len());
            a.bytes()
                .chain(core::iter::repeat(b'0'))
                .take(width)
                .cmp(b.bytes().chain(core::iter::repeat(b'0')).take(width))
        });
        if self.mantissa.sign() == Sign::Minus {
            magnitude.reverse()
        } else {
            magnitude
        }
    }
}

impl core::hash::Hash for Decimal {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        let normalized = self.normalized();
        normalized.mantissa.hash(state);
        normalized.scale.hash(state);
    }
}

impl FromStr for Decimal {
    type Err = DecimalParseError;

    /// Parses `[-+]?digits[.digits][(e|E)[-+]?digits]`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(DecimalParseError::Empty);
        }
        let (number, exponent) = match s.find(['e', 'E']) {
            Some(pos) => (&s[..pos], Some(&s[pos + 1..])),
            None => (s, None),
        };
        let (negative, number) = match number.as_bytes().first() {
            Some(b'-') => (true, &number[1..]),
            Some(b'+') => (false, &number[1..]),
            _ => (false, number),
        };
        let (int_part, frac_part) = match number.split_once('.') {
            Some((int_part, frac_part)) => (int_part, frac_part),
            None => (number, ""),
        };
        if int_part.is_empty() || (number.contains('.') && frac_part.is_empty()) {
            return Err(DecimalParseError::MissingDigits);
        }
        let mut digits = String::with_capacity(int_part.len() + frac_part.len() + 1);
        if negative {
            digits.push('-');
        }
        for c in int_part.chars().chain(frac_part.chars()) {
            if !c.is_ascii_digit() {
                return Err(DecimalParseError::InvalidCharacter(c));
            }
            digits.push(c);
        }
        let mantissa: BigInt = digits.parse().expect("validated ASCII digits");
        let frac_len =
            i64::try_from(frac_part.len()).map_err(|_| DecimalParseError::ExponentOutOfRange)?;
        let exponent = match exponent {
            Some(exp) => parse_exponent(exp)?,
            None => 0,
        };
        let scale = i128::from(frac_len)
            .checked_sub(exponent)
            .and_then(|scale| i64::try_from(scale).ok())
            .ok_or(DecimalParseError::ExponentOutOfRange)?;
        Ok(Self::new(mantissa, scale))
    }
}

fn parse_exponent(s: &str) -> Result<i128, DecimalParseError> {
    let digits = s.strip_prefix(['-', '+']).unwrap_or(s);
    if digits.is_empty() {
        return Err(DecimalParseError::MissingDigits);
    }
    if let Some(c) = digits.chars().find(|c| !c.is_ascii_digit()) {
        return Err(DecimalParseError::InvalidCharacter(c));
    }
    s.parse().map_err(|_| DecimalParseError::ExponentOutOfRange)
}

impl core::fmt::Display for Decimal {
    /// Always produces a valid Eure float literal that round-trips with the same scale:
    /// `1.10`, `0.005`, `-3.0`, or scientific form `1.2e4` when the scale is zero,
    /// negative, or would otherwise require more than 20 leading fractional zeros.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let digits = self.mantissa.magnitude().to_string();
        if self.mantissa.sign() == Sign::Minus {
            write!(f, "-")?;
        }
        if self.scale <= 0 || i128::from(self.scale) - digits.len() as i128 > 20 {
            // Scientific notation keeps every mantissa digit, so the scale round-trips:
            // mantissa 10 with scale -9 is `1.0e10`.
            let (first, rest) = digits.split_at(1);
            let exponent = (digits.len() as i128 - 1) - i128::from(self.scale);
            if rest.is_empty() {
                return write!(f, "{first}e{exponent}");
            }
            return write!(f, "{first}.{rest}e{exponent}");
        }
        let scale = usize::try_from(self.scale).expect("positive scale fits in usize");
        if digits.len() > scale {
            let (int_part, frac_part) = digits.split_at(digits.len() - scale);
            write!(f, "{int_part}.{frac_part}")
        } else {
            write!(f, "0.")?;
            for _ in digits.len()..scale {
                write!(f, "0")?;
            }
            write!(f, "{digits}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dec(s: &str) -> Decimal {
        s.parse().unwrap()
    }

    #[test]
    fn parse_preserves_scale() {
        assert_eq!(dec("1.10").scale(), 2);
        assert_eq!(dec("1.10").mantissa(), &BigInt::from(110));
        assert_eq!(dec("1e3").scale(), -3);
        assert_eq!(dec("1.5e-2").scale(), 3);
        assert_eq!(dec("-0.5").mantissa(), &BigInt::from(-5));
    }

    #[test]
    fn display_round_trips() {
        for s in [
            "1.10", "0.005", "-3.0", "-0.25", "1.2e4", "1e0", "123.456", "1.0e10", "-5e3", "0e2",
        ] {
            let d = dec(s);
            assert_eq!(d.to_string(), s);
            let reparsed = dec(&d.to_string());
            assert_eq!(reparsed.scale(), d.scale());
        }
        assert_eq!(dec("1.5E+2").to_string(), "1.5e2");
        assert_eq!(dec("12e3").to_string(), "1.2e4");
    }

    #[test]
    fn equality_is_numeric() {
        assert_eq!(dec("1.10"), dec("1.1"));
        assert_eq!(dec("1e2"), dec("100.0"));
        assert_eq!(dec("0.0"), dec("-0e5"));
        assert_ne!(dec("1.1"), dec("1.01"));
        assert!(dec("1.09") < dec("1.1"));
        assert!(dec("-2.0") < dec("-1.5"));
    }

    #[test]
    fn multiple_of() {
        assert!(dec("1.50").is_multiple_of(&dec("0.25")));
        assert!(dec("-0.3").is_multiple_of(&dec("0.1")));
        assert!(dec("1e3").is_multiple_of(&dec("5")));
        assert!(!dec("1.05").is_multiple_of(&dec("0.1")));
        assert!(dec("0.0").is_multiple_of(&dec("0")));
        assert!(!dec("1").is_multiple_of(&dec("0.0")));
    }

    #[test]
    fn conversions() {
        assert_eq!(dec("0.1").to_f64(), 0.1);
        assert_eq!(dec("1.5e2").to_f64(), 150.0);
        assert_eq!(dec("1.50e2").to_integer(), Some(BigInt::from(150)));
        assert_eq!(dec("1.5").to_integer(), None);
    }

    #[test]
    fn from_f64_is_shortest() {
        assert_eq!(Decimal::from_f64(0.1).unwrap().to_string(), "0.1");
        assert_eq!(Decimal::from_f64(100.0).unwrap().to_string(), "100.0");
        assert_eq!(Decimal::from_f64(-1.5).unwrap().to_string(), "-1.5");
        assert_eq!(Decimal::from_f64(1e300).unwrap().to_string(), "1e300");
        assert_eq!(Decimal::from_f64(f64::NAN), None);
    }

    #[test]
    fn exact_f64_conversion() {
        assert_eq!(dec("0.1").to_f64_exact(), Some(0.1));
        assert_eq!(dec("1.10").to_f64_exact(), Some(1.1));
        assert_eq!(dec("0.10000000000000000000001").to_f64_exact(), None);
    }

    #[test]
    fn extreme_scales_round_trip_and_compare() {
        for scale in [i64::MIN, i64::MAX] {
            for mantissa in [-100, -1, 0, 1, 100] {
                let d = Decimal::new(mantissa.into(), scale);
                let rendered = d.to_string();
                assert!(rendered.len() < 32);
                let parsed = dec(&rendered);
                assert_eq!(parsed.scale(), scale);
                assert_eq!(parsed.mantissa(), d.mantissa());
                assert_eq!(d, d.normalized());
                assert_eq!(d.cmp(&dec("0")), mantissa.cmp(&0));
                assert!(!d.to_f64().is_nan());
            }
        }
        let large = Decimal::new(1.into(), i64::MIN);
        let tiny = Decimal::new(1.into(), i64::MAX);
        assert!(large > tiny);
        assert!(large.is_multiple_of(&tiny));
        assert!(!tiny.is_multiple_of(&large));
        assert!(!large.is_multiple_of(&Decimal::new(3.into(), i64::MAX)));
        assert_eq!(large.to_integer(), None);
        assert_eq!(tiny.to_f64(), 0.0);
        assert_eq!(large.to_f64(), f64::INFINITY);
    }

    #[test]
    fn large_exponents_do_not_expand() {
        let tiny = dec("1.00e-1000000");
        assert_eq!(tiny.to_string(), "1.00e-1000000");
        assert_eq!(tiny.to_f64(), 0.0);
        assert_eq!(tiny.to_f32(), 0.0);
        assert_eq!(tiny.to_f64_exact(), None);
        assert!(tiny > dec("0"));
        assert!(tiny < dec("1"));
        assert!(dec("1e1000000").is_multiple_of(&dec("0.25")));
        assert!(!tiny.is_multiple_of(&dec("0.25")));
    }

    #[test]
    fn normalized_boundary_hashes_match() {
        use core::hash::BuildHasher;
        let a = Decimal::new(10.into(), i64::MIN);
        let b = Decimal::new(100.into(), i64::MIN + 1);
        assert_eq!(a, b);
        let hash = |d: &Decimal| {
            let state = foldhash::fast::FixedState::with_seed(0);
            state.hash_one(d)
        };
        assert_eq!(hash(&a), hash(&b));
    }

    proptest::proptest! {
        #[test]
        fn comparison_and_multiples_match_integer_arithmetic(
            a in -100000i64..100000,
            b in -100000i64..100000,
            a_scale in -8i64..8,
            b_scale in -8i64..8,
        ) {
            let left = Decimal::new(a.into(), a_scale);
            let right = Decimal::new(b.into(), b_scale);
            let common = a_scale.max(b_scale);
            let a = i128::from(a) * 10i128.pow((common - a_scale) as u32);
            let b = i128::from(b) * 10i128.pow((common - b_scale) as u32);
            proptest::prop_assert_eq!(left.cmp(&right), a.cmp(&b));
            proptest::prop_assert_eq!(left.is_multiple_of(&right), if b == 0 { a == 0 } else { a % b == 0 });
        }
    }

    #[test]
    fn parse_errors() {
        assert_eq!("".parse::<Decimal>(), Err(DecimalParseError::Empty));
        assert_eq!(
            "1.".parse::<Decimal>(),
            Err(DecimalParseError::MissingDigits)
        );
        assert_eq!(
            ".5".parse::<Decimal>(),
            Err(DecimalParseError::MissingDigits)
        );
        assert_eq!(
            "1e".parse::<Decimal>(),
            Err(DecimalParseError::MissingDigits)
        );
        assert_eq!(
            "1_0.5".parse::<Decimal>(),
            Err(DecimalParseError::InvalidCharacter('_'))
        );
    }
}
