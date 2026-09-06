use anyhow::{Context, Result, bail};

use crate::{agents::TaskContext, git::GitRepo};

/// Pull-request revisions captured before analysis and checked before publishing.
#[derive(Debug, Clone)]
pub struct ReviewTarget {
    pub base_ref: String,
    pub base_sha: String,
    pub head_sha: String,
}

impl ReviewTarget {
    pub(super) fn validate(&self, base_ref: &str, base_sha: &str, head: &str) -> Result<()> {
        // A base advance can change the three-dot diff even when the PR head is unchanged.
        if self.base_ref != base_ref || self.base_sha != base_sha || self.head_sha != head {
            bail!("Pull request changed during analysis. Generate a new review before publishing.");
        }
        Ok(())
    }

    /// Resolve local refs to immutable commits matching the publication target.
    pub fn pin_context(&self, repo: &GitRepo, context: TaskContext) -> Result<TaskContext> {
        let repo = git2::Repository::open(repo.repo_path())?;
        let resolve = |reference: &str| -> Result<git2::Oid> {
            Ok(repo.revparse_single(reference)
                .with_context(|| format!("Review commit {reference} is unavailable locally; fetch the pull request first"))?
                .peel_to_commit()?.id())
        };
        let head = resolve(&self.head_sha)?;
        let pinned = match context {
            TaskContext::Staged {
                include_unstaged: false,
            } => {
                let base = resolve(&self.base_sha)?;
                let merge_base = repo.merge_base(base, head)?;
                TaskContext::Range {
                    from: merge_base.to_string(),
                    to: head.to_string(),
                }
            }
            TaskContext::Commit { commit_id } => {
                let commit = resolve(&commit_id)?;
                if commit != head {
                    bail!(
                        "Selected commit does not match the pull-request head. Review the current PR head before publishing."
                    );
                }
                TaskContext::Commit {
                    commit_id: commit.to_string(),
                }
            }
            TaskContext::Range { from, to } => {
                if resolve(&to)? != head {
                    bail!(
                        "Review --to does not match the pull-request head. Review the current PR head before publishing."
                    );
                }
                TaskContext::Range {
                    from: resolve(&from)?.to_string(),
                    to: head.to_string(),
                }
            }
            _ => bail!(
                "Uncommitted changes cannot be attached to a GitHub commit review. Commit and push them, or review locally without --github-review."
            ),
        };
        Ok(pinned)
    }
}
