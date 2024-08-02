use rand::seq::SliceRandom;

pub fn get_random_message() -> String {
    let messages = vec![
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
    messages
        .choose(&mut rand::thread_rng())
        .unwrap()
        .to_string()
}
