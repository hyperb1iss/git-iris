# GitHub Action

Git-Iris is available as a GitHub Action for automating release notes, changelogs, and other Git artifacts directly in your CI/CD pipelines.

## Quick Start

```yaml
- name: Generate Release Notes
  uses: hyperb1iss/git-iris@v2
  with:
    command: release-notes
    from: v1.0.0
    to: v2.0.0
    provider: openai
    api-key: ${{ secrets.OPENAI_API_KEY }}
    output-file: RELEASE_NOTES.md
```

## Inputs

| Input                 | Description                                     | Required | Default          |
| --------------------- | ----------------------------------------------- | -------- | ---------------- |
| `command`             | Command to run: `release-notes`, `changelog`    | No       | `release-notes`  |
| `from`                | Starting Git reference (tag, commit, or branch) | **Yes**  | -                |
| `to`                  | Ending Git reference                            | No       | `HEAD`           |
| `provider`            | LLM provider: `openai`, `anthropic`, `google`   | No       | `openai`         |
| `model`               | Model to use (provider-specific)                | No       | Provider default |
| `api-key`             | API key for the LLM provider                    | **Yes**  | -                |
| `output-file`         | File path to write output                       | No       | -                |
| `version-name`        | Explicit version name to use in output          | No       | -                |
| `custom-instructions` | Custom instructions for generation              | No       | -                |
| `update-file`         | Apply native `git-iris --update` behavior       | No       | `false`          |
| `version`             | Git-Iris version to use                         | No       | `latest`         |
| `build-from-source`   | Build from source instead of binary             | No       | `false`          |
| `binary-path`         | Path to pre-built binary                        | No       | -                |

## Outputs

| Output        | Description                            |
| ------------- | -------------------------------------- |
| `content`     | Generated content as a string          |
| `output-file` | Path to the output file (if specified) |

## Examples

### Generate Release Notes for a Tag

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Get previous tag
        id: prev_tag
        run: |
          PREV=$(git describe --tags --abbrev=0 HEAD^ 2>/dev/null || echo "")
          echo "tag=$PREV" >> $GITHUB_OUTPUT

      - name: Generate Release Notes
        id: notes
        uses: hyperb1iss/git-iris@v2
        with:
          command: release-notes
          from: ${{ steps.prev_tag.outputs.tag }}
          to: ${{ github.ref_name }}
          version-name: ${{ github.ref_name }}
          provider: openai
          api-key: ${{ secrets.OPENAI_API_KEY }}
          output-file: RELEASE_NOTES.md

      - name: Create GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          body_path: RELEASE_NOTES.md
```

### Update CHANGELOG.md

```yaml
- name: Update Changelog
  uses: hyperb1iss/git-iris@v2
  with:
    command: changelog
    from: v1.0.0
    to: HEAD
    version-name: '2.0.0'
    provider: openai
    api-key: ${{ secrets.OPENAI_API_KEY }}
    output-file: CHANGELOG.md
    update-file: 'true' # Uses native changelog update behavior
```

### Use with Different Providers

#### OpenAI

```yaml
- uses: hyperb1iss/git-iris@v2
  with:
    command: release-notes
    from: v1.0.0
    provider: openai
    model: gpt-6-astra
    api-key: ${{ secrets.OPENAI_API_KEY }}
```

#### Anthropic

```yaml
- uses: hyperb1iss/git-iris@v2
  with:
    command: release-notes
    from: v1.0.0
    provider: anthropic
    api-key: ${{ secrets.ANTHROPIC_API_KEY }}
```

#### Google

```yaml
- uses: hyperb1iss/git-iris@v2
  with:
    command: release-notes
    from: v1.0.0
    provider: google
    api-key: ${{ secrets.GOOGLE_API_KEY }}
```

### Custom Instructions

```yaml
- uses: hyperb1iss/git-iris@v2
  with:
    command: release-notes
    from: v1.0.0
    provider: openai
    api-key: ${{ secrets.OPENAI_API_KEY }}
    custom-instructions: |
      Focus on user-facing changes.
      Use simple language suitable for non-technical users.
      Group changes by feature area.
```

### Use Output in Subsequent Steps

```yaml
- name: Generate Notes
  id: notes
  uses: hyperb1iss/git-iris@v2
  with:
    command: release-notes
    from: v1.0.0
    provider: openai
    api-key: ${{ secrets.OPENAI_API_KEY }}

- name: Use Generated Content
  run: |
    echo "Generated content:"
    echo "${{ steps.notes.outputs.content }}"
```

### Pin to Specific Version

```yaml
- uses: hyperb1iss/git-iris@v2
  with:
    command: release-notes
    from: v1.0.0
    provider: openai
    api-key: ${{ secrets.OPENAI_API_KEY }}
    version: v2.0.0 # Use specific git-iris version
```

## Supported Platforms

The action automatically downloads the appropriate binary for your runner:

| Runner             | Architecture | Binary                     |
| ------------------ | ------------ | -------------------------- |
| `ubuntu-latest`    | x64          | `git-iris-linux-amd64`     |
| `ubuntu-24.04-arm` | ARM64        | `git-iris-linux-arm64`     |
| `macos-latest`     | ARM64        | `git-iris-macos-arm64`     |
| `windows-latest`   | x64          | `git-iris-windows-gnu.exe` |

## Tips

### Fetch Full History

The action needs access to the Git history between your `from` and `to` references:

```yaml
- uses: actions/checkout@v4
  with:
    fetch-depth: 0 # Fetch all history for all tags and branches
```

### Store API Keys Securely

Always use GitHub Secrets for API keys:

1. Go to your repository Settings > Secrets and variables > Actions
2. Click "New repository secret"
3. Add your provider's API key (for example `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, or `GOOGLE_API_KEY`)

### Build from Source (Advanced)

If you need the latest features or encounter issues with the binary:

```yaml
- uses: hyperb1iss/git-iris@v2
  with:
    command: release-notes
    from: v1.0.0
    provider: openai
    api-key: ${{ secrets.OPENAI_API_KEY }}
    build-from-source: 'true'
```

::: warning
Building from source significantly increases workflow time (~2-3 minutes for compilation).
:::

### Use Pre-built Binary

If you've already built git-iris in a previous step:

```yaml
- name: Build git-iris
  run: cargo build --release

- uses: hyperb1iss/git-iris@v2
  with:
    command: release-notes
    from: v1.0.0
    provider: openai
    api-key: ${{ secrets.OPENAI_API_KEY }}
    binary-path: ./target/release/git-iris
```
