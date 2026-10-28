# OpenReview Component Specifications

Component definitions for the status landing UI and GitHub markdown review renderings.

---

## 1. Status Dashboard Header (`Header`)
- **Variants**: Standard dark minimal header.
- **States**: Online, Webhook Ingestion Active.
- **Tokens**: `bg-surface`, `border-border`, `text-text`, `font-sans`.

## 2. GitHub PR Comment Card (`PRComment`)
- **Structure**:
  - Issue summary header (Title, severity indicator).
  - Problem explanation (what is wrong, why it matters).
  - Remediation instructions.
  - Suggestion block:
    ````markdown
    ```suggestion
    let client = reqwest::Client::new();
    ```
    ````
  - Attribution footer:
    `---`
    `*Powered by [OpenReview-RS](https://github.com/imMamdouhaboammar/openreview-rs)*`

## 3. Metrics Card (`MetricsCard`)
- **Displays**: Active Reviews, Processed Webhooks, Average Latency, Sandbox Memory.
- **Tokens**: `bg-surface`, `border-border`, `text-muted`.
