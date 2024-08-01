use std::collections::HashMap;

fn create_gitmoji_map() -> HashMap<&'static str, (&'static str, &'static str)> {
    let mut m = HashMap::new();

    m.insert("feat", ("✨", "Introduce new features"));
    m.insert("fix", ("🐛", "Fix a bug"));
    m.insert("docs", ("📝", "Add or update documentation"));
    m.insert("style", ("💄", "Add or update the UI and style files"));
    m.insert("refactor", ("♻️", "Refactor code"));
    m.insert("perf", ("⚡️", "Improve performance"));
    m.insert("test", ("✅", "Add or update tests"));
    m.insert("build", ("👷", "Add or update build scripts"));
    m.insert("ci", ("🔧", "Add or update CI configuration"));
    m.insert("chore", ("🔨", "Other changes that don't modify src or test files"));
    m.insert("revert", ("⏪️", "Revert changes"));
    m.insert("wip", ("🚧", "Work in progress"));
    m.insert("dependencies", ("⬆️", "Update dependencies"));
    m.insert("remove", ("🔥", "Remove code or files"));
    m.insert("i18n", ("🌐", "Internationalization and localization"));
    m.insert("security", ("🔒️", "Fix security issues"));
    m.insert("debug", ("🐛", "Add or update debugging code"));
    m.insert("deployment", ("🚀", "Deploy stuff"));
    m.insert("hotfix", ("🚑", "Critical hotfix"));
    m.insert("accessibility", ("♿", "Improve accessibility"));
    m.insert("analytics", ("📈", "Add or update analytics"));
    m.insert("seo", ("🔍️", "Improve SEO"));
    m.insert("config", ("🔧", "Add or update configuration files"));
    m.insert("tracking", ("📈", "Add or update tracking code"));
    m.insert("design", ("🎨", "Improve structure / format of the code"));
    m.insert("error", ("🚨", "Fix compiler / linter warnings"));
    m.insert("test_failure", ("💥", "Fix tests or CI failures"));
    m.insert("data", ("📊", "Add or update data"));
    m.insert("content", ("📝", "Add or update content"));
    m.insert("linter", ("👕", "Add or update linters"));
    m.insert("initial", ("🎉", "Begin a project"));

    m
}

lazy_static::lazy_static! {
    static ref GITMOJI_MAP: HashMap<&'static str, (&'static str, &'static str)> = create_gitmoji_map();
}

pub fn get_gitmoji(commit_type: &str) -> Option<&'static str> {
    GITMOJI_MAP.get(commit_type).map(|&(emoji, _)| emoji)
}

pub fn apply_gitmoji(commit_message: &str) -> String {
    let parts: Vec<&str> = commit_message.splitn(2, ':').collect();
    if parts.len() == 2 {
        if let Some((gitmoji, _)) = GITMOJI_MAP.get(parts[0].trim()) {
            return format!("{} {}: {}", gitmoji, parts[0].trim(), parts[1].trim());
        }
    }
    commit_message.to_string()
}

pub fn get_gitmoji_list() -> String {
    GITMOJI_MAP
        .iter()
        .map(|(key, (emoji, description))| format!("{} - :{}: - {}", emoji, key, description))
        .collect::<Vec<String>>()
        .join("\n")
}
