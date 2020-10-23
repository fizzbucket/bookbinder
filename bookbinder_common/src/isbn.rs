/// present a nicely-printed version of an isbn-13
pub fn display_isbn<S: AsRef<str>>(isbn: S, suffix: Option<&str>) -> Result<String, ()> {
    let isbn = isbn.as_ref();

    if !validate_isbn(isbn) {
        return Err(());
    }

    let mut isbn = isbn
        .chars()
        .skip_while(|x| !x.is_ascii_digit())
        .collect::<String>();
    if isbn.starts_with("13 ") {
        let _ignored_prefix = isbn.drain(..3);
    }
    if let Some(suffix) = suffix {
        Ok(format!("ISBN-13: {} ({})", isbn.trim(), suffix))
    } else {
        Ok(format!("ISBN-13: {}", isbn.trim()))
    }
}

fn isbn_to_array<S: AsRef<str>>(isbn: S) -> Result<[u32; 13], ()> {
    let mut arr = [0; 13];
    let isbn_str = isbn.as_ref();

    let mut isbn = isbn_str
        .chars()
        .filter(|c| c.is_ascii_digit())
        .map(|c| c.to_digit(10).unwrap())
        .take(15)
        .collect::<Vec<_>>();

    match isbn.len() {
        13 => (),
        15 => {
            let mut prefix = isbn.drain(0..2);
            let first = prefix.next().unwrap();
            let second = prefix.next().unwrap();
            if !(first == 1 && second == 3) {
                // this could have been a prefix like ISBN-13
                return Err(());
            }
        }
        _ => return Err(()),
    }

    for (i, v) in isbn.into_iter().enumerate() {
        arr[i] = v;
    }
    Ok(arr)
}

/// Validate that a `str` can be interpreted as an isbn-13.
/// A str will be valid whatever its non-digit components
/// so long as it has at least 13 digits and the last digit is the correct checksum,
/// for the first 12 (or first 12 after a preceding `13`)
/// This allows, for example, the validation of prefixed or hyphenated isbns:
/// ```
/// use bookbinder_common::validate_isbn;
/// assert!(validate_isbn("ISBN13 978-1-4920-6766-5"));
/// assert!(validate_isbn("ISBN-13 9781492067665"));
/// assert!(validate_isbn("978-1-4920-6766-5"));
/// assert!(validate_isbn("9781492067665"));
/// assert!(validate_isbn("ISBN13 978-1-4920-6766-5 (epub)"));
/// ```
pub fn validate_isbn<S: AsRef<str>>(isbn: S) -> bool {
    let arr = isbn_to_array(isbn);
    match arr {
        Ok(arr) => validate_isbn_numeric(arr),
        Err(_) => false,
    }
}

fn calculate_check_digit(digits: &[u32]) -> u32 {
    let sum: u32 = digits
        .iter()
        .enumerate()
        .take(12)
        .map(|(i, &d)| d as u32 * (3 - 2 * ((i as u32 + 1) % 2)))
        .sum();
    (10 - (sum % 10)) % 10
}

fn validate_isbn_numeric(isbn: [u32; 13]) -> bool {
    let check_digit = calculate_check_digit(&isbn[0..12]);
    check_digit == isbn[12]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_isbn_to_array() {
        let a = "ISBN13 978-1-4920-6766-5";
        let b = "ISBN13 9781492067665";
        let c = "978-1-4920-6766-5";
        let d = "9781492067665";

        let expected = [9, 7, 8, 1, 4, 9, 2, 0, 6, 7, 6, 6, 5];

        assert_eq!(isbn_to_array(a).unwrap(), expected);
        assert_eq!(isbn_to_array(b).unwrap(), expected);
        assert_eq!(isbn_to_array(c).unwrap(), expected);
        assert_eq!(isbn_to_array(d).unwrap(), expected);
    }

    #[test]
    fn test_validate_isbn() {
        let a = "ISBN13 978-1-4920-6766-5";
        let b = "ISBN-13 9781492067665";
        let c = "978-1-4920-6766-5";
        let d = "9781492067665";

        let e = "978-1-4920-6766-6";

        assert_eq!(validate_isbn(a), true);
        assert_eq!(validate_isbn(b), true);
        assert_eq!(validate_isbn(c), true);
        assert_eq!(validate_isbn(d), true);
        assert_eq!(validate_isbn(e), false);
    }
}
