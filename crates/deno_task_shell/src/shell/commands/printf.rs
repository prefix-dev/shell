use futures::future::LocalBoxFuture;

use crate::shell::types::ExecuteResult;

use super::ShellCommand;
use super::ShellCommandContext;

pub struct PrintfCommand;

impl ShellCommand for PrintfCommand {
    fn execute(
        &self,
        mut context: ShellCommandContext,
    ) -> LocalBoxFuture<'static, ExecuteResult> {
        let result = execute_printf(&context.args, &mut context.stdout);
        Box::pin(futures::future::ready(result))
    }
}

fn execute_printf(
    args: &[String],
    stdout: &mut crate::shell::types::ShellPipeWriter,
) -> ExecuteResult {
    if args.is_empty() {
        return ExecuteResult::Continue(1, Vec::new(), Vec::new());
    }

    let format = &args[0];
    let params = &args[1..];
    let mut param_idx = 0;

    let output = format_string(format, params, &mut param_idx);
    let _ = stdout.write_all(output.as_bytes());

    ExecuteResult::Continue(0, Vec::new(), Vec::new())
}

fn format_string(format: &str, params: &[String], param_idx: &mut usize) -> String {
    let mut result = String::new();
    let mut chars = format.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            // Handle escape sequences
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some('r') => result.push('\r'),
                Some('\\') => result.push('\\'),
                Some('a') => result.push('\x07'),
                Some('b') => result.push('\x08'),
                Some('f') => result.push('\x0C'),
                Some('v') => result.push('\x0B'),
                Some('0') => {
                    // Octal escape \0NNN
                    let mut octal = String::new();
                    for _ in 0..3 {
                        if let Some(&ch) = chars.peek() {
                            if ch.is_ascii_digit() && ch < '8' {
                                octal.push(ch);
                                chars.next();
                            } else {
                                break;
                            }
                        }
                    }
                    if octal.is_empty() {
                        result.push('\0');
                    } else if let Ok(val) = u8::from_str_radix(&octal, 8) {
                        result.push(val as char);
                    }
                }
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                }
                None => result.push('\\'),
            }
        } else if c == '%' {
            // Handle format specifiers
            match chars.peek() {
                Some('%') => {
                    chars.next();
                    result.push('%');
                }
                Some(_) => {
                    let param = if *param_idx < params.len() {
                        let p = &params[*param_idx];
                        *param_idx += 1;
                        p.as_str()
                    } else {
                        ""
                    };
                    // Parse optional flags, width, precision
                    let mut spec = String::new();
                    // Flags
                    while let Some(&ch) = chars.peek() {
                        if ch == '-' || ch == '+' || ch == ' ' || ch == '0' || ch == '#' {
                            spec.push(ch);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    // Width
                    while let Some(&ch) = chars.peek() {
                        if ch.is_ascii_digit() {
                            spec.push(ch);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    // Precision
                    if let Some(&'.') = chars.peek() {
                        spec.push('.');
                        chars.next();
                        while let Some(&ch) = chars.peek() {
                            if ch.is_ascii_digit() {
                                spec.push(ch);
                                chars.next();
                            } else {
                                break;
                            }
                        }
                    }
                    // Conversion character
                    match chars.next() {
                        Some('s') => {
                            if spec.is_empty() {
                                result.push_str(param);
                            } else {
                                // Handle width/precision for strings
                                let width = parse_width(&spec);
                                let precision = parse_precision(&spec);
                                let s = if let Some(prec) = precision {
                                    &param[..param.len().min(prec)]
                                } else {
                                    param
                                };
                                if let Some(w) = width {
                                    if spec.starts_with('-') {
                                        result.push_str(&format!("{:<width$}", s, width = w));
                                    } else {
                                        result.push_str(&format!("{:>width$}", s, width = w));
                                    }
                                } else {
                                    result.push_str(s);
                                }
                            }
                        }
                        Some('d' | 'i') => {
                            let val: i64 = param.parse().unwrap_or(0);
                            if spec.is_empty() {
                                result.push_str(&val.to_string());
                            } else {
                                let width = parse_width(&spec);
                                let zero_pad = spec.starts_with('0');
                                if let Some(w) = width {
                                    if zero_pad {
                                        result.push_str(&format!("{:0>width$}", val, width = w));
                                    } else if spec.starts_with('-') {
                                        result.push_str(&format!("{:<width$}", val, width = w));
                                    } else {
                                        result.push_str(&format!("{:>width$}", val, width = w));
                                    }
                                } else {
                                    result.push_str(&val.to_string());
                                }
                            }
                        }
                        Some('o') => {
                            let val: i64 = param.parse().unwrap_or(0);
                            result.push_str(&format!("{:o}", val));
                        }
                        Some('x') => {
                            let val: i64 = param.parse().unwrap_or(0);
                            result.push_str(&format!("{:x}", val));
                        }
                        Some('X') => {
                            let val: i64 = param.parse().unwrap_or(0);
                            result.push_str(&format!("{:X}", val));
                        }
                        Some('f') => {
                            let val: f64 = param.parse().unwrap_or(0.0);
                            let precision = parse_precision(&spec).unwrap_or(6);
                            result.push_str(&format!("{:.prec$}", val, prec = precision));
                        }
                        Some('c') => {
                            if let Some(ch) = param.chars().next() {
                                result.push(ch);
                            }
                        }
                        Some('b') => {
                            // %b: interpret backslash escapes in the argument
                            let expanded = expand_backslash_escapes(param);
                            result.push_str(&expanded);
                        }
                        Some(other) => {
                            result.push('%');
                            result.push_str(&spec);
                            result.push(other);
                        }
                        None => {
                            result.push('%');
                            result.push_str(&spec);
                        }
                    }
                }
                None => {
                    result.push('%');
                }
            }
        } else {
            result.push(c);
        }
    }

    result
}

fn parse_width(spec: &str) -> Option<usize> {
    let s = spec.trim_start_matches(&['-', '+', ' ', '0', '#'][..]);
    let s = s.split('.').next().unwrap_or("");
    s.parse().ok()
}

fn parse_precision(spec: &str) -> Option<usize> {
    if let Some(pos) = spec.find('.') {
        spec[pos + 1..].parse().ok()
    } else {
        None
    }
}

fn expand_backslash_escapes(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some('r') => result.push('\r'),
                Some('\\') => result.push('\\'),
                Some('a') => result.push('\x07'),
                Some('b') => result.push('\x08'),
                Some('f') => result.push('\x0C'),
                Some('v') => result.push('\x0B'),
                Some('0') => {
                    let mut octal = String::new();
                    for _ in 0..3 {
                        if let Some(&ch) = chars.peek() {
                            if ch.is_ascii_digit() && ch < '8' {
                                octal.push(ch);
                                chars.next();
                            } else {
                                break;
                            }
                        }
                    }
                    if octal.is_empty() {
                        result.push('\0');
                    } else if let Ok(val) = u8::from_str_radix(&octal, 8) {
                        result.push(val as char);
                    }
                }
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(c);
        }
    }
    result
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_basic_printf() {
        let mut idx = 0;
        assert_eq!(format_string("hello", &[], &mut idx), "hello");
    }

    #[test]
    fn test_printf_string_format() {
        let mut idx = 0;
        assert_eq!(
            format_string("%s world", &["hello".to_string()], &mut idx),
            "hello world"
        );
    }

    #[test]
    fn test_printf_integer_format() {
        let mut idx = 0;
        assert_eq!(
            format_string("%d", &["42".to_string()], &mut idx),
            "42"
        );
    }

    #[test]
    fn test_printf_escape_sequences() {
        let mut idx = 0;
        assert_eq!(format_string("a\\nb", &[], &mut idx), "a\nb");
        idx = 0;
        assert_eq!(format_string("a\\tb", &[], &mut idx), "a\tb");
    }

    #[test]
    fn test_printf_percent_escape() {
        let mut idx = 0;
        assert_eq!(format_string("100%%", &[], &mut idx), "100%");
    }

    #[test]
    fn test_printf_multiple_args() {
        let mut idx = 0;
        assert_eq!(
            format_string(
                "%s is %d",
                &["answer".to_string(), "42".to_string()],
                &mut idx
            ),
            "answer is 42"
        );
    }

    #[test]
    fn test_printf_hex() {
        let mut idx = 0;
        assert_eq!(
            format_string("%x", &["255".to_string()], &mut idx),
            "ff"
        );
    }

    #[test]
    fn test_printf_float() {
        let mut idx = 0;
        assert_eq!(
            format_string("%.2f", &["3.14159".to_string()], &mut idx),
            "3.14"
        );
    }
}
