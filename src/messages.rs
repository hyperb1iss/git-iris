use crate::ui::*;
use lazy_static::lazy_static;
use rand::seq::SliceRandom;
use ratatui::style::Color;

#[derive(Clone)]
pub struct ColoredMessage {
    pub text: String,
    pub color: Color,
}

lazy_static! {
    static ref RANDOM_MESSAGES: Vec<ColoredMessage> = vec![
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
}

pub fn get_random_message() -> ColoredMessage {
    RANDOM_MESSAGES
        .choose(&mut rand::thread_rng())
        .unwrap()
        .clone()
}
