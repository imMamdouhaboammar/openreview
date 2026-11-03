use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "openreview")]
#[command(about = "High-performance self-hosted AI code review bot and microservice in Rust", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start the OpenReview microservice daemon
    Serve {
        /// Host to bind on
        #[arg(long, env = "HOST", default_value = "0.0.0.0")]
        host: String,

        /// Port to bind on
        #[arg(short, long, env = "PORT", default_value_t = 3000)]
        port: u16,
    },

    /// Run an on-demand review for a specific pull request
    Review {
        /// Target repository (e.g. owner/repo)
        #[arg(short, long)]
        repo: String,

        /// Pull request number
        #[arg(short, long)]
        pr: u64,

        /// PR branch name (optional, fetched from GitHub if omitted)
        #[arg(short, long)]
        branch: Option<String>,

        /// Run in dry-run mode (do not post comments or push commits to remote)
        #[arg(long)]
        dry_run: bool,
    },

    /// Run the full agent for an authenticated GitHub Actions event, without a webhook server
    Action {
        /// Perform analysis and local changes without any GitHub comments or pushes
        #[arg(long)]
        dry_run: bool,
    },

    /// Verify local configuration and credential validity
    VerifyConfig,

    /// Generate a GitHub-owned registration link prefilled for this self-hosted deployment
    GithubAppUrl {
        /// Public HTTPS origin where the webhook will be hosted, e.g. https://review.example.com
        #[arg(long)]
        domain: String,

        /// Unique GitHub App display name (set by the deployment owner)
        #[arg(long)]
        name: String,

        /// Register under a GitHub organization instead of your personal account
        #[arg(long)]
        org: Option<String>,

        /// Make the App installable by other accounts and organizations
        #[arg(long)]
        public: bool,
    },

    /// Test sending a synthetic signed webhook to a running daemon
    TestWebhook {
        /// Webhook URL to target
        #[arg(long, default_value = "http://127.0.0.1:3000/api/webhooks/github")]
        url: String,

        /// PR number to simulate
        #[arg(long, default_value_t = 1)]
        pr: u64,

        /// Repository to simulate
        #[arg(long, default_value = "example/repo")]
        repo: String,
    },

    /// End-to-End autonomous error finding, replay verification, and cache management
    E2e {
        #[command(subcommand)]
        subcommand: E2eSubcommands,
    },
}

#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum E2eSubcommands {
    /// Probe a target URL, preview app, or local server for runtime errors, broken routes, and accessibility defects
    FindErrors {
        /// Target URL to probe (e.g. http://localhost:3000 or https://preview.example.com)
        #[arg(short, long)]
        url: String,

        /// Maximum exploration depth
        #[arg(short, long, default_value_t = 3)]
        depth: usize,

        /// Run Causal Delta-Debugging (ddmin) to minimize reproducing steps
        #[arg(long, default_value_t = true)]
        causal: bool,
    },

    /// Run and verify a recorded E2E replay manifest against a live or sandboxed endpoint
    Run {
        /// Replay manifest test name or file path
        #[arg(short, long)]
        test_name: String,

        /// Base URL override for testing environments
        #[arg(short, long)]
        url: Option<String>,

        /// Path to replay cache directory
        #[arg(long, default_value = ".openreview/e2e-cache")]
        cache_dir: String,
    },

    /// Manage the E2E replay cache
    Cache {
        /// Cache operation: status or clean
        #[arg(default_value = "status")]
        action: String,

        /// Path to replay cache directory
        #[arg(long, default_value = ".openreview/e2e-cache")]
        cache_dir: String,
    },
}

#[cfg(test)]
mod tests {
    use super::{Cli, Commands, E2eSubcommands};
    use clap::Parser;

    #[test]
    fn e2e_command_parses_subcommands() {
        let args = Cli::try_parse_from([
            "openreview",
            "e2e",
            "find-errors",
            "--url",
            "http://localhost:3000",
            "--depth",
            "5",
        ])
        .unwrap();
        assert!(matches!(
            args.command,
            Commands::E2e {
                subcommand: E2eSubcommands::FindErrors { depth: 5, .. }
            }
        ));
    }

    #[test]
    fn registration_link_is_a_first_class_command_without_github_credentials() {
        let args = Cli::try_parse_from([
            "openreview",
            "github-app-url",
            "--domain",
            "https://review.example.org",
            "--name",
            "OpenReview for Mamdouh",
            "--org",
            "my-org",
            "--public",
        ])
        .unwrap();
        assert!(matches!(
            args.command,
            Commands::GithubAppUrl {
                public: true,
                org: Some(_),
                ..
            }
        ));
    }
}
