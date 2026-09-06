# Model Selection

Iris uses separate models for generation, delegated analysis, and status messages. The primary
model handles commits, reviews, PRs, changelogs, release notes, and chat. Subagents use the primary
model unless you configure `subagent_model`. The fast model writes progress messages.

## Default Models and Effort

| Provider  | Primary model      | Primary effort | Subagent effort | Status model                                     |
| --------- | ------------------ | -------------- | --------------- | ------------------------------------------------ |
| OpenAI    | `gpt-6-astra`      | `medium`       | `low`           | `gpt-5.6-luna`, reasoning `none`                 |
| Anthropic | `claude-opus-5`    | `high`         | `low`           | `claude-haiku-4-5-20251001`, no effort parameter |
| Google    | `gemini-3.8-flash` | `medium`       | `low`           | `gemini-3.5-flash-lite`                          |

These defaults were checked against provider documentation in September 2026. Model availability
still depends on your account. Existing explicit model choices stay configured until you change them.
See [Providers](./providers) for OpenRouter and Fireworks configuration.

## Configure Each Role

```bash
# Primary analysis model
git-iris config --provider openai --model gpt-6-astra

# Optional independent model for delegated analysis
git-iris config --provider openai --subagent-model gpt-5.6-terra

# Lightweight progress messages
git-iris config --provider openai --fast-model gpt-5.6-luna
```

The same fields work in the global provider configuration:

```toml
[providers.openai]
model = "gpt-6-astra"
subagent_model = "gpt-5.6-terra"
fast_model = "gpt-5.6-luna"
```

Leave `subagent_model` unset to use the primary model for delegated analysis. Changing `fast_model`
only changes status generation. Use lower effort or a different worker model after comparing review
findings and completion quality on representative repositories.

## Reasoning Controls

Iris chooses effort by task role. You can override provider parameters through `--param`:

```bash
git-iris config --provider openai --param reasoning='{"effort":"medium"}'
git-iris config --provider anthropic --param output_config='{"effort":"high"}'
```

Explicit parameters apply to requests using that provider configuration. Check the selected model's
supported parameters before overriding defaults, especially when the status model differs from the
primary model. Anthropic Haiku does not support the effort parameter.

Astra uses the Responses API for tool calling. Astra does not accept reasoning `none` or `minimal`,
and does not support sampling parameters such as `temperature` and `top_p`.
[OpenAI migration guidance](https://developers.openai.com/api/docs/guides/latest-model?model=gpt-6-astra)
explains the request requirements.

Opus 5 uses adaptive thinking and defaults to high effort. Haiku 4.5 remains the lightweight status
model. See [Anthropic's model overview](https://platform.claude.com/docs/en/models/overview).

Google's Gemini 3.8 Flash supports `low`, `medium`, and `high` thinking. The old
`gemini-3-pro-preview` endpoint was retired; update saved configurations that still name it.
See [Gemini 3.8 Flash](https://ai.google.dev/gemini-api/docs/models/gemini-3.8-flash) and
[Google's deprecation schedule](https://ai.google.dev/gemini-api/docs/deprecations).

## Context and Output Budgets

A model's context window covers input and conversation history. Its output limit is a separate
constraint. A larger output budget does not make the input context window larger.

The primary Astra and Opus models have roughly one million tokens of context, as does Gemini 3.8
Flash. Haiku's context is 200K. Tool calls still use targeted file excerpts and diff summaries to
keep evidence relevant. Those tools do not guarantee that every conversation fits its model's
context window.

## Availability and Diagnostics

Choose an explicit model ID offered by your provider. Iris reports provider errors when a selected
model is unavailable; it does not silently switch the main task to a different model.

```bash
git-iris gen --debug
```

Debug output shows agent activity and token usage. If a request exceeds context, narrow the requested
comparison or choose a model with sufficient context. Iris currently requests 16,384 output tokens for main tasks and 4,096 for subagents;
`token_limit` is context metadata and does not change those budgets. Authentication errors require checking the active
provider's key, independently of model selection.
