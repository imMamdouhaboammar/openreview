# Connect OpenReview to OpenCode models and custom providers

OpenReview uses OpenCode Console as an inference gateway. You can select any
**accessible text model** from your account; the model must support the API
format and tools that OpenReview needs. No Rust changes are required to switch
between models or to select a configured OpenCode connection.

> OpenCode account access, model entitlements, provider-specific capabilities,
> API usage limits and billing still apply. Model availability is not a guarantee
> that a particular model accepts tool calls or has remaining quota.

## 1. OpenCode Console (pay-as-you-go)

Create an API key in your OpenCode Console workspace. Set the following
environment variables (through your secrets manager, never in Git):

```dotenv
LLM_PROVIDER=opencode
LLM_MODEL=gemini-3.8-flash
OPENCODE_API_KEY=your_console_key_from_secrets
```

OpenReview sends OpenAI-compatible chat requests to the Console gateway. The
gateway can translate requests to the model's native API. The model must be
enabled for your workspace. OpenCode's **free-tier chat models are generally
restricted to the official OpenCode client**, so external API requests can
receive `403` or quota errors even with a valid key.

For an OpenCode Go subscription, use `LLM_PROVIDER=opencode_go` and an
accessible Go model. OpenReview maintains a stable `x-opencode-session` header
for the run; do not hardcode session IDs in workflow configuration.

## 2. An OpenCode Custom Provider connection

OpenCode Console's **Providers** page has a Console API URL for each
configured connection. Copy the URL, not the upstream provider's private API
key. For example:

```text
https://opencode.ai/inference/custom/conn_REPLACEID
```

Set:

```dotenv
LLM_PROVIDER=opencode_custom
LLM_MODEL=gemini-3.8-flash
OPENCODE_API_KEY=your_console_key_from_secrets
OPENCODE_CUSTOM_URL=https://opencode.ai/inference/custom/conn_REPLACEID
OPENCODE_CUSTOM_PROTOCOL=gemini
```

Choose the protocol to match the connection's **actual provider API**, not
just its model name:

| `OPENCODE_CUSTOM_PROTOCOL` | Request sent by OpenReview | Typical connection |
| --- | --- | --- |
| `chat` | `/chat/completions` | OpenAI-compatible chat |
| `responses` | `/responses` | OpenAI Responses |
| `anthropic` | `/messages` | Anthropic-compatible |
| `gemini` | `/models/<model>:generateContent` | Google Generative AI |

All four adapters use OpenReview's normal Review/Fix agent interface,
including message history, responses, and tool calls. They do **not** guarantee
that the upstream model implements tool/function calling correctly; verify
your model with a controlled test PR before enabling Fix.

The `gemini` adapter sends tool results as Google `functionResponse` parts,
groups parallel responses into one `role: user` content, and replays the
opaque `thoughtSignature` attached to each model function-call part. These
details are required by Gemini thinking models in multi-step tool workflows.
When a Review reaches its last permitted inference turn, OpenReview requests
a qualified text report with no available tools and sets Gemini's
`toolConfig.functionCallingConfig.mode` to `NONE`. The report must distinguish
verified findings from unverified areas; no extra tool call is authorized.
The custom `responses` adapter handles OpenAI `function_call` and
`function_call_output` items.

**Security:** `OPENCODE_CUSTOM_URL` must be an HTTPS
`opencode.ai/inference/custom/conn_<id>` URL. Arbitrary hosts, credentials in
the URL, query strings and extra path components are rejected. OpenReview sends
the Console key as a request header and does not follow redirects on custom
requests. The upstream provider key stays inside your OpenCode workspace.

## 3. GitHub-hosted OpenReview Action

Save the Console credential as `OPENCODE_API_KEY` in the **calling
repository's Actions Secrets**. With a released, reviewed OpenReview Action,
a custom Gemini connection can be configured like this:

