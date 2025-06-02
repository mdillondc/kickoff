use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(f64),
    Plus,
    Minus,
    Multiply,
    Divide,
    LeftParen,
    RightParen,
}

pub fn is_math_expression(input: &str) -> bool {
    let input = input.trim();
    if input.is_empty() {
        return false;
    }
    
    // Check if it contains any numbers
    let has_number = input.chars().any(|c| c.is_ascii_digit() || c == '.');
    
    // Must have at least one number
    if !has_number {
        return false;
    }
    
    // Try to actually evaluate the expression - if it fails, it's not valid
    evaluate(input).is_ok()
}

pub fn evaluate(input: &str) -> Result<f64, String> {
    let tokens = tokenize(input)?;
    if tokens.is_empty() {
        return Err("Empty expression".to_string());
    }
    
    let mut tokens = VecDeque::from(tokens);
    let result = parse_expression(&mut tokens)?;
    
    if !tokens.is_empty() {
        return Err("Unexpected tokens at end of expression".to_string());
    }
    
    Ok(result)
}

fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    
    while let Some(&ch) = chars.peek() {
        match ch {
            ' ' => {
                chars.next();
            }
            '+' => {
                tokens.push(Token::Plus);
                chars.next();
            }
            '-' => {
                chars.next();
                // Check if this is a negative number
                if tokens.is_empty() || matches!(tokens.last(), Some(Token::LeftParen | Token::Plus | Token::Minus | Token::Multiply | Token::Divide)) {
                    if let Some(&next_ch) = chars.peek() {
                        if next_ch.is_ascii_digit() || next_ch == '.' {
                            let num = parse_number(&mut chars, true)?;
                            tokens.push(Token::Number(num));
                            continue;
                        }
                    }
                }
                tokens.push(Token::Minus);
            }
            '*' => {
                tokens.push(Token::Multiply);
                chars.next();
            }
            '/' => {
                tokens.push(Token::Divide);
                chars.next();
            }
            '(' => {
                tokens.push(Token::LeftParen);
                chars.next();
            }
            ')' => {
                tokens.push(Token::RightParen);
                chars.next();
            }
            c if c.is_ascii_digit() || c == '.' => {
                let num = parse_number(&mut chars, false)?;
                tokens.push(Token::Number(num));
            }
            _ => {
                return Err(format!("Unexpected character: {}", ch));
            }
        }
    }
    
    Ok(tokens)
}

fn parse_number(chars: &mut std::iter::Peekable<std::str::Chars>, negative: bool) -> Result<f64, String> {
    let mut number_str = String::new();
    
    if negative {
        number_str.push('-');
    }
    
    let mut has_dot = false;
    
    while let Some(&ch) = chars.peek() {
        if ch.is_ascii_digit() {
            number_str.push(ch);
            chars.next();
        } else if ch == '.' && !has_dot {
            has_dot = true;
            number_str.push(ch);
            chars.next();
        } else {
            break;
        }
    }
    
    number_str.parse::<f64>().map_err(|_| format!("Invalid number: {}", number_str))
}

fn parse_expression(tokens: &mut VecDeque<Token>) -> Result<f64, String> {
    parse_addition(tokens)
}

fn parse_addition(tokens: &mut VecDeque<Token>) -> Result<f64, String> {
    let mut left = parse_multiplication(tokens)?;
    
    while let Some(token) = tokens.front() {
        match token {
            Token::Plus => {
                tokens.pop_front();
                let right = parse_multiplication(tokens)?;
                left += right;
            }
            Token::Minus => {
                tokens.pop_front();
                let right = parse_multiplication(tokens)?;
                left -= right;
            }
            _ => break,
        }
    }
    
    Ok(left)
}

fn parse_multiplication(tokens: &mut VecDeque<Token>) -> Result<f64, String> {
    let mut left = parse_factor(tokens)?;
    
    while let Some(token) = tokens.front() {
        match token {
            Token::Multiply => {
                tokens.pop_front();
                let right = parse_factor(tokens)?;
                left *= right;
            }
            Token::Divide => {
                tokens.pop_front();
                let right = parse_factor(tokens)?;
                if right == 0.0 {
                    return Err("Division by zero".to_string());
                }
                left /= right;
            }
            _ => break,
        }
    }
    
    Ok(left)
}

