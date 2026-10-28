use crate::skills::frontmatter::{SkillMetadata, parse_skill_file};
use std::collections::HashSet;
use std::path::Path;
use tokio::fs;
use tracing::{debug, warn};

pub async fn discover_skills_in_dir(dir: &Path) -> Vec<SkillMetadata> {
    let mut skills = Vec::new();
    if !dir.exists() {
        return skills;
    }

    let mut entries = match fs::read_dir(dir).await {
        Ok(e) => e,
        Err(err) => {
            debug!("Could not read skills directory {}: {err}", dir.display());
            return skills;
        }
    };

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.is_dir() {
            let skill_file = path.join("SKILL.md");
            if skill_file.exists() {
                match fs::read_to_string(&skill_file).await {
                    Ok(content) => match parse_skill_file(&content) {
                        Ok(metadata) => skills.push(metadata),
                        Err(err) => {
                            warn!("Failed to parse skill at {}: {err}", skill_file.display())
                        }
                    },
                    Err(err) => warn!("Failed to read skill at {}: {err}", skill_file.display()),
                }
            }
        }
    }

    skills
}

pub async fn discover_skills(search_paths: &[&Path]) -> Vec<SkillMetadata> {
    let mut all_skills = Vec::new();
    let mut seen_names = HashSet::new();

    for path in search_paths {
        let skills = discover_skills_in_dir(path).await;
        for skill in skills {
            if !seen_names.contains(&skill.name.to_lowercase()) {
                seen_names.insert(skill.name.to_lowercase());
                all_skills.push(skill);
            }
        }
    }

    all_skills
}
