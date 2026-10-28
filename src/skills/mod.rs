pub mod discovery;
pub mod frontmatter;
pub mod prompt;

pub use discovery::{discover_skills, discover_skills_in_dir};
pub use frontmatter::{SkillMetadata, parse_skill_file};
pub use prompt::build_skills_prompt;