```yaml
- uses: imMamdouhaboammar/openreview@vX.Y.Z # replace with a published audited release
  with:
    provider: opencode_custom
    model: gemini-3.8-flash
    opencode_custom_url: https://opencode.ai/inference/custom/conn_REPLACEID
    opencode_custom_protocol: gemini
    opencode_api_key: ${{ secrets.OPENCODE_API_KEY }}
```

The reference `@vX.Y.Z` is **not runnable until that release exists**. Use
the full workflow in [`examples/github-actions.yml`](../examples/github-actions.yml)
and keep its permissions, event filters and rootless sandbox checks intact.
The Action only exposes the key to the trusted final agent process, not the
untrusted Docker workspace.

For Docker Compose, add the five environment variables from section 2 to
your securely managed `.env` file. The Compose service forwards the
`OPENCODE_CUSTOM_URL` and `OPENCODE_CUSTOM_PROTOCOL` values; the model's
provider does not need a separate upstream Google/Anthropic key.

## 4. Validate the full PR mention workflow

These checks are required for a real deployment, not covered by a model-list
request or a successful standalone inference request:

1. Use an **audited, published** Action reference (or a deployed GitHub App).
   Configure the repository's Actions secret and the matching provider/model
   settings. The placeholder `@vX.Y.Z` is not a working release.
2. In a same-repository test PR, comment `@openreview review`. Confirm the
   GitHub event starts an OpenReview workflow, authenticates to the correct PR,
   reads the diff, performs model/tool round trips through the rootless sandbox,
   and publishes a real review comment. A read-only Review must never modify
   the branch or publish commits.
3. Verify the model can call `readFile`, receive the file contents, and then
   produce a final response. A plain one-shot prompt response does not prove
   the tool-enabled agent works.
4. Only from a controlled branch with a collaborator who has **write** access,
   comment `@openreview fix <specific task>`. The authorized Fix must run in
   a separate writable sandbox, modify the intended files, execute relevant
   tests, publish with a normal **non-force push**, verify GitHub's final head
   SHA, and report the actual outcome. Never trial Fix on a protected branch.
5. Also test negative cases: unqualified mentions default to Review, an
   unauthorized Fix author is denied, fork Fix is rejected, failed model/tool
   calls do not publish code, and stale PR heads prevent publication.

The hosted CI uses a mock model to test authenticated PR lookup, rootless
sandbox execution and tool-call round trips. **It cannot prove live OpenCode
model interoperability or an authorized remote Fix push.** Run those two
integration checks with short-lived credentials before release.

## 5. Smoke-check and troubleshooting

First verify the connection ID, model and protocol in OpenCode Console.
For the example native Gemini endpoint, test with a short prompt before
running any authenticated Fix job:

```bash
curl --fail-with-body --max-time 30 \
  "$OPENCODE_CUSTOM_URL/models/$LLM_MODEL:generateContent" \
  -H "Authorization: Bearer $OPENCODE_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"contents":[{"role":"user","parts":[{"text":"Reply OK."}]}],"generationConfig":{"maxOutputTokens":128}}'
```

Avoid `curl -v`, shell tracing, CI debug dumps, or printing tokens. A smoke
request can incur provider charges; use a minimal prompt and check your
workspace budget.

- `401`: credential missing, expired, or revoked.
- `403`: insufficient key/connection permission, model not enabled, or
  a free-tier restriction for external clients.
- `400`: request format/model/role mismatch; check the configured protocol.
- `429`: quota or usage limit. A successful model list request alone does
  not prove inference is available.
- `5xx` or transport error: provider/gateway unavailable; do not silently
  fall back to a different provider or credential.

OpenReview's tests use local HTTP fixtures to check request formats,
authorization headers and tool-result parsing. Those tests are **not proof
of live interoperability with every custom model**. Release verification
still requires a live test on the candidate head using your connection,
followed by an authorized Review and Fix flow when permitted.

References: [OpenCode Providers API](https://opencode.ai/v2/docs/console/api/providers/)
and [OpenCode Harnesses](https://opencode.ai/v2/docs/console/harnesses).
