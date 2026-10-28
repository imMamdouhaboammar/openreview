use crate::error::{AppError, Result};
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct SkillMetadata {
    pub name: String,
    pub description: String,
    pub content: String,
}

#[derive(Deserialize)]
struct Frontmatter {
    name: String,
    description: String,
}

pub fn parse_skill_file(raw: &str) -> Result<SkillMetadata> {
    let trimmed = raw.trim();
    if !trimmed.starts_with("---") {
        return Err(AppError::Skill(
            "No frontmatter delimiter found at start".to_string(),
        ));
    }

    let rest = &trimmed[3..];
    let end_idx = rest
        .find("\n---")
        .or_else(|| rest.find("\r\n---"))
        .ok_or_else(|| AppError::Skill("Closing frontmatter delimiter not found".to_string()))?;

    let fm_yaml = &rest[..end_idx].trim();
    let body_start = end_idx + 4; // Skip \n---
    let body = rest[body_start..].trim_start_matches(['\r', '\n']).trim();

    let fm: Frontmatter = serde_yaml::from_str(fm_yaml)
        .map_err(|e| AppError::Skill(format!("Failed to parse YAML frontmatter: {e}")))?;

    if fm.name.trim().is_empty() {
        return Err(AppError::Skill("Skill name cannot be empty".to_string()));
    }
    if fm.description.trim().is_empty() {
        return Err(AppError::Skill(
            "Skill description cannot be empty".to_string(),
        ));
    }

    Ok(SkillMetadata {
        name: fm.name.trim().to_string(),
        description: fm.description.trim().to_string(),
        content: body.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_skill_parsing() {
        let raw = r#"---
name: rust-best-practices
description: Review Rust code for idioms, safety, and performance
---

# Rust Best Practices

1. Use pattern matching
2. Avoid unwrap in production
"#;

        let skill = parse_skill_file(raw).unwrap();
        assert_eq!(skill.name, "rust-best-practices");
        assert_eq!(
            skill.description,
            "Review Rust code for idioms, safety, and performance"
        );
        assert!(skill.content.contains("# Rust Best Practices"));
        assert!(skill.content.contains("Avoid unwrap in production"));
    }

    #[test]
    fn test_missing_frontmatter() {
        let raw = "# Just a markdown file";
        assert!(parse_skill_file(raw).is_err());
    }

    #[test]
    fn test_missing_description() {
        let raw = r#"---
name: incomplete-skill
---
# Content"#;
        assert!(parse_skill_file(raw).is_err());
    }
}
