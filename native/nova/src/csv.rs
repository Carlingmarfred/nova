//! Minimal RFC 4180-style CSV support (N06b).
//!
//! Supported subset, matched to what the stdlib surface needs:
//! - comma-separated fields, `\r\n` / `\n` / `\r` record separators
//! - quoted fields with `""` as an escaped quote
//! - a final record may omit the trailing newline

/// Parses CSV text into records of fields. An empty input yields no records.
pub fn parse(text: &str) -> Result<Vec<Vec<String>>, String> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        if in_quotes {
            if c == '"' {
                match chars.peek() {
                    Some('"') => {
                        chars.next();
                        field.push('"');
                    }
                    _ => in_quotes = false,
                }
            } else {
                field.push(c);
            }
            continue;
        }
        match c {
            '"' => {
                if !field.is_empty() {
                    return Err("unexpected quote inside unquoted field".to_string());
                }
                in_quotes = true;
            }
            ',' => {
                row.push(std::mem::take(&mut field));
            }
            '\n' => {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
            }
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
            }
            other => field.push(other),
        }
    }
    if in_quotes {
        return Err("unterminated quoted field - add the closing quote".to_string());
    }
    // A trailing separator/newline already flushed the row; only flush remains.
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    Ok(rows)
}

/// Serializes records back to CSV, quoting any field that contains a comma,
/// quote or newline. Quotes are escaped by doubling.
pub fn stringify(rows: &[Vec<String>]) -> String {
    let mut out = String::new();
    for (ri, row) in rows.iter().enumerate() {
        if ri > 0 {
            out.push('\n');
        }
        let fields: Vec<String> = row.iter().map(|f| quote_field(f)).collect();
        out.push_str(&fields.join(","));
    }
    out
}

fn quote_field(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') || field.contains('\r') {
        format!("\"{}\"", field.replace("\"", "\"\""))
    } else {
        field.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_rows() {
        assert_eq!(parse("a,b\nc,d").unwrap(), vec![vec!["a", "b"], vec!["c", "d"]]);
    }

    #[test]
    fn quoted_field_with_comma_and_escaped_quote() {
        let rows = parse("name,note\n\"Doe, Jane\",\"said \"\"hi\"\"\"").unwrap();
        assert_eq!(rows[1], vec!["Doe, Jane", "said \"hi\""]);
    }

    #[test]
    fn newline_inside_quotes_is_data() {
        let rows = parse("\"a\nb\",c").unwrap();
        assert_eq!(rows, vec![vec!["a\nb", "c"]]);
    }

    #[test]
    fn crlf_records() {
        assert_eq!(parse("a,b\r\nc,d").unwrap(), vec![vec!["a", "b"], vec!["c", "d"]]);
    }

    #[test]
    fn unterminated_quote_is_error() {
        assert!(parse("a,\"oops").is_err());
    }

    #[test]
    fn empty_input_no_rows() {
        assert_eq!(parse("").unwrap().len(), 0);
    }

    #[test]
    fn stringify_quotes_when_needed() {
        let rows = vec![vec!["plain".into(), "with,comma".into()], vec!["say \"x\"".into()]];
        assert_eq!(
            stringify(&rows),
            "plain,\"with,comma\"\n\"say \"\"x\"\"\""
        );
    }

    #[test]
    fn roundtrip_preserves_fields() {
        let original = vec![
            vec!["a".into(), "b,c".into()],
            vec!["line\nbreak".into(), "\"q\"".into()],
        ];
        let parsed = parse(&stringify(&original)).unwrap();
        assert_eq!(parsed, original);
    }
}
