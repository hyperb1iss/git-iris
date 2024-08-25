use crate::ui::{
    AURORA_GREEN, CELESTIAL_BLUE, COMET_ORANGE, GALAXY_PINK, METEOR_RED, NEBULA_PURPLE,
    PLASMA_CYAN, SOLAR_YELLOW, STARLIGHT,
};
use lazy_static::lazy_static;
use rand::seq::SliceRandom;
use ratatui::style::Color;

#[derive(Clone)]
pub struct ColoredMessage {
    pub text: String,
    pub color: Color,
}

lazy_static! {
    static ref WAITING_MESSAGES: Vec<ColoredMessage> = vec![
        ColoredMessage {
            text: "🔮 Consulting the cosmic commit oracle...".to_string(),
            color: NEBULA_PURPLE
        },
        ColoredMessage {
            text: "🌌 Aligning the celestial code spheres...".to_string(),
            color: CELESTIAL_BLUE
        },
        ColoredMessage {
            text: "👻 Channeling the spirit of clean commits...".to_string(),
            color: AURORA_GREEN
        },
        ColoredMessage {
            text: "🚀 Launching commit ideas into the coding cosmos...".to_string(),
            color: METEOR_RED
        },
        ColoredMessage {
            text: "🌠 Exploring the galaxy of potential messages...".to_string(),
            color: PLASMA_CYAN
        },
        ColoredMessage {
            text: "🔭 Peering into the commit-verse for inspiration...".to_string(),
            color: SOLAR_YELLOW
        },
        ColoredMessage {
            text: "🧙 Casting a spell for the perfect commit message...".to_string(),
            color: GALAXY_PINK
        },
        ColoredMessage {
            text: "✨ Harnessing the power of a thousand code stars...".to_string(),
            color: STARLIGHT
        },
        ColoredMessage {
            text: "🪐 Orbiting the planet of precise git descriptions...".to_string(),
            color: CELESTIAL_BLUE
        },
        ColoredMessage {
            text: "🎨 Weaving a tapestry of colorful commit prose...".to_string(),
            color: PLASMA_CYAN
        },
        ColoredMessage {
            text: "🎇 Igniting the fireworks of code brilliance...".to_string(),
            color: COMET_ORANGE
        },
        ColoredMessage {
            text: "🧠 Syncing with the collective coding consciousness...".to_string(),
            color: AURORA_GREEN
        },
        ColoredMessage {
            text: "🌙 Aligning the moon phases for optimal commit clarity...".to_string(),
            color: STARLIGHT
        },
        ColoredMessage {
            text: "🔬 Analyzing code particles at the quantum level...".to_string(),
            color: NEBULA_PURPLE
        },
        ColoredMessage {
            text: "🧬 Decoding the DNA of your changes...".to_string(),
            color: GALAXY_PINK
        },
        ColoredMessage {
            text: "🏺 Summoning the ancient spirits of version control...".to_string(),
            color: METEOR_RED
        },
        ColoredMessage {
            text: "📡 Tuning into the frequency of flawless commits...".to_string(),
            color: CELESTIAL_BLUE
        },
        ColoredMessage {
            text: "💎 Charging the commit crystals with cosmic energy...".to_string(),
            color: PLASMA_CYAN
        },
        ColoredMessage {
            text: "🌍 Translating your changes into universal code...".to_string(),
            color: AURORA_GREEN
        },
        ColoredMessage {
            text: "🧪 Distilling the essence of your modifications...".to_string(),
            color: SOLAR_YELLOW
        },
        ColoredMessage {
            text: "🕸️ Unraveling the threads of your code tapestry...".to_string(),
            color: NEBULA_PURPLE
        },
        ColoredMessage {
            text: "🦉 Consulting the all-knowing git guardians...".to_string(),
            color: CELESTIAL_BLUE
        },
        ColoredMessage {
            text: "🎵 Harmonizing with the rhythms of the coding universe...".to_string(),
            color: GALAXY_PINK
        },
        ColoredMessage {
            text: "🌊 Diving into the depths of the code ocean...".to_string(),
            color: PLASMA_CYAN
        },
        ColoredMessage {
            text: "🧓 Seeking wisdom from the repository sages...".to_string(),
            color: AURORA_GREEN
        },
        ColoredMessage {
            text: "🧭 Calibrating the commit compass for true north...".to_string(),
            color: SOLAR_YELLOW
        },
        ColoredMessage {
            text: "🔐 Unlocking the secrets of the commit constellations...".to_string(),
            color: NEBULA_PURPLE
        },
        ColoredMessage {
            text: "⭐ Gathering stardust for your stellar commit...".to_string(),
            color: STARLIGHT
        },
        ColoredMessage {
            text: "🔎 Focusing the lens of the code telescope...".to_string(),
            color: CELESTIAL_BLUE
        },
        ColoredMessage {
            text: "🏄 Riding the waves of inspiration through the code cosmos...".to_string(),
            color: PLASMA_CYAN
        },
    ];
    static ref USER_MESSAGES: Vec<ColoredMessage> = vec![
        ColoredMessage {
            text: "🚀 Launching commit rocket".to_string(),
            color: METEOR_RED
        },
        ColoredMessage {
            text: "🌟 Illuminating code cosmos".to_string(),
            color: STARLIGHT
        },
        ColoredMessage {
            text: "🔭 Observing code constellations".to_string(),
            color: CELESTIAL_BLUE
        },
        ColoredMessage {
            text: "🧙‍♂️ Weaving code enchantments".to_string(),
            color: GALAXY_PINK
        },
        ColoredMessage {
            text: "⚛️ Splitting code atoms".to_string(),
            color: PLASMA_CYAN
        },
        ColoredMessage {
            text: "🌈 Painting commit rainbows".to_string(),
            color: AURORA_GREEN
        },
        ColoredMessage {
            text: "🔑 Unlocking git portals".to_string(),
            color: SOLAR_YELLOW
        },
        ColoredMessage {
            text: "🎭 Staging code drama".to_string(),
            color: COMET_ORANGE
        },
        ColoredMessage {
            text: "🌌 Expanding code universe".to_string(),
            color: NEBULA_PURPLE
        },
        ColoredMessage {
            text: "🏹 Aiming commit arrows".to_string(),
            color: METEOR_RED
        },
        ColoredMessage {
            text: "🎨 Brushing commit strokes".to_string(),
            color: PLASMA_CYAN
        },
        ColoredMessage {
            text: "🌱 Growing code forests".to_string(),
            color: AURORA_GREEN
        },
        ColoredMessage {
            text: "🧩 Assembling code puzzle".to_string(),
            color: GALAXY_PINK
        },
        ColoredMessage {
            text: "🎶 Orchestrating commit symphony".to_string(),
            color: CELESTIAL_BLUE
        },
        ColoredMessage {
            text: "⚖️ Balancing code forces".to_string(),
            color: SOLAR_YELLOW
        },
    ];
}

#[allow(clippy::unwrap_used)] // todo: handle unwrap
pub fn get_waiting_message() -> ColoredMessage {
    WAITING_MESSAGES
        .choose(&mut rand::thread_rng())
        .unwrap()
        .clone()
}

#[allow(clippy::unwrap_used)] // todo: handle unwrap
pub fn get_user_message() -> ColoredMessage {
    USER_MESSAGES
        .choose(&mut rand::thread_rng())
        .unwrap()
        .clone()
}
