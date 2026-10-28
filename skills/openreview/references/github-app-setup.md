# OpenReview: GitHub App Setup Guide

This guide details how to create and configure the GitHub App required by OpenReview.

---

## 1. Create the GitHub App

Navigate to **GitHub Settings > Developer Settings > GitHub Apps > New GitHub App**:

1. **GitHub App name**: `OpenReview Bot` (or your company name, e.g. `Acme Review Bot`).
2. **Homepage URL**: Your repository URL or service URL (e.g. `https://github.com/your-org/openreview`).
3. **Webhook URL**: `https://<your-domain>/api/webhooks/github` (or your reverse proxy / tunnel URL).
4. **Webhook secret**: Generate a secure high-entropy secret:
   ```bash
   openssl rand -hex 32
   ```
   Save this value to `GITHUB_APP_WEBHOOK_SECRET`.

---

## 2. Required Permissions Matrix

In the **Repository permissions** tab, configure the following grants:

| Permission | Access Level | Purpose |
|:-----------|:-------------|:--------|
| **Pull requests** | **Read & write** | Post review comments, read PR diffs, submit pull request reviews, list files changed. |
| **Issues** | **Read & write** | Read and post comments on issue threads and pull request discussion conversations. |
| **Contents** | **Read & write** | Clone the repository branch, commit verified remediations, and push changes back to the PR branch. |
| **Metadata** | **Read-only** | Basic repository metadata (mandatory for all GitHub Apps). |

---

## 3. Webhook Events Subscriptions

Subscribe to the following events under **Subscribe to events**:

- [x] **Pull request**: Triggered when PRs are opened, synchronized (new commits pushed), or labeled.
- [x] **Pull request review comment**: Triggered when developers comment on line-level diffs or mention `@openreview`.
- [x] **Issue comment**: Triggered when developers mention `@openreview` in main PR conversation threads.

---

## 4. Generate Private Key

1. Scroll to the bottom of your GitHub App settings page.
2. Click **Generate a private key**.
3. A `.pem` file will automatically download.
4. Convert this key to a single-line string with escaped newlines (`\n`) for environment storage, or mount it as a file:
   ```bash
   awk '{printf "%s\\n", $0}' your-app-private-key.pem
   ```
5. Set `GITHUB_APP_PRIVATE_KEY` to this string.

---

## 5. Install the GitHub App

1. In the left sidebar of your App settings, click **Install App**.
2. Select your user account or target GitHub organization.
3. Choose either **All repositories** or **Only select repositories**.
4. Note the numeric `installation_id` from the URL after installation:
   `https://github.com/settings/installations/<INSTALLATION_ID>`
5. Save this to `GITHUB_APP_INSTALLATION_ID`.
