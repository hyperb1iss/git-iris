use crate::context::{ChangeType, StagedFile};
use crate::file_analyzers::get_analyzer;
use anyhow::Result;
use git2::{Commit, DiffDelta, Repository};

pub struct ChangeAnalyzer<'a> {
    repo: &'a Repository,
}

impl<'a> ChangeAnalyzer<'a> {
    pub fn new(repo: &'a Repository) -> Self {
        Self { repo }
    }

    pub fn analyze_commit(&self, commit: &Commit) -> Result<AnalyzedChange> {
        let parent = commit.parent(0).ok();
        let diff = self.repo.diff_tree_to_tree(
            parent.as_ref().map(|c| c.tree().ok()).flatten().as_ref(),
            Some(&commit.tree()?),
            None,
        )?;

        let mut file_changes = Vec::new();
        diff.foreach(
            &mut |delta: DiffDelta, _: f32| {
                if let Some(file_change) = self.analyze_file_change(&delta) {
                    file_changes.push(file_change);
                }
                true
            },
            None,
            None,
            None,
        )?;

        let stats = diff.stats()?;
        let impact_score = self.calculate_impact_score(&stats, &file_changes);

        Ok(AnalyzedChange {
            commit_hash: commit.id().to_string(),
            commit_message: commit.message().unwrap_or("").to_string(),
            author: commit.author().name().unwrap_or("").to_string(),
            file_changes,
            insertions: stats.insertions(),
            deletions: stats.deletions(),
            files_changed: stats.files_changed(),
            impact_score,
        })
    }

    fn analyze_file_change(&self, delta: &DiffDelta) -> Option<FileChange> {
        let old_file = delta.old_file().path()?;
        let new_file = delta.new_file().path()?;

        let change_type = match delta.status() {
            git2::Delta::Added => ChangeType::Added,
            git2::Delta::Deleted => ChangeType::Deleted,
            _ => ChangeType::Modified,
        };

        let analyzer = get_analyzer(new_file.to_str()?);
        let staged_file = StagedFile {
            path: new_file.to_str()?.to_string(),
            change_type: change_type.clone(),
            diff: self.get_file_diff(delta).unwrap_or_default(),
            analysis: Vec::new(),
            content_excluded: false,
        };

        let analysis = analyzer.analyze(new_file.to_str()?, &staged_file);

        Some(FileChange {
            old_path: old_file.to_str()?.to_string(),
            new_path: new_file.to_str()?.to_string(),
            change_type,
            analysis,
        })
    }

    fn get_file_diff(&self, _delta: &DiffDelta) -> Result<String> {
        let mut diff_content = String::new();
        let diff = self.repo.diff_tree_to_tree(None, None, None)?;
        diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
            let prefix = match line.origin() {
                '+' => "+",
                '-' => "-",
                _ => " ",
            };
            diff_content.push_str(&format!(
                "{}{}",
                prefix,
                std::str::from_utf8(line.content()).unwrap()
            ));
            true
        })?;
        Ok(diff_content)
    }

    fn calculate_impact_score(&self, stats: &git2::DiffStats, file_changes: &[FileChange]) -> f32 {
        let base_score = (stats.insertions() + stats.deletions()) as f32 / 100.0;
        let file_score = file_changes.len() as f32 / 10.0;
        let analysis_score = file_changes
            .iter()
            .map(|fc| fc.analysis.len() as f32 / 5.0)
            .sum::<f32>();

        base_score + file_score + analysis_score
    }
}

pub struct AnalyzedChange {
    pub commit_hash: String,
    pub commit_message: String,
    pub author: String,
    pub file_changes: Vec<FileChange>,
    pub insertions: usize,
    pub deletions: usize,
    pub files_changed: usize,
    pub impact_score: f32,
}

pub struct FileChange {
    pub old_path: String,
    pub new_path: String,
    pub change_type: ChangeType,
    pub analysis: Vec<String>,
}
