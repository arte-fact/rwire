use std::io::{self, Write};
use std::thread::sleep;
use std::time::Duration;

use crate::assets::text;

pub fn print_at(row: usize, col: usize, text: &str) {
    print!("\x1B[{};{}H{}", row + 1, col + 1, text);
    io::stdout().flush().unwrap();
}

pub fn clear_screen() {
    print!("\x1B[2J");
    io::stdout().flush().unwrap();
}

pub fn read_line() -> String {
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

pub fn alternate_screen_buffer() {
    print!("\x1B[?1049h");
    io::stdout().flush().unwrap();
}

pub fn move_cursor_to(row: usize, col: usize) {
    print!("\x1B[{};{}H", row + 1, col + 1);
    io::stdout().flush().unwrap();
}

pub fn black_on_gray(text: &str) -> String {
    format!("\x1B[100m{}\x1B[0m", text)
}

pub fn bottom_prompt(text: &str) -> String {
    print_at(21, 0, &" ".repeat(80));
    print_at(21, 0, text);
    print_at(22, 0, &" ".repeat(80));
    move_cursor_to(22, 0);
    read_line()
}

pub fn bottom_press_enter() {
    bottom_prompt(" ↵");
}

/// 1000 => 1,000
pub fn large_number(number: i32) -> String {
    let digits: Vec<char> = number.abs().to_string().chars().collect();
    let mut result = String::new();
    for (i, c) in digits.iter().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(*c);
    }
    if number < 0 {
        result.insert(0, '-');
    }
    result
}

pub fn bottom_input_number(error: Option<&str>) -> i32 {
    let mut error = error.map(str::to_string);
    loop {
        if let Ok(n) = bottom_prompt(error.as_deref().unwrap_or("")).parse::<i32>() {
            return n;
        }
        error = Some("Veuillez entrer un nombre entier.".to_string());
    }
}

pub fn bottom_input_number_with_range(error: Option<&str>, min: i32, max: i32) -> i32 {
    let mut error = error.map(str::to_string);
    loop {
        let n = bottom_input_number(error.as_deref());
        if (min..=max).contains(&n) {
            return n;
        }
        error = Some(format!(
            "Veuillez entrer une valeur entre {} et {}.",
            min, max
        ));
    }
}

/// Like [`bottom_input_number_with_range`] but an empty line (↵) yields `None`.
pub fn bottom_input_number_with_range_or_enter(
    error: Option<&str>,
    min: i32,
    max: i32,
) -> Option<i32> {
    let input = bottom_prompt(error.unwrap_or(""));
    if input.is_empty() {
        return None;
    }
    match input.parse::<i32>() {
        Ok(n) if (min..=max).contains(&n) => Some(n),
        _ => Some(bottom_input_number_with_range(
            Some(&format!(
                "Veuillez entrer une valeur entre {} et {}.",
                min, max
            )),
            min,
            max,
        )),
    }
}

pub fn input_number_with_range_at(
    row: usize,
    col: usize,
    min: usize,
    max: usize,
    err: Option<&str>,
) -> Option<usize> {
    if let Some(err) = err {
        print_at(row, col, &" ".repeat(80 - col));
        print_at(row, col, err);
        sleep(Duration::from_secs(2));
        print_at(row, col, &" ".repeat(80 - col));
    }

    move_cursor_to(row, col);
    let n = read_line();

    if n.is_empty() {
        return None;
    }

    let n = match n.parse::<usize>() {
        Ok(n) => n,
        Err(_) => {
            return input_number_with_range_at(
                row,
                col,
                min,
                max,
                Some(text::PLEASE_ENTER_VALID_NUMBER),
            )
        }
    };

    if n < min || n > max {
        return input_number_with_range_at(
            row,
            col,
            min,
            max,
            Some(&format!(
                "Veuillez entrer un nombre entre {} et {}",
                min, max
            )),
        );
    }

    Some(n)
}

#[cfg(test)]
mod tests {
    use super::large_number;

    #[test]
    fn large_number_groups_thousands() {
        assert_eq!(large_number(0), "0");
        assert_eq!(large_number(999), "999");
        assert_eq!(large_number(1000), "1,000");
        assert_eq!(large_number(1234567), "1,234,567");
        assert_eq!(large_number(-1234), "-1,234");
    }
}
