use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstallCommand {
    pub cmd: String,
    pub args: Vec<String>,
}

pub fn detect_install_command(dir: &Path) -> Option<InstallCommand> {
    if dir.join("bun.lock").exists() || dir.join("bun.lockb").exists() {
        return Some(InstallCommand {
            cmd: "bun".to_string(),
            args: vec!["install".to_string(), "--frozen-lockfile".to_string()],
        });
    }

    if dir.join("pnpm-lock.yaml").exists() {
        return Some(InstallCommand {
            cmd: "pnpm".to_string(),
            args: vec!["install".to_string(), "--frozen-lockfile".to_string()],
        });
    }

    if dir.join("yarn.lock").exists() {
        return Some(InstallCommand {
            cmd: "yarn".to_string(),
            args: vec!["install".to_string(), "--frozen-lockfile".to_string()],
        });
    }

    if dir.join("package-lock.json").exists() {
        return Some(InstallCommand {
            cmd: "npm".to_string(),
            args: vec!["ci".to_string()],
        });
    }

    if dir.join("package.json").exists() {
        return Some(InstallCommand {
            cmd: "npm".to_string(),
            args: vec!["install".to_string()],
        });
    }

    if dir.join("Cargo.toml").exists() {
        return Some(InstallCommand {
            cmd: "cargo".to_string(),
            args: vec!["check".to_string()],
        });
    }

    if dir.join("go.mod").exists() {
        return Some(InstallCommand {
            cmd: "go".to_string(),
            args: vec!["mod".to_string(), "download".to_string()],
        });
    }

    if dir.join("requirements.txt").exists() {
        return Some(InstallCommand {
            cmd: "pip".to_string(),
            args: vec![
                "install".to_string(),
                "-r".to_string(),
                "requirements.txt".to_string(),
            ],
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_detect_bun() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("bun.lock")).unwrap();
        let cmd = detect_install_command(dir.path()).unwrap();
        assert_eq!(cmd.cmd, "bun");
    }

    #[test]
    fn test_detect_cargo() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("Cargo.toml")).unwrap();
        let cmd = detect_install_command(dir.path()).unwrap();
        assert_eq!(cmd.cmd, "cargo");
        assert_eq!(cmd.args, vec!["check"]);
    }
}
