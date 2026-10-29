pub mod detect;
pub mod docker;
pub mod local;
pub mod path;
pub mod traits;

pub use detect::{InstallCommand, detect_install_command};
pub use docker::{DockerSandbox, DockerSandboxDriver};
pub use local::{LocalSandbox, LocalSandboxDriver};
pub use path::safe_resolve_path;
pub use traits::{CommandOutput, SandboxDriver, SandboxInstance};
