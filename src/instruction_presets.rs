use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstructionPreset {
    pub name: String,
    pub description: String,
    pub instructions: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstructionPresetLibrary {
    presets: HashMap<String, InstructionPreset>,
}

impl InstructionPresetLibrary {
    pub fn new() -> Self {
        let mut presets = HashMap::new();

        presets.insert(
            "default".to_string(),
            InstructionPreset {
                name: "Default".to_string(),
                description: "Standard commit message style".to_string(),
                instructions: "".to_string(),
            },
        );

        presets.insert(
            "detailed".to_string(),
            InstructionPreset {
                name: "Detailed".to_string(),
                description: "Provide more context and explanation in commit messages".to_string(),
                instructions: "Include detailed explanations of why the changes were made, potential impacts, and any related issues or tickets.".to_string(),
            },
        );

        presets.insert(
            "concise".to_string(),
            InstructionPreset {
                name: "Concise".to_string(),
                description: "Short and to-the-point commit messages".to_string(),
                instructions: "Keep commit messages brief and focused on the core change. Avoid unnecessary details.".to_string(),
            },
        );

        presets.insert(
            "conventional".to_string(),
            InstructionPreset {
                name: "Conventional Commits".to_string(),
                description: "Follow the Conventional Commits specification".to_string(),
                instructions: "Use the Conventional Commits format: <type>[optional scope]: <description>. Types include feat, fix, docs, style, refactor, perf, test, and chore.".to_string(),
            },
        );

        presets.insert(
            "storyteller".to_string(),
            InstructionPreset {
                name: "Storyteller".to_string(),
                description: "Frame commits as part of an ongoing story".to_string(),
                instructions: "Write commit messages as if they're part of an epic tale. Use narrative elements to describe the changes, their purpose, and their place in the project's journey.".to_string(),
            },
        );

        presets.insert(
            "technical".to_string(),
            InstructionPreset {
                name: "Technical".to_string(),
                description: "Focus on technical details in commit messages".to_string(),
                instructions: "Emphasize technical aspects of the changes. Include specific function names, algorithms used, or performance impacts where relevant.".to_string(),
            },
        );

        presets.insert(
            "emoji-lover".to_string(),
            InstructionPreset {
                name: "Emoji Lover".to_string(),
                description: "Use plenty of emojis in commit messages".to_string(),
                instructions: "Sprinkle relevant emojis throughout the commit message to add visual flair and quickly convey the nature of the changes.".to_string(),
            },
        );

        presets.insert(
            "haiku".to_string(),
            InstructionPreset {
                name: "Haiku".to_string(),
                description: "Write commit messages in haiku form".to_string(),
                instructions: "Compose the commit message as a haiku (5-7-5 syllable structure). Capture the essence of the change in this poetic form.".to_string(),
            },
        );

        presets.insert(
            "cinematic".to_string(),
            InstructionPreset {
                name: "Cinematic".to_string(),
                description: "Describe commits as if they're scenes from a movie".to_string(),
                instructions: "Frame each commit as a scene in a movie. Use dramatic language, describe the 'setting' (codebase state), and treat code changes as character actions or plot developments.".to_string(),
            },
        );

        presets.insert(
            "pirate".to_string(),
            InstructionPreset {
                name: "Pirate".to_string(),
                description: "Arrr! Write commit messages like a seafaring pirate".to_string(),
                instructions: "Craft ye commit message in the manner of a salty sea dog. Use pirate lingo, nautical metaphors, and a hearty dose of 'arrr's to describe yer code changes, ye scurvy bilge rat!".to_string(),
            },
        );

        // New mystical preset
        presets.insert(
            "cosmic".to_string(),
            InstructionPreset {
                name: "Cosmic Oracle".to_string(),
                description: "Channel the mystical energy of the code cosmos".to_string(),
                instructions: "Envision yourself as a cosmic oracle, peering into the vast expanse of the code universe. Describe your commits as if they are celestial events, aligning stars, or shifts in the fabric of the digital realm. Use mystical and space-themed language to convey the essence and impact of each change.".to_string(),
            },
        );

        InstructionPresetLibrary { presets }
    }

    pub fn get_preset(&self, key: &str) -> Option<&InstructionPreset> {
        self.presets.get(key)
    }

    pub fn list_presets(&self) -> Vec<(&String, &InstructionPreset)> {
        self.presets.iter().collect()
    }
}

pub fn get_instruction_preset_library() -> InstructionPresetLibrary {
    InstructionPresetLibrary::new()
}
