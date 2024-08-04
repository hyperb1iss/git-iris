use colored::ColoredString;
use colored::*;
use lazy_static::lazy_static;
use rand::seq::SliceRandom;
use std::collections::HashMap;

lazy_static! {
    static ref RANDOM_MESSAGES: Vec<ColoredString> = vec![
        "🔮 Consulting the cosmic commit oracle...".purple().bold(),
        "🌌 Aligning the celestial code spheres...".blue().bold(),
        "👻 Channeling the spirit of clean commits..."
            .green()
            .bold(),
        "🚀 Launching commit ideas into the coding cosmos..."
            .red()
            .bold(),
        "🌠 Exploring the galaxy of potential messages..."
            .cyan()
            .bold(),
        "🔭 Peering into the commit-verse for inspiration..."
            .yellow()
            .bold(),
        "🧙 Casting a spell for the perfect commit message..."
            .purple()
            .bold(),
        "✨ Harnessing the power of a thousand code stars..."
            .magenta()
            .bold(),
        "🪐 Orbiting the planet of precise git descriptions..."
            .blue()
            .bold(),
        "🎨 Weaving a tapestry of colorful commit prose..."
            .cyan()
            .bold(),
        "🎇 Igniting the fireworks of code brilliance..."
            .red()
            .bold(),
        "🧠 Syncing with the collective coding consciousness..."
            .green()
            .bold(),
        "🌙 Aligning the moon phases for optimal commit clarity..."
            .yellow()
            .bold(),
        "🔬 Analyzing code particles at the quantum level..."
            .purple()
            .bold(),
        "🧬 Decoding the DNA of your changes...".magenta().bold(),
        "🏺 Summoning the ancient spirits of version control..."
            .red()
            .bold(),
        "📡 Tuning into the frequency of flawless commits..."
            .blue()
            .bold(),
        "💎 Charging the commit crystals with cosmic energy..."
            .cyan()
            .bold(),
        "🌍 Translating your changes into universal code..."
            .green()
            .bold(),
        "🧪 Distilling the essence of your modifications..."
            .yellow()
            .bold(),
        "🕸️ Unraveling the threads of your code tapestry..."
            .purple()
            .bold(),
        "🦉 Consulting the all-knowing git guardians..."
            .blue()
            .bold(),
        "🎵 Harmonizing with the rhythms of the coding universe..."
            .magenta()
            .bold(),
        "🌊 Diving into the depths of the code ocean..."
            .cyan()
            .bold(),
        "🧓 Seeking wisdom from the repository sages..."
            .green()
            .bold(),
        "🧭 Calibrating the commit compass for true north..."
            .yellow()
            .bold(),
        "🔐 Unlocking the secrets of the commit constellations..."
            .purple()
            .bold(),
        "⭐ Gathering stardust for your stellar commit..."
            .magenta()
            .bold(),
        "🔎 Focusing the lens of the code telescope..."
            .blue()
            .bold(),
        "🏄 Riding the waves of inspiration through the code cosmos..."
            .cyan()
            .bold(),
    ];
    static ref CALLBACK_MESSAGES: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();
        m.insert("analyze_branch", "🔍 Analyzing current branch...");
        m.insert("fetch_commits", "📜 Fetching recent commits...");
        m.insert("analyze_files", "📊 Analyzing file statuses...");
        m.insert("extract_metadata", "📦 Extracting project metadata...");
        m.insert("optimize_context", "✨ Optimizing context...");
        m.insert("process_file", "📄 Processing file {} of {}");
        m.insert("update_config", "🛠️ Updating configuration...");
        m
    };
}

pub fn get_random_message() -> String {
    RANDOM_MESSAGES
        .choose(&mut rand::thread_rng())
        .unwrap()
        .to_string()
}

pub fn get_callback_message(key: &str) -> ColoredString {
    CALLBACK_MESSAGES
        .get(key)
        .copied()
        .unwrap_or("Unknown message key")
        .cyan()
        .bold()
}

pub fn format_callback_message(key: &str, args: &[String]) -> ColoredString {
    let message = CALLBACK_MESSAGES
        .get(key)
        .copied()
        .unwrap_or("Unknown message key");

    let formatted = if args.is_empty() {
        message.to_string()
    } else {
        match args.len() {
            1 => format!("{}", args[0]),
            2 => format!("{} {}", args[0], args[1]),
            _ => message
                .replace("{}", "{}")
                .replacen("{}", &args[0], 1)
                .replacen("{}", &args.get(1).map(String::as_str).unwrap_or(""), 1),
        }
    };

    formatted.cyan().bold()
}

pub fn format_progress(current: usize, total: usize) -> ColoredString {
    format!("{} of {}", current, total).yellow()
}
