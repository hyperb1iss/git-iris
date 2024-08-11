use crate::common::CommonParams;

#[derive(Clone, Debug)]
pub struct CommitConfig {
    pub common: CommonParams,
    pub auto_commit: bool,
    pub use_gitmoji: bool,
    pub print: bool,
}