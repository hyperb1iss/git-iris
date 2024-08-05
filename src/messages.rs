use colored::ColoredString;
use colored::*;
use lazy_static::lazy_static;
use rand::seq::SliceRandom;

lazy_static! {
    static ref RANDOM_MESSAGES: Vec<ColoredString> = vec![
        "🔮 Consulting the cosmic commit oracle..."
            .purple()
            .bold(),
        "🌌 Aligning the celestial code spheres..."
            .blue()
            .bold(),
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
        "🧬 Decoding the DNA of your changes..."
            .magenta()
            .bold(),
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
}

pub fn get_random_message() -> String {
    RANDOM_MESSAGES
        .choose(&mut rand::thread_rng())
        .unwrap()
        .to_string()
}

