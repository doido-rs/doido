use doido_view::helpers::number::{
    number_to_currency, number_to_percentage, number_with_delimiter,
};

#[test]
fn delimiter_groups_thousands() {
    assert_eq!(number_with_delimiter(1234567), "1,234,567");
    assert_eq!(number_with_delimiter(100), "100");
    assert_eq!(number_with_delimiter(-12345), "-12,345");
    assert_eq!(number_with_delimiter(0), "0");
}

#[test]
fn currency_has_two_decimals_and_delimiter() {
    assert_eq!(number_to_currency(1234.5), "$1,234.50");
    assert_eq!(number_to_currency(9.0), "$9.00");
    assert_eq!(number_to_currency(1000000.0), "$1,000,000.00");
}

#[test]
fn percentage_respects_precision() {
    assert_eq!(number_to_percentage(45.5, 1), "45.5%");
    assert_eq!(number_to_percentage(100.0, 0), "100%");
}
