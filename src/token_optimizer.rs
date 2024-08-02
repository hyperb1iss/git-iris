use crate::context::{CommitContext, RecentCommit, StagedFile};
use tiktoken_rs::cl100k_base;

pub struct TokenOptimizer {
    encoder: tiktoken_rs::CoreBPE,
    max_tokens: usize,
}

impl TokenOptimizer {
    pub fn new(max_tokens: usize) -> Self {
        TokenOptimizer {
            encoder: cl100k_base().unwrap(),
            max_tokens,
        }
    }

    pub fn optimize_context(&self, context: &mut CommitContext) {
        let total_tokens = self.count_total_tokens(context);
        if total_tokens <= self.max_tokens {
            return;
        }

        let commit_tokens = self.max_tokens / 3;
        let staged_tokens = self.max_tokens / 3;
        let unstaged_tokens = self.max_tokens - commit_tokens - staged_tokens;

        self.truncate_recent_commits(&mut context.recent_commits, commit_tokens);
        self.truncate_staged_files(&mut context.staged_files, staged_tokens);
        self.truncate_unstaged_files(&mut context.unstaged_files, unstaged_tokens);

        // Ensure we don't exceed the max tokens
        while self.count_total_tokens(context) > self.max_tokens {
            if !context.unstaged_files.is_empty() {
                context.unstaged_files.pop();
            } else if !context.staged_files.is_empty() {
                context.staged_files.pop();
            } else if !context.recent_commits.is_empty() {
                context.recent_commits.pop();
            } else {
                break;
            }
        }
    }

    fn count_total_tokens(&self, context: &CommitContext) -> usize {
        let commit_tokens: usize = context
            .recent_commits
            .iter()
            .map(|c| self.count_tokens(&c.message))
            .sum();
        let staged_tokens: usize = context
            .staged_files
            .iter()
            .map(|f| self.count_tokens(&f.diff))
            .sum();
        let unstaged_tokens: usize = context
            .unstaged_files
            .iter()
            .map(|f| self.count_tokens(f))
            .sum();
        commit_tokens + staged_tokens + unstaged_tokens
    }

    fn truncate_recent_commits(&self, commits: &mut Vec<RecentCommit>, max_tokens: usize) {
        let mut total_tokens = 0;
        commits.retain_mut(|commit| {
            if total_tokens >= max_tokens {
                return false;
            }
            let available_tokens = max_tokens.saturating_sub(total_tokens);
            commit.message = self.truncate_string(&commit.message, available_tokens);
            total_tokens += self.count_tokens(&commit.message);
            true
        });
    }

    fn truncate_staged_files(&self, files: &mut Vec<StagedFile>, max_tokens: usize) {
        let mut total_tokens = 0;
        files.retain_mut(|file| {
            if total_tokens >= max_tokens {
                return false;
            }
            let available_tokens = max_tokens.saturating_sub(total_tokens);
            file.diff = self.truncate_string(&file.diff, available_tokens);
            total_tokens += self.count_tokens(&file.diff);
            true
        });
    }

    fn truncate_unstaged_files(&self, files: &mut Vec<String>, max_tokens: usize) {
        let mut total_tokens = 0;
        files.retain_mut(|file| {
            if total_tokens >= max_tokens {
                return false;
            }
            let available_tokens = max_tokens.saturating_sub(total_tokens);
            *file = self.truncate_string(file, available_tokens);
            total_tokens += self.count_tokens(file);
            true
        });
    }

    pub fn truncate_string(&self, s: &str, max_tokens: usize) -> String {
        let tokens = self.encoder.encode_ordinary(s);
        if tokens.len() <= max_tokens {
            s.to_string()
        } else {
            let mut truncated_tokens = tokens[..max_tokens.saturating_sub(1)].to_vec();
            truncated_tokens.push(self.encoder.encode_ordinary("…")[0]);
            self.encoder.decode(truncated_tokens).unwrap()
        }
    }

    pub fn count_tokens(&self, s: &str) -> usize {
        self.encoder.encode_ordinary(s).len()
    }
}
