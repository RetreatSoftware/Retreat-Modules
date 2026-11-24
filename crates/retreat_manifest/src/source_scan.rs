//! A small text scanner over a module's Rust source. It finds what the module really registers,
//! so the manifest can be checked against it without compiling anything.
//!
//! Recognized forms (all modules follow them; a new form must be added here with a test):
//! - `register_command("area.verb", ...)`: the first string literal is the command id.
//! - `register_action(InputAction { id: "name", ... })`: the literal after `id:`.
//! - `register_action(helper("name", ...))`: no `id:` field, so the first string literal.
//!
//! Comments are ignored. The scanner understands string literals (with `\` escapes) but not raw
//! strings; module code does not register ids through them.

/// Command ids registered with `register_command`, in source order.
pub fn registered_commands(source: &str) -> Vec<String> {
    let code = strip_comments(source);
    call_bodies(&code, "register_command")
        .iter()
        .filter_map(|body| first_literal(body))
        .collect()
}

/// Input action ids registered with `register_action`, in source order.
pub fn registered_actions(source: &str) -> Vec<String> {
    let code = strip_comments(source);
    call_bodies(&code, "register_action")
        .iter()
        .filter_map(|body| match body.find("id:") {
            Some(position) => first_literal(&body[position + 3..]),
            None => first_literal(body),
        })
        .collect()
}

/// True when the source declares `pub struct <name>` / `pub enum <name>` and
/// `impl Plugin for <name>`.
pub fn declares_plugin(source: &str, type_name: &str) -> bool {
    let code = strip_comments(source);
    let words: Vec<&str> = code
        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|word| !word.is_empty())
        .collect();
    let declared = words
        .windows(3)
        .any(|w| w[0] == "pub" && (w[1] == "struct" || w[1] == "enum") && w[2] == type_name);
    let implemented = words
        .windows(4)
        .any(|w| w[0] == "impl" && w[1] == "Plugin" && w[2] == "for" && w[3] == type_name);
    declared && implemented
}

/// Replaces `//` and `/* */` comments with spaces (string literals are left untouched).
fn strip_comments(source: &str) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut out = String::with_capacity(source.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '"' {
            out.push(c);
            i += 1;
            while i < chars.len() {
                out.push(chars[i]);
                if chars[i] == '\\' && i + 1 < chars.len() {
                    out.push(chars[i + 1]);
                    i += 2;
                    continue;
                }
                i += 1;
                if chars[i - 1] == '"' {
                    break;
                }
            }
        } else if c == '/' && chars.get(i + 1) == Some(&'/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && chars.get(i + 1) == Some(&'*') {
            i += 2;
            while i < chars.len() && !(chars[i] == '*' && chars.get(i + 1) == Some(&'/')) {
                i += 1;
            }
            i += 2;
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

/// The text between the parentheses of every call `name(...)` that is not a definition
/// (`fn name(`) or a longer identifier.
fn call_bodies(code: &str, name: &str) -> Vec<String> {
    let chars: Vec<char> = code.chars().collect();
    let pattern: Vec<char> = name.chars().collect();
    let mut bodies = Vec::new();
    let mut i = 0;
    while i + pattern.len() < chars.len() {
        let matches_name = chars[i..i + pattern.len()] == pattern[..]
            && chars[i + pattern.len()] == '('
            && (i == 0 || !(chars[i - 1].is_alphanumeric() || chars[i - 1] == '_'));
        if matches_name && !is_definition(&chars, i) {
            let start = i + pattern.len() + 1;
            let end = matching_paren(&chars, start);
            bodies.push(chars[start..end].iter().collect());
            i = start;
        } else {
            i += 1;
        }
    }
    bodies
}

fn is_definition(chars: &[char], name_start: usize) -> bool {
    let before: String = chars[..name_start].iter().collect();
    before.trim_end().ends_with("fn")
}

/// Index of the `)` that closes the call whose body starts at `start`.
fn matching_paren(chars: &[char], start: usize) -> usize {
    let mut depth = 1;
    let mut i = start;
    while i < chars.len() {
        match chars[i] {
            '"' => {
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' {
                        i += 1;
                    }
                    i += 1;
                }
            }
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return i;
                }
            }
            _ => {}
        }
        i += 1;
    }
    chars.len()
}

/// The contents of the first string literal in `text`.
fn first_literal(text: &str) -> Option<String> {
    let start = text.find('"')? + 1;
    let mut value = String::new();
    let mut escaped = false;
    for c in text[start..].chars() {
        if escaped {
            value.push(c);
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == '"' {
            return Some(value);
        } else {
            value.push(c);
        }
    }
    None
}
