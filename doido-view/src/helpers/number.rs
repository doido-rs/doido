//! Number/currency formatting helpers (Rails `number_with_delimiter`,
//! `number_to_currency`, …).

/// Group the integer part with thousands separators: `1234567` → `"1,234,567"`.
pub fn number_with_delimiter(n: i64) -> String {
    let neg = n < 0;
    let digits = n.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    if neg {
        format!("-{out}")
    } else {
        out
    }
}

/// Format money: `1234.5` → `"$1,234.50"` (2 decimals, `$` unit).
pub fn number_to_currency(amount: f64) -> String {
    number_to_currency_unit(amount, "$")
}

/// Format money with a custom unit prefix.
pub fn number_to_currency_unit(amount: f64, unit: &str) -> String {
    let rounded = (amount * 100.0).round() / 100.0;
    let whole = rounded.trunc() as i64;
    let cents = ((rounded.abs() - whole.unsigned_abs() as f64) * 100.0).round() as u64;
    format!("{unit}{}.{:02}", number_with_delimiter(whole), cents)
}

/// Format a percentage: `number_to_percentage(45.5, 1)` → `"45.5%"`.
pub fn number_to_percentage(value: f64, precision: usize) -> String {
    format!("{value:.precision$}%")
}
