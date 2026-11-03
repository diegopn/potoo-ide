pub(crate) fn normalize(value: &str) -> Option<String> {
    let (negative, digits) = match value.strip_prefix('-') {
        Some(digits) => (true, digits),
        None => (false, value.strip_prefix('+').unwrap_or(value)),
    };
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let digits = digits.trim_start_matches('0');
    Some(if digits.is_empty() {
        "0".to_owned()
    } else if negative {
        format!("-{digits}")
    } else {
        digits.to_owned()
    })
}

pub(crate) fn negate(value: &str) -> String {
    if value == "0" {
        "0".to_owned()
    } else if let Some(value) = value.strip_prefix('-') {
        value.to_owned()
    } else {
        format!("-{value}")
    }
}

pub(crate) fn compare(left: &str, right: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    let (left_negative, left_magnitude) = split_sign(left);
    let (right_negative, right_magnitude) = split_sign(right);
    match (left_negative, right_negative) {
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        (false, false) => compare_magnitude(left_magnitude, right_magnitude),
        (true, true) => compare_magnitude(left_magnitude, right_magnitude).reverse(),
    }
}

pub(crate) fn add(left: &str, right: &str) -> String {
    let (left_negative, left_magnitude) = split_sign(left);
    let (right_negative, right_magnitude) = split_sign(right);
    if left_negative == right_negative {
        with_sign(
            left_negative,
            add_magnitudes(left_magnitude, right_magnitude),
        )
    } else {
        match compare_magnitude(left_magnitude, right_magnitude) {
            std::cmp::Ordering::Greater => with_sign(
                left_negative,
                subtract_magnitudes(left_magnitude, right_magnitude),
            ),
            std::cmp::Ordering::Less => with_sign(
                right_negative,
                subtract_magnitudes(right_magnitude, left_magnitude),
            ),
            std::cmp::Ordering::Equal => "0".to_owned(),
        }
    }
}

pub(crate) fn subtract(left: &str, right: &str) -> String {
    add(left, &negate(right))
}

pub(crate) fn multiply(left: &str, right: &str) -> String {
    let (left_negative, left_magnitude) = split_sign(left);
    let (right_negative, right_magnitude) = split_sign(right);
    if left_magnitude == "0" || right_magnitude == "0" {
        return "0".to_owned();
    }
    let left_digits = left_magnitude
        .bytes()
        .rev()
        .map(|byte| u32::from(byte - b'0'))
        .collect::<Vec<_>>();
    let right_digits = right_magnitude
        .bytes()
        .rev()
        .map(|byte| u32::from(byte - b'0'))
        .collect::<Vec<_>>();
    let mut result = vec![0_u32; left_digits.len() + right_digits.len()];
    for (left_index, left_digit) in left_digits.iter().enumerate() {
        let mut carry = 0;
        for (right_index, right_digit) in right_digits.iter().enumerate() {
            let index = left_index + right_index;
            let product = result[index] + left_digit * right_digit + carry;
            result[index] = product % 10;
            carry = product / 10;
        }
        let mut index = left_index + right_digits.len();
        while carry > 0 {
            let product = result[index] + carry;
            result[index] = product % 10;
            carry = product / 10;
            index += 1;
        }
    }
    let magnitude = result
        .iter()
        .rev()
        .map(|digit| char::from(b'0' + *digit as u8))
        .collect::<String>();
    let magnitude = normalize(&magnitude).expect("product is a decimal magnitude");
    with_sign(left_negative != right_negative, magnitude)
}

