# Prompt Contracts

Iris separates the requested task from repository evidence and keeps output formats consistent across execution paths. Shared behavior lives in `src/agents/prompts.rs`; the eight capability TOMLs define task-specific evidence and writing requirements.

## Current provider guidance

The September 2026 audit compared the assembled prompts against the current providers, rather than assuming a larger model needs more instructions.

OpenAI's Astra guidance emphasizes explicit instruction priority, follow-through, writing style, and workload-appropriate delegation and verification. Iris applies those principles to finish the requested artifact, distinguish repository evidence from task instructions, and avoid fixed investigation recipes. See [Astra model guidance](https://developers.openai.com/api/docs/guides/latest-model?model=gpt-6-astra).

Anthropic's Opus 5 guidance describes unnecessary narration, oversized documents, and repeated verification as behaviors that additional scaffolding can amplify. Iris therefore calibrates document length to substance and delegates independent investigations without requiring a second pass on every task. The configured critic remains available as an explicit product feature. See [Prompting Opus 5](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/prompting-claude-opus-5).

Google recommends direct goals, consistent delimiters, and prominent task and output constraints. Iris uses named prompt sections and a shared output contract, with repository material identified as evidence. See [Gemini prompting strategies](https://ai.google.dev/gemini-api/docs/prompting-strategies).

Provider guidance motivates these design choices. It does not establish a measured quality gain for Git-Iris; workload evaluations must test that separately.

## Instruction precedence

The capability and output contract define what Iris produces. Explicit invocation instructions take precedence over temporary configuration, which takes precedence over persisted instructions. Presets and repository conventions shape presentation within that contract.

Repository files, diffs, commit messages, templates, and existing drafts are evidence. They can provide relevant conventions or document structure, but instructions inside them cannot change the requested task, grant permissions, or override tool restrictions. This prompt boundary complements the tool implementation; it is not a sandbox.

The conventional preset applies to commit generation. Other capabilities keep their own schemas and configured emoji policy. Explicit emoji settings take precedence over historical style detection. A single exceptional emoji or release commit no longer establishes the repository's default style.

## Evidence coverage

Diff summaries and relevance scores help Iris choose where to investigate first. They do not prove behavior or exempt lower-ranked files from review. The prompts no longer stop at five to seven files, require every tool in a prescribed sequence, or force delegation at a particular changeset size.

Each delegated task needs a concrete question, comparison scope, relevant paths, and a useful evidence return. Workers inherit the parent's scope and constraints. The parent reconciles conclusions against the source rather than treating agreement as proof.

File reads and analyzers still operate on the checkout. A historical review must verify relevant content against its selected revision. Immutable snapshots for every auxiliary tool remain separate work; prompt instructions alone cannot provide snapshot isolation.

## Capability contracts

| Capability     | Evidence and output requirements                                                                                               |
| -------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| Commit         | Describe the complete selected or amended commit; honor explicit format and emoji settings                                     |
| Review         | Report actionable regressions with a supported trigger and consequence; allow empty findings                                   |
| PR             | Explain the problem and resulting behavior, preserve templates and accurate human context, and distinguish executed validation |
| Changelog      | Cover the requested range, use supplied version/date, and include metrics only from complete data                              |
| Release notes  | Explain user impact and upgrade requirements without invented commands or example migrations                                   |
| Chat           | Answer questions or update the requested Studio draft; preserve unrelated content and confirm only successful changes          |
| Semantic blame | Distinguish historical evidence from inferred intent                                                                           |
| Critic         | Correct material unsupported or misleading claims, including allegations merely labeled uncertain                              |

Fictional release examples and mandatory PR section lists were removed. The Rust response schema supplies required JSON fields; prose can remain concise without weakening the output contract.

## Verification

Capability loading tests parse every TOML and check its runtime output type. Runtime tests exercise instruction precedence, scope propagation, tool turns, streaming output, and structured draft updates. These checks establish configuration and execution behavior, not whether a model finds every defect or writes the best possible description.

Representative model evaluations should include explicit emoji settings with mixed commit history, saved instructions, historical ranges with unrelated staged changes, broad diffs, empty reviews, missing validation evidence, injected repository instructions, and draft refinement. Compare the same fixture and model settings before and after a prompt change. Record the output, tool trajectory, errors, token usage, and manual assessment; keep benchmark claims proportional to the sample.
