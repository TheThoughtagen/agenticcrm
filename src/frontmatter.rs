use anyhow::{Context, Result, bail};
use regex::Regex;

/// Parse raw frontmatter from a markdown file, returning (raw_yaml, body).
/// The raw YAML is the verbatim text between `---` delimiters, preserving
/// comments, blank lines, and field order.
pub fn parse_raw_frontmatter(content: &str) -> Result<(String, String)> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        bail!("File does not start with frontmatter delimiter");
    }

    let after_first = &trimmed[3..];
    // Skip the newline after opening ---
    let after_first = after_first.strip_prefix('\n').unwrap_or(after_first);

    let end = after_first
        .find("\n---")
        .context("No closing frontmatter delimiter found")?;

    let raw_yaml = after_first[..end].to_string();
    let body_start = end + 4; // "\n---"
    let rest = &after_first[body_start..];
    // Strip leading newlines from body
    let body = rest.strip_prefix('\n').unwrap_or(rest);
    let body = body.strip_prefix('\n').unwrap_or(body);

    Ok((raw_yaml, body.to_string()))
}

/// Update a scalar field value in raw frontmatter text.
/// Preserves comments, blank lines, and field order.
/// If the field doesn't exist, appends it before the last line.
pub fn update_field(raw_frontmatter: &str, key: &str, new_value: &str) -> String {
    let pattern = format!(r"(?m)^({}:[ \t]*)(.*)$", regex::escape(key));
    let re = Regex::new(&pattern).unwrap();

    if re.is_match(raw_frontmatter) {
        re.replace(raw_frontmatter, |caps: &regex::Captures| {
            let prefix = &caps[1];
            // Ensure there's always a space after the colon
            if prefix.ends_with(' ') || prefix.ends_with('\t') {
                format!("{}{}", prefix, new_value)
            } else {
                format!("{} {}", prefix, new_value)
            }
        })
        .to_string()
    } else {
        // Append before the last line
        let mut lines: Vec<&str> = raw_frontmatter.lines().collect();
        let new_line = format!("{key}: {new_value}");
        if lines.is_empty() {
            return new_line;
        }
        let last = lines.pop().unwrap();
        lines.push(&new_line);
        lines.push(last);
        lines.join("\n")
    }
}

