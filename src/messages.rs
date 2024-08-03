use colored::ColoredString;
use colored::*;
use lazy_static::lazy_static;
use rand::seq::SliceRandom;
use std::collections::HashMap;

lazy_static! {
    static ref RANDOM_MESSAGES: Vec<&'static str> = vec![
        "🔮 Consulting the cosmic commit oracle...",
        "🌌 Aligning the celestial code spheres...",
        "👻 Channeling the spirit of clean commits...",
        "🚀 Launching commit ideas into the coding cosmos...",
        "🌠 Exploring the galaxy of potential messages...",
        "🔭 Peering into the commit-verse for inspiration...",
        "🧙 Casting a spell for the perfect commit message...",
        "✨ Harnessing the power of a thousand code stars...",
        "🪐 Orbiting the planet of precise git descriptions...",
        "🎨 Weaving a tapestry of colorful commit prose...",
        "🎇 Igniting the fireworks of code brilliance...",
        "🧠 Syncing with the collective coding consciousness...",
        "🌙 Aligning the moon phases for optimal commit clarity...",
        "🔬 Analyzing code particles at the quantum level...",
        "🧬 Decoding the DNA of your changes...",
        "🏺 Summoning the ancient spirits of version control...",
        "📡 Tuning into the frequency of flawless commits...",
        "💎 Charging the commit crystals with cosmic energy...",
        "🌍 Translating your changes into universal code...",
        "🧪 Distilling the essence of your modifications...",
        "🕸️ Unraveling the threads of your code tapestry...",
        "🦉 Consulting the all-knowing git guardians...",
        "🎵 Harmonizing with the rhythms of the coding universe...",
        "🌊 Diving into the depths of the code ocean...",
        "🧓 Seeking wisdom from the repository sages...",
        "🧭 Calibrating the commit compass for true north...",
        "🔐 Unlocking the secrets of the commit constellations...",
        "⭐ Gathering stardust for your stellar commit...",
        "🔎 Focusing the lens of the code telescope...",
        "🏄 Riding the waves of inspiration through the code cosmos...",
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
