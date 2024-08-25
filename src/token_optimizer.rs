use crate::context::CommitContext;
use tiktoken_rs::cl100k_base;

pub struct TokenOptimizer {
    encoder: tiktoken_rs::CoreBPE,
    max_tokens: usize,
}

impl TokenOptimizer {
    #[allow(clippy::unwrap_used)] // todo: handle unwrap
    pub fn new(max_tokens: usize) -> Self {
        Self {
            encoder: cl100k_base().unwrap(),
            max_tokens,
        }
    }

    pub fn optimize_context(&self, context: &mut CommitContext) {
        let mut remaining_tokens = self.max_tokens;

        // Step 1: Allocate tokens for the diffs (highest priority)
        for file in &mut context.staged_files {
            let diff_tokens = self.count_tokens(&file.diff);
            if diff_tokens > remaining_tokens {
                file.diff = self.truncate_string(&file.diff, remaining_tokens);
                remaining_tokens = 0;
            } else {
                remaining_tokens = remaining_tokens.saturating_sub(diff_tokens);
            }

            if remaining_tokens == 0 {
                // If we exhaust the tokens in step 1, clear commits and contents
                Self::clear_commits_and_contents(context);
                return;
            }
        }

        // Step 2: Allocate remaining tokens for recent commits (medium priority)
        for commit in &mut context.recent_commits {
            let commit_tokens = self.count_tokens(&commit.message);
            if commit_tokens > remaining_tokens {
                commit.message = self.truncate_string(&commit.message, remaining_tokens);
                remaining_tokens = 0;
            } else {
                remaining_tokens = remaining_tokens.saturating_sub(commit_tokens);
            }

            if remaining_tokens == 0 {
                // If we exhaust the tokens in step 2, clear contents
                Self::clear_contents(context);
                return;
            }
        }

        // Step 3: Allocate any leftover tokens for full file contents (lowest priority)
        for file in &mut context.staged_files {
            if let Some(content) = &mut file.content {
                let content_tokens = self.count_tokens(content);
                if content_tokens > remaining_tokens {
                    *content = self.truncate_string(content, remaining_tokens);
                    remaining_tokens = 0;
                } else {
                    remaining_tokens = remaining_tokens.saturating_sub(content_tokens);
                }

                if remaining_tokens == 0 {
                    return; // Exit early if we've exhausted the token budget
                }
            }
        }
    }

    // Truncate a string to fit within the specified token limit
    #[allow(clippy::unwrap_used)] // todo: handle unwrap
    pub fn truncate_string(&self, s: &str, max_tokens: usize) -> String {
        let tokens = self.encoder.encode_ordinary(s);

        if tokens.len() <= max_tokens {
            return s.to_string();
        }

        let truncation_limit = max_tokens.saturating_sub(1); // Reserve space for the ellipsis
        let mut truncated_tokens = tokens[..truncation_limit].to_vec();
        truncated_tokens.push(self.encoder.encode_ordinary("…")[0]);

        self.encoder.decode(truncated_tokens).unwrap()
    }

    // Clear all recent commits and full file contents
    fn clear_commits_and_contents(context: &mut CommitContext) {
        Self::clear_commits(context);
        Self::clear_contents(context);
    }

    // Clear all recent commits
    fn clear_commits(context: &mut CommitContext) {
        for commit in &mut context.recent_commits {
            commit.message.clear();
        }
    }

    // Clear all full file contents
    fn clear_contents(context: &mut CommitContext) {
        for file in &mut context.staged_files {
            file.content = None;
        }
    }

    // Count the number of tokens in a string
    pub fn count_tokens(&self, s: &str) -> usize {
        let tokens = self.encoder.encode_ordinary(s);
        tokens.len()
    }
}
