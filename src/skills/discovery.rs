use crate::skills::frontmatter::{SkillMetadata, parse_skill_file};
use std::collections::HashSet;
use std::path::Path;
use tokio::fs;
use tracing::{debug, warn};

const MAX_SKILL_BYTES: u64 = 128 * 1024;

// Repository-controlled symlinks must never redirect trusted orchestration
// reads to host paths (including service credentials outside the sandbox).
async fn is_plain_path(path: &Path, directory: bool) -> bool {
    let mut current = std::path::PathBuf::new();
    for component in path.components() {
        current.push(component.as_os_str());
        let Ok(metadata) = fs::symlink_metadata(&current).await else {
            return false;
        };
        if metadata.file_type().is_symlink() {
            return false;
        }
    }
    match fs::symlink_metadata(path).await {
        Ok(m) if directory => m.file_type().is_dir(),
        Ok(m) => m.file_type().is_file() && m.len() <= MAX_SKILL_BYTES,
        Err(_) => false,
    }
}

pub async fn discover_skills_in_dir(dir: &Path) -> Vec<SkillMetadata> {
    let mut skills = Vec::new();
    if !is_plain_path(dir, true).await {
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
        if is_plain_path(&path, true).await {
            let skill_file = path.join("SKILL.md");
            if is_plain_path(&skill_file, false).await {
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

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    #[tokio::test]
    async fn repository_skill_symlinks_cannot_read_outside_workspace() {
        let repo = tempfile::tempdir().unwrap();
        let secret = tempfile::tempdir().unwrap();
        let skills_dir = repo.path().join(".agents/skills");
        std::fs::create_dir_all(&skills_dir).unwrap();
        std::fs::write(
            secret.path().join("SKILL.md"),
            "---\nname: leak\ndescription: secret\n---\nTOP_SECRET",
        )
        .unwrap();
        symlink(secret.path(), skills_dir.join("linked-directory")).unwrap();
        std::fs::create_dir(skills_dir.join("linked-file")).unwrap();
        symlink(
            secret.path().join("SKILL.md"),
            skills_dir.join("linked-file/SKILL.md"),
        )
        .unwrap();
        assert!(discover_skills(&[&skills_dir]).await.is_empty());

        // Entire search roots can also be symlinks.
        let alternate = repo.path().join("linked-skills");
        symlink(&skills_dir, &alternate).unwrap();
        assert!(discover_skills(&[&alternate]).await.is_empty());
    }
}
