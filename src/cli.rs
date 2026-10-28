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

    /// Verify local configuration and credential validity
    VerifyConfig,

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
}
