pub mod detect;
pub mod docker;
pub mod local;
pub mod traits;

pub use detect::{InstallCommand, detect_install_command};
pub use docker::{DockerSandbox, DockerSandboxDriver};
pub use local::{LocalSandbox, LocalSandboxDriver};
pub use traits::{CommandOutput, SandboxDriver, SandboxInstance};
