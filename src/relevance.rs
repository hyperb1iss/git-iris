use crate::context::{ChangeType, CommitContext};
use std::collections::HashMap;

pub struct RelevanceScorer {
    scorers: Vec<Box<dyn Scorer>>,
}

trait Scorer {
    fn score(&self, context: &CommitContext) -> HashMap<String, f32>;
}

struct FileTypeScorer;
impl Scorer for FileTypeScorer {
    fn score(&self, context: &CommitContext) -> HashMap<String, f32> {
        let mut scores = HashMap::new();
        for file in &context.staged_files {
            let score = match file.path.split('.').last() {
                Some("rs") => 1.0,
                Some("js" | "ts") => 0.9,
                Some("py") => 0.8,
                _ => 0.5,
            };
            scores.insert(file.path.clone(), score);
        }
        scores
    }
}

struct ChangeTypeScorer;
impl Scorer for ChangeTypeScorer {
    fn score(&self, context: &CommitContext) -> HashMap<String, f32> {
        let mut scores = HashMap::new();
        for file in &context.staged_files {
            let score = match file.change_type {
                ChangeType::Added => 0.9,
                ChangeType::Modified => 1.0,
                ChangeType::Deleted => 0.7,
            };
            scores.insert(file.path.clone(), score);
        }
        scores
    }
}

impl RelevanceScorer {
    pub fn new() -> Self {
        RelevanceScorer {
            scorers: vec![Box::new(FileTypeScorer), Box::new(ChangeTypeScorer)],
        }
    }

    pub fn score(&self, context: &CommitContext) -> HashMap<String, f32> {
        let mut final_scores = HashMap::new();
        for scorer in &self.scorers {
            let scores = scorer.score(context);
            for (key, value) in scores {
                *final_scores.entry(key).or_insert(0.0) += value;
            }
        }
        final_scores
    }
}
