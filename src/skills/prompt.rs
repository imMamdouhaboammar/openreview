use crate::skills::frontmatter::SkillMetadata;

pub fn build_skills_prompt(skills: &[SkillMetadata]) -> String {
    if skills.is_empty() {
        return String::new();
    }

    let mut prompt = String::from("## Skills\n\n");
    prompt.push_str("Use the `loadSkill` tool to load a skill when the user's request would benefit from specialized instructions. Only the skill names and descriptions are shown here — load a skill to get the full instructions.\n\nAvailable skills:\n");

    for s in skills {
        prompt.push_str(&format!("- **{}**: {}\n", s.name, s.description));
    }

    prompt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_skills_prompt() {
        assert_eq!(build_skills_prompt(&[]), "");
    }

    #[test]
    fn test_skills_prompt_formatting() {
        let skills = vec![
            SkillMetadata {
                name: "rust-check".to_string(),
                description: "Lint Rust files".to_string(),
                content: "content".to_string(),
            },
            SkillMetadata {
                name: "web-audit".to_string(),
                description: "Audit accessibility".to_string(),
                content: "content".to_string(),
            },
        ];

        let rendered = build_skills_prompt(&skills);
        assert!(rendered.contains("- **rust-check**: Lint Rust files"));
        assert!(rendered.contains("- **web-audit**: Audit accessibility"));
        assert!(rendered.contains("loadSkill"));
    }
}
