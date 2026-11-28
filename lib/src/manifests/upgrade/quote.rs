//! Format 1 renders the whole file before it reads it, so `path: {{ user.home_dir }}/bin` worked
//! without quotes. Format 2 reads the file first, where an unquoted `{{` is YAML or TOML syntax,
//! so such values get quotes

/// `text` with quotes around each value that starts with a template, like `key: {{ x }}` or
/// `- {{ x }}` in YAML and `key = {{ x }}` in TOML
pub(super) fn templates(text: &str, toml: bool) -> Result<String, String> {
    let mut quoted = String::with_capacity(text.len());
    for (index, line) in text.split_inclusive('\n').enumerate() {
        let (content, end) = split_line_end(line);
        let line = match unquoted_value(content, toml) {
            Some(start) => {
                let value = content[start..].trim_end();
                let quoted = quote(value).ok_or_else(|| {
                    format!(
                        "line {}: `{value}` has both kinds of quotes or a backslash, add quotes by hand",
                        index + 1
                    )
                })?;
                format!("{}{quoted}{end}", &content[..start])
            }
            None => line.to_string(),
        };
        quoted.push_str(&line);
    }
    Ok(quoted)
}

/// Whether `text` has Tera syntax
pub(super) fn has_template(text: &str) -> bool {
    ["{{", "{%", "{#"].iter().any(|start| text.contains(start))
}

/// `line` without its line break, and the line break
fn split_line_end(line: &str) -> (&str, &str) {
    let content = line.trim_end_matches(['\n', '\r']);
    (content, &line[content.len()..])
}

/// Where the value of `line` starts, if it is a template without quotes
fn unquoted_value(line: &str, toml: bool) -> Option<usize> {
    let start = if toml {
        toml_value_start(line)?
    } else {
        yaml_value_start(line)?
    };
    line[start..].starts_with("{{").then_some(start)
}

/// Where the value of a YAML `key: value` or `- value` line starts
fn yaml_value_start(line: &str) -> Option<usize> {
    let mut start = line.len() - line.trim_start().len();
    while line[start..].starts_with("- ") {
        start += 2;
        start += line[start..].len() - line[start..].trim_start().len();
    }
    if line[start..].starts_with("{{") {
        return Some(start);
    }
    let colon = line[start..].find(": ")?;
    is_plain_key(&line[start..start + colon]).then(|| {
        let value = start + colon + 2;
        value + line[value..].len() - line[value..].trim_start().len()
    })
}

/// Where the value of a TOML `key = value` line starts
fn toml_value_start(line: &str) -> Option<usize> {
    let equals = line.find('=')?;
    is_plain_key(line[..equals].trim()).then(|| {
        let value = equals + 1;
        value + line[value..].len() - line[value..].trim_start().len()
    })
}

/// Whether `key` is a key without quotes, like `path` or `walk_dir`
fn is_plain_key(key: &str) -> bool {
    !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

/// `value` in quotes that keep it as it is, in YAML and TOML alike: single quotes if it has
/// none, else double quotes if it has no double quotes or backslashes, which they read as escapes
fn quote(value: &str) -> Option<String> {
    if !value.contains('\'') {
        Some(format!("'{value}'"))
    } else if !value.contains('"') && !value.contains('\\') {
        Some(format!("\"{value}\""))
    } else {
        None
    }
}
