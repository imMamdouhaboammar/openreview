# OpenReview: LLM Provider Configuration Matrix

OpenReview decouples the agentic reasoning engine from specific proprietary SDKs through a unified provider trait.

---

## Supported Providers

### 1. Anthropic Claude (`LLM_PROVIDER=anthropic`)
- **Default Model**: `claude-3-7-sonnet-20250219` (or `claude-3-5-sonnet-20241022`)
- **Required Env Var**: `ANTHROPIC_API_KEY`
- **Strengths**: Best-in-class multi-step ReAct tool calling, precise AST and syntax editing, adheres strictly to GitHub suggestion formatting.
- **Config**:
  ```bash
  LLM_PROVIDER=anthropic
  LLM_MODEL=claude-3-7-sonnet-20250219
  ANTHROPIC_API_KEY=sk-ant-api03-...
  ```

### 2. OpenAI (`LLM_PROVIDER=openai`)
- **Default Model**: `gpt-4o` (or `gpt-4o-mini`, `o3-mini`)
- **Required Env Var**: `OPENAI_API_KEY`
- **Optional Env Var**: `OPENAI_BASE_URL` (for Azure OpenAI, Groq, or OpenAI-compatible proxies)
- **Strengths**: Fast tool calling latency, high token throughput.
- **Config**:
  ```bash
  LLM_PROVIDER=openai
  LLM_MODEL=gpt-4o
  OPENAI_API_KEY=sk-...
  # OPENAI_BASE_URL=https://api.openai.com/v1
  ```

### 3. Google Gemini (`LLM_PROVIDER=gemini`)
- **Default Model**: `gemini-2.5-flash` (or `gemini-2.5-pro`)
- **Required Env Var**: `GEMINI_API_KEY`
- **Strengths**: Massive context windows (up to 1M tokens), high reasoning speed, competitive pricing.
- **Config**:
  ```bash
  LLM_PROVIDER=gemini
  LLM_MODEL=gemini-2.5-flash
  GEMINI_API_KEY=AIzaSy...
  ```

### 4. Local Ollama / vLLM (`LLM_PROVIDER=ollama`)
- **Default Model**: `deepseek-r1:latest` (or `qwen2.5-coder:32b`, `llama3.3:70b`)
- **Required Env Var**: `OLLAMA_BASE_URL` (default: `http://localhost:11434`)
- **Strengths**: 100% offline, air-gapped security, zero external API costs.
- **Config**:
  ```bash
  LLM_PROVIDER=ollama
  LLM_MODEL=deepseek-r1:latest
  OLLAMA_BASE_URL=http://localhost:11434
  ```

---

## Model Selection Guidelines

| Repository Profile | Recommended Provider | Model |
|:-------------------|:---------------------|:------|
| Complex multi-file refactors & security audits | Anthropic | `claude-3-7-sonnet-20250219` |
| Fast routine PR checks & high-throughput queues | Google Gemini | `gemini-2.5-flash` |
| Standard corporate enterprise with OpenAI agreement | OpenAI | `gpt-4o` |
| Air-gapped / HIPAA / defense compliant codebases | Ollama | `deepseek-r1:latest` / `qwen2.5-coder:32b` |
