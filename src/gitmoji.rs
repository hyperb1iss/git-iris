use std::collections::HashMap;

fn create_gitmoji_map() -> HashMap<&'static str, (&'static str, &'static str)> {
    let mut m = HashMap::new();

    m.insert("adhesive_bandage", ("🩹", "Simple fix for a non-critical issue"));
    m.insert("alembic", ("⚗️", "Perform experiments"));
    m.insert("alien", ("👽️", "Update code due to external API changes"));
    m.insert("ambulance", ("🚑️", "Critical hotfix"));
    m.insert("arrow_down", ("⬇️", "Downgrade dependencies"));
    m.insert("arrow_up", ("⬆️", "Upgrade dependencies"));
    m.insert("art", ("🎨", "Improve structure / format of the code"));
    m.insert("beers", ("🍻", "Write code drunkenly"));
    m.insert("bento", ("🍱", "Add or update assets"));
    m.insert("bookmark", ("🔖", "Release / Version tags"));
    m.insert("boom", ("💥", "Introduce breaking changes"));
    m.insert("bricks", ("🧱", "Infrastructure related changes"));
    m.insert("bug", ("🐛", "Fix a bug"));
    m.insert("building_construction", ("🏗️", "Make architectural changes"));
    m.insert("bulb", ("💡", "Add or update comments in source code"));
    m.insert("busts_in_silhouette", ("👥", "Add or update contributor(s)"));
    m.insert("camera_flash", ("📸", "Add or update snapshots"));
    m.insert("card_file_box", ("🗃️", "Perform database related changes"));
    m.insert("chart_with_upwards_trend", ("📈", "Add or update analytics or track code"));
    m.insert("children_crossing", ("🚸", "Improve user experience / usability"));
    m.insert("closed_lock_with_key", ("🔐", "Add or update secrets"));
    m.insert("clown_face", ("🤡", "Mock things"));
    m.insert("coffin", ("⚰️", "Remove dead code"));
    m.insert("construction", ("🚧", "Work in progress"));
    m.insert("construction_worker", ("👷", "Add or update CI build system"));
    m.insert("dizzy", ("💫", "Add or update animations and transitions"));
    m.insert("egg", ("🥚", "Add or update an easter egg"));
    m.insert("fire", ("🔥", "Remove code or files"));
    m.insert("globe_with_meridians", ("🌐", "Internationalization and localization"));
    m.insert("goal_net", ("🥅", "Catch errors"));
    m.insert("green_heart", ("💚", "Fix CI Build"));
    m.insert("hammer", ("🔨", "Add or update development scripts"));
    m.insert("heavy_minus_sign", ("➖", "Remove a dependency"));
    m.insert("heavy_plus_sign", ("➕", "Add a dependency"));
    m.insert("iphone", ("📱", "Work on responsive design"));
    m.insert("label", ("🏷️", "Add or update types"));
    m.insert("lipstick", ("💄", "Add or update the UI and style files"));
    m.insert("lock", ("🔒️", "Fix security or privacy issues"));
    m.insert("loud_sound", ("🔊", "Add or update logs"));
    m.insert("mag", ("🔍️", "Improve SEO"));
    m.insert("memo", ("📝", "Add or update documentation"));
    m.insert("money_with_wings", ("💸", "Add sponsorships or money related infrastructure"));
    m.insert("monocle_face", ("🧐", "Data exploration/inspection"));
    m.insert("mute", ("🔇", "Remove logs"));
    m.insert("necktie", ("👔", "Add or update business logic"));
    m.insert("package", ("📦️", "Add or update compiled files or packages"));
    m.insert("page_facing_up", ("📄", "Add or update license"));
    m.insert("passport_control", ("🛂", "Work on code related to authorization, roles and permissions"));
    m.insert("pencil2", ("✏️", "Fix typos"));
    m.insert("poop", ("💩", "Write bad code that needs to be improved"));
    m.insert("pushpin", ("📌", "Pin dependencies to specific versions"));
    m.insert("recycle", ("♻️", "Refactor code"));
    m.insert("rewind", ("⏪️", "Revert changes"));
    m.insert("rocket", ("🚀", "Deploy stuff"));
    m.insert("rotating_light", ("🚨", "Fix compiler / linter warnings"));
    m.insert("safety_vest", ("🦺", "Add or update code related to validation"));
    m.insert("see_no_evil", ("🙈", "Add or update a .gitignore file"));
    m.insert("seedling", ("🌱", "Add or update seed files"));
    m.insert("sparkles", ("✨", "Introduce new features"));
    m.insert("speech_balloon", ("💬", "Add or update text and literals"));
    m.insert("stethoscope", ("🩺", "Add or update healthcheck"));
    m.insert("tada", ("🎉", "Begin a project"));
    m.insert("technologist", ("🧑‍💻", "Improve developer experience"));
    m.insert("test_tube", ("🧪", "Add a failing test"));
    m.insert("thread", ("🧵", "Add or update code related to multithreading or concurrency"));
    m.insert("triangular_flag_on_post", ("🚩", "Add, update, or remove feature flags"));
    m.insert("truck", ("🚚", "Move or rename resources (e.g.: files, paths, routes)"));
    m.insert("twisted_rightwards_arrows", ("🔀", "Merge branches"));
    m.insert("wastebasket", ("🗑️", "Deprecate code that needs to be cleaned up"));
    m.insert("wheelchair", ("♿️", "Improve accessibility"));
    m.insert("white_check_mark", ("✅", "Add, update, or pass tests"));
    m.insert("wrench", ("🔧", "Add or update configuration files"));
    m.insert("zap", ("⚡️", "Improve performance"));

    m.insert("chore", ("🔧", "Add or update configuration files"));
    m.insert("docs", ("📝", "Add or update documentation"));
    m.insert("feat", ("✨", "Introduce new features"));
    m.insert("fix", ("🐛", "Fix a bug"));
    m.insert("perf", ("⚡️", "Improve performance"));
    m.insert("refactor", ("♻️", "Refactor code"));
    m.insert("style", ("💄", "Add or update the UI and style files"));
    m.insert("test", ("✅", "Add, update, or pass tests"));
    
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