fn parse_factor(tokens: &mut VecDeque<Token>) -> Result<f64, String> {
    match tokens.pop_front() {
        Some(Token::Number(n)) => Ok(n),
        Some(Token::LeftParen) => {
            let result = parse_expression(tokens)?;
            match tokens.pop_front() {
                Some(Token::RightParen) => Ok(result),
                _ => Err("Missing closing parenthesis".to_string()),
            }
        }
        Some(Token::Minus) => {
            let factor = parse_factor(tokens)?;
            Ok(-factor)
        }
        Some(Token::Plus) => {
            parse_factor(tokens)
        }
        _ => Err("Expected number or opening parenthesis".to_string()),
    }
}

pub fn format_result(result: f64) -> String {
    if result.fract() == 0.0 && result.abs() < 1e15 {
        format!("{}", result as i64)
    } else if result.abs() >= 1e15 {
        format!("{:e}", result)
    } else {
        format!("{}", result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_math_expression() {
        // Basic arithmetic
        assert!(is_math_expression("10-5"));
        assert!(is_math_expression("2+3*4"));
        assert!(is_math_expression("(1+2)*3"));
        assert!(is_math_expression("42"));
        assert!(is_math_expression("-5"));
        
        // With spaces
        assert!(is_math_expression("10 - 5"));
        assert!(is_math_expression("2 + 3 * 4"));
        
        // Decimals
        assert!(is_math_expression("3.14*2"));
        assert!(is_math_expression("10.5/2.5"));
        
        // Complex expressions
        assert!(is_math_expression("((1+2)*3)/4"));
        assert!(is_math_expression("-5.5+10"));
        
        // Not math expressions
        assert!(!is_math_expression("hello"));
        assert!(!is_math_expression(""));
        assert!(!is_math_expression("abc123"));
        assert!(!is_math_expression("++"));
        assert!(!is_math_expression("--"));
    }

    #[test]
    fn test_evaluate() {
        // Basic operations
        assert_eq!(evaluate("10-5").unwrap(), 5.0);
        assert_eq!(evaluate("2+3*4").unwrap(), 14.0);
        assert_eq!(evaluate("(1+2)*3").unwrap(), 9.0);
        assert_eq!(evaluate("42").unwrap(), 42.0);
        assert_eq!(evaluate("-5").unwrap(), -5.0);
        assert_eq!(evaluate("10/2").unwrap(), 5.0);
        
        // Order of operations
        assert_eq!(evaluate("2+3*4").unwrap(), 14.0);
        assert_eq!(evaluate("(2+3)*4").unwrap(), 20.0);
        assert_eq!(evaluate("10-6/2").unwrap(), 7.0);
        
        // Decimals
        assert_eq!(evaluate("3.5*2").unwrap(), 7.0);
        assert_eq!(evaluate("10.5/2.1").unwrap(), 5.0);
        
        // Negative numbers
        assert_eq!(evaluate("-5+10").unwrap(), 5.0);
        assert_eq!(evaluate("(-2)*3").unwrap(), -6.0);
        assert_eq!(evaluate("10-(-5)").unwrap(), 15.0);
        
        // Complex expressions
        assert_eq!(evaluate("((1+2)*3)/4").unwrap(), 2.25);
        assert_eq!(evaluate("2*3+4*5").unwrap(), 26.0);
        
        // With spaces
        assert_eq!(evaluate("10 - 5").unwrap(), 5.0);
        assert_eq!(evaluate("2 + 3 * 4").unwrap(), 14.0);
    }

    #[test]
    fn test_evaluate_errors() {
        // Division by zero
        assert!(evaluate("5/0").is_err());
        assert!(evaluate("10/(5-5)").is_err());
        
        // Invalid expressions
        assert!(evaluate("").is_err());
        assert!(evaluate("++").is_err());
        assert!(evaluate("5+").is_err());
        assert!(evaluate("*5").is_err());
        assert!(evaluate("(5").is_err());
        assert!(evaluate("5)").is_err());
        assert!(evaluate("5..5").is_err());
    }

    #[test]
    fn test_format_result() {
        // Integers
        assert_eq!(format_result(5.0), "5");
        assert_eq!(format_result(-5.0), "-5");
        assert_eq!(format_result(0.0), "0");
        assert_eq!(format_result(42.0), "42");
        
        // Decimals
        assert_eq!(format_result(5.5), "5.5");
        assert_eq!(format_result(3.14159), "3.14159");
        assert_eq!(format_result(-2.5), "-2.5");
        
        // Very large numbers (should use scientific notation)
        assert_eq!(format_result(1e16), "1e16");
        assert_eq!(format_result(1e17), "1e17");
    }
}