pub(crate) fn div_rem(left: &str, right: &str) -> Option<(String, String)> {
    let (left_negative, left_magnitude) = split_sign(left);
    let (right_negative, right_magnitude) = split_sign(right);
    if right_magnitude == "0" {
        return None;
    }
    let mut remainder = String::from("0");
    let mut quotient = String::with_capacity(left_magnitude.len());
    for digit in left_magnitude.bytes() {
        remainder = append_digit(remainder, digit - b'0');
        let mut quotient_digit = 0;
        while quotient_digit < 9
            && compare_magnitude(&remainder, right_magnitude) != std::cmp::Ordering::Less
        {
            remainder = subtract_magnitudes(&remainder, right_magnitude);
            quotient_digit += 1;
        }
        quotient.push(char::from(b'0' + quotient_digit));
    }
    let quotient = normalize(&if left_negative != right_negative {
        format!("-{quotient}")
    } else {
        quotient
    })?;
    let remainder = normalize(&if left_negative {
        format!("-{remainder}")
    } else {
        remainder
    })?;
    Some((quotient, remainder))
}

fn split_sign(value: &str) -> (bool, &str) {
    value
        .strip_prefix('-')
        .map_or((false, value), |magnitude| (true, magnitude))
}

fn with_sign(negative: bool, magnitude: String) -> String {
    if magnitude == "0" {
        magnitude
    } else if negative {
        format!("-{magnitude}")
    } else {
        magnitude
    }
}

fn compare_magnitude(left: &str, right: &str) -> std::cmp::Ordering {
    left.len().cmp(&right.len()).then_with(|| left.cmp(right))
}

fn add_magnitudes(left: &str, right: &str) -> String {
    let mut result = Vec::with_capacity(left.len().max(right.len()) + 1);
    let mut carry = 0_u8;
    for index in 0..left.len().max(right.len()) {
        let left_digit = left
            .as_bytes()
            .get(left.len().wrapping_sub(index + 1))
            .map_or(0, |byte| byte - b'0');
        let right_digit = right
            .as_bytes()
            .get(right.len().wrapping_sub(index + 1))
            .map_or(0, |byte| byte - b'0');
        let sum = left_digit + right_digit + carry;
        result.push(b'0' + sum % 10);
        carry = sum / 10;
    }
    if carry > 0 {
        result.push(b'0' + carry);
    }
    result.reverse();
    String::from_utf8(result).expect("decimal digits are UTF-8")
}

fn subtract_magnitudes(left: &str, right: &str) -> String {
    let mut result = Vec::with_capacity(left.len());
    let mut borrow = 0_i16;
    for index in 0..left.len() {
        let left_digit = i16::from(left.as_bytes()[left.len() - index - 1] - b'0') - borrow;
        let right_digit = right
            .as_bytes()
            .get(right.len().wrapping_sub(index + 1))
            .map_or(0, |byte| i16::from(byte - b'0'));
        if left_digit < right_digit {
            result.push((left_digit + 10 - right_digit) as u8 + b'0');
            borrow = 1;
        } else {
            result.push((left_digit - right_digit) as u8 + b'0');
            borrow = 0;
        }
    }
    while result.len() > 1 && result.last() == Some(&b'0') {
        result.pop();
    }
    result.reverse();
    String::from_utf8(result).expect("decimal digits are UTF-8")
}

fn append_digit(value: String, digit: u8) -> String {
    if value == "0" {
        if digit == 0 {
            value
        } else {
            char::from(b'0' + digit).to_string()
        }
    } else {
        format!("{value}{}", char::from(b'0' + digit))
    }
}

#[cfg(test)]
mod tests {
    use super::{div_rem, multiply};

    #[test]
    fn multiplication_returns_canonical_decimal_values() {
        assert_eq!(multiply("1", "1"), "1");
        assert_eq!(multiply("2", "3"), "6");
        assert_eq!(multiply("0", "-999"), "0");
        assert_eq!(multiply("-12", "3"), "-36");
        assert_eq!(multiply("123", "456"), "56088");
    }

    #[test]
    fn division_and_remainder_match_signed_integer_arithmetic() {
        for left in -15_i64..=15 {
            for right in -15_i64..=15 {
                let actual = div_rem(&left.to_string(), &right.to_string());
                let expected = if right == 0 {
                    None
                } else {
                    Some(((left / right).to_string(), (left % right).to_string()))
                };
                assert_eq!(actual, expected, "{left} div/mod {right}");
            }
        }
    }
}