/// Update an array field in raw frontmatter text.
/// Replaces the entire array block (the field line and all subsequent indented `  -` lines).
/// If values is empty, writes `key: []`.
pub fn update_array_field(raw_frontmatter: &str, key: &str, values: &[String]) -> String {
    let lines: Vec<&str> = raw_frontmatter.lines().collect();
    let mut result = Vec::new();
    let field_prefix = format!("{}:", key);
    let mut i = 0;
    let mut found = false;

    while i < lines.len() {
        if lines[i].starts_with(&field_prefix) {
            found = true;
            if values.is_empty() {
                result.push(format!("{key}: []"));
            } else {
                result.push(format!("{key}:"));
                for v in values {
                    result.push(format!("  - \"{v}\""));
                }
            }
            // Skip old array items (lines starting with whitespace + -)
            i += 1;
            while i < lines.len() && lines[i].starts_with("  -") {
                i += 1;
            }
            continue;
        }
        result.push(lines[i].to_string());
        i += 1;
    }

    if !found {
        // Append the field
        if values.is_empty() {
            result.push(format!("{key}: []"));
        } else {
            result.push(format!("{key}:"));
            for v in values {
                result.push(format!("  - \"{v}\""));
            }
        }
    }

    result.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEMPLATE_FRONTMATTER: &str = r#"id: "abc-123"
name: "Jane Smith"
aliases: []
pronouns: ""

# Contact
email: []
phone: []
address: []

# Professional
company: "Acme Corp"
role: "Engineer"
industry: ""
linkedin: ""

# Social
twitter: ""
facebook: ""
instagram: ""
github: ""
website: ""

# Personal
birthday:
interests: []
family: []

# Relationship
how_we_met: ""
met_date:
introduced_by: ""
relationship: acquaintance
tags:
  - "rust"
  - "cli"

# CRM
status: active
follow_up_cadence: ""
last_contacted:
next_follow_up:
priority: medium

# Source
source: manual
source_id: """#;

    #[test]
    fn test_parse_raw_frontmatter() {
        let content = format!("---\n{TEMPLATE_FRONTMATTER}\n---\n\n## Notes\n\nSome notes.\n");
        let (raw, body) = parse_raw_frontmatter(&content).unwrap();
        assert_eq!(raw, TEMPLATE_FRONTMATTER);
        assert!(body.starts_with("## Notes"));
    }

    #[test]
    fn test_parse_raw_frontmatter_preserves_comments() {
        let content = format!("---\n{TEMPLATE_FRONTMATTER}\n---\n\nBody\n");
        let (raw, _) = parse_raw_frontmatter(&content).unwrap();
        assert!(raw.contains("# Contact"));
        assert!(raw.contains("# Professional"));
        assert!(raw.contains("# Social"));
        assert!(raw.contains("# Personal"));
        assert!(raw.contains("# Relationship"));
        assert!(raw.contains("# CRM"));
        assert!(raw.contains("# Source"));
    }

    #[test]
    fn test_update_field_replaces_value() {
        let result = update_field(TEMPLATE_FRONTMATTER, "status", "archived");
        assert!(result.contains("status: archived"));
        // Original value gone
        assert!(!result.contains("status: active"));
    }

    #[test]
    fn test_update_field_preserves_comments() {
        let result = update_field(TEMPLATE_FRONTMATTER, "status", "archived");
        assert!(result.contains("# Contact"));
        assert!(result.contains("# Professional"));
        assert!(result.contains("# CRM"));
    }

    #[test]
    fn test_update_field_preserves_blank_lines_and_order() {
        let result = update_field(TEMPLATE_FRONTMATTER, "company", "NewCo");
        let lines: Vec<&str> = result.lines().collect();
        // Blank line after pronouns should still be there
        assert!(result.contains("pronouns: \"\"\n\n# Contact"));
        // company should come before role
        let company_pos = lines.iter().position(|l| l.starts_with("company:")).unwrap();
        let role_pos = lines.iter().position(|l| l.starts_with("role:")).unwrap();
        assert!(company_pos < role_pos);
    }

    #[test]
    fn test_update_field_ensures_space_after_colon() {
        // Template has "birthday:" with no trailing space — update should add one
        let result = update_field(TEMPLATE_FRONTMATTER, "birthday", "1990-01-15");
        assert!(
            result.contains("birthday: 1990-01-15"),
            "Expected 'birthday: 1990-01-15' but got: {}",
            result.lines().find(|l| l.starts_with("birthday")).unwrap_or("NOT FOUND")
        );
    }

    #[test]
    fn test_update_field_appends_missing_field() {
        let result = update_field(TEMPLATE_FRONTMATTER, "new_field", "new_value");
        assert!(result.contains("new_field: new_value"));
    }

    #[test]
    fn test_update_array_field_replaces_array() {
        let result = update_array_field(
            TEMPLATE_FRONTMATTER,
            "tags",
            &["new-tag".to_string()],
        );
        assert!(result.contains("tags:"));
        assert!(result.contains("  - \"new-tag\""));
        // Old values gone
        assert!(!result.contains("\"rust\""));
        assert!(!result.contains("\"cli\""));
    }

    #[test]
    fn test_update_array_field_empty_values() {
        let result = update_array_field(TEMPLATE_FRONTMATTER, "tags", &[]);
        assert!(result.contains("tags: []"));
        assert!(!result.contains("\"rust\""));
    }

    #[test]
    fn test_update_array_field_preserves_surrounding() {
        let result = update_array_field(
            TEMPLATE_FRONTMATTER,
            "tags",
            &["one".to_string(), "two".to_string()],
        );
        // Comments before and after should remain
        assert!(result.contains("# Relationship"));
        assert!(result.contains("# CRM"));
    }
}
