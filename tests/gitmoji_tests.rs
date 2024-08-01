use git_iris::gitmoji::{apply_gitmoji, get_gitmoji, get_gitmoji_list};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_gitmoji() {
        assert_eq!(
            apply_gitmoji("feat: add new feature"),
            "✨ feat: add new feature"
        );
        assert_eq!(apply_gitmoji("fix: resolve bug"), "🐛 fix: resolve bug");
        assert_eq!(
            apply_gitmoji("docs: update documentation"),
            "📝 docs: update documentation"
        );
        assert_eq!(apply_gitmoji("style: format code"), "💄 style: format code");
        assert_eq!(
            apply_gitmoji("refactor: improve code structure"),
            "♻️ refactor: improve code structure"
        );
        assert_eq!(
            apply_gitmoji("test: add unit tests"),
            "✅ test: add unit tests"
        );
        assert_eq!(
            apply_gitmoji("chore: update dependencies"),
            "🔨 chore: update dependencies"
        );
        assert_eq!(
            apply_gitmoji("unknown: some message"),
            "unknown: some message"
        );
        assert_eq!(apply_gitmoji(""), "");
        assert_eq!(apply_gitmoji("no_colon_here"), "no_colon_here");
    }

    #[test]
    fn test_get_gitmoji_list() {
        let list = get_gitmoji_list();
        assert!(list.contains("✨ - :feat: - Introduce new features"));
        assert!(list.contains("🐛 - :fix: - Fix a bug"));
        assert!(list.contains("📝 - :docs: - Add or update documentation"));
        assert!(list.contains("💄 - :style: - Add or update the UI and style files"));
        assert!(list.contains("♻️ - :refactor: - Refactor code"));
        assert!(list.contains("✅ - :test: - Add or update tests"));
        assert!(list.contains("🔨 - :chore: - Other changes that don't modify src or test files"));
        // Add more assertions for other gitmojis as needed
    }

    #[test]
    fn test_get_gitmoji() {
        assert_eq!(get_gitmoji("feat"), Some("✨"));
        assert_eq!(get_gitmoji("fix"), Some("🐛"));
        assert_eq!(get_gitmoji("docs"), Some("📝"));
        assert_eq!(get_gitmoji("style"), Some("💄"));
        assert_eq!(get_gitmoji("refactor"), Some("♻️"));
        assert_eq!(get_gitmoji("test"), Some("✅"));
        assert_eq!(get_gitmoji("chore"), Some("🔨"));
        assert_eq!(get_gitmoji("unknown"), None);
    }
}
