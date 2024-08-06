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
                description: "Standard professional style".to_string(),
                instructions: "Provide clear, concise, and professional responses. Focus on accuracy and relevance.".to_string(),
            },
        );

        presets.insert(
            "detailed".to_string(),
            InstructionPreset {
                name: "Detailed".to_string(),
                description: "Provide more context and explanation".to_string(),
                instructions: "Offer comprehensive explanations, including background information, potential impacts, and related considerations. Aim for thoroughness while maintaining clarity.".to_string(),
            },
        );

        presets.insert(
            "concise".to_string(),
            InstructionPreset {
                name: "Concise".to_string(),
                description: "Short and to-the-point responses".to_string(),
                instructions: "Keep responses brief and focused on the core information. Prioritize essential details and avoid unnecessary elaboration.".to_string(),
            },
        );

        presets.insert(
            "technical".to_string(),
            InstructionPreset {
                name: "Technical".to_string(),
                description: "Focus on technical details".to_string(),
                instructions: "Emphasize technical aspects in your responses. Include specific terminology, methodologies, or performance impacts where relevant. Assume a technically proficient audience.".to_string(),
            },
        );

        presets.insert(
            "storyteller".to_string(),
            InstructionPreset {
                name: "Storyteller".to_string(),
                description: "Frame information as part of an ongoing narrative".to_string(),
                instructions: "Present information as if it's part of a larger story. Use narrative elements to describe changes, developments, or features. Connect individual elements to create a cohesive narrative arc.".to_string(),
            },
        );

        presets.insert(
            "emoji-lover".to_string(),
            InstructionPreset {
                name: "Emoji Enthusiast".to_string(),
                description: "Use emojis to enhance communication".to_string(),
                instructions: "Incorporate relevant emojis throughout your responses to add visual flair and quickly convey the nature of the information. Ensure emojis complement rather than replace clear communication.".to_string(),
            },
        );

        presets.insert(
            "formal".to_string(),
            InstructionPreset {
                name: "Formal".to_string(),
                description: "Maintain a highly professional and formal tone".to_string(),
                instructions: "Use formal language and structure in your responses. Avoid colloquialisms and maintain a respectful, business-like tone throughout.".to_string(),
            },
        );

        presets.insert(
            "explanatory".to_string(),
            InstructionPreset {
                name: "Explanatory".to_string(),
                description: "Focus on explaining concepts and changes".to_string(),
                instructions: "Prioritize explaining the 'why' behind information or changes. Provide context, rationale, and potential implications to foster understanding.".to_string(),
            },
        );

        presets.insert(
            "user-focused".to_string(),
            InstructionPreset {
                name: "User-Focused".to_string(),
                description: "Emphasize user impact and benefits".to_string(),
                instructions: "Frame information in terms of its impact on users or stakeholders. Highlight benefits, improvements, and how changes affect the user experience.".to_string(),
            },
        );

        presets.insert(
            "cosmic".to_string(),
            InstructionPreset {
                name: "Cosmic Oracle".to_string(),
                description: "Channel mystical and cosmic energy".to_string(),
                instructions: "Envision yourself as a cosmic entity, peering into the vast expanse of possibilities. Describe information as if they are celestial events or shifts in the fabric of reality. Use mystical and space-themed language to convey the essence and impact of each element.".to_string(),
            },
        );

        presets.insert(
            "academic".to_string(),
            InstructionPreset {
                name: "Academic".to_string(),
                description: "Scholarly and research-oriented style".to_string(),
                instructions: "Adopt an academic tone, citing relevant sources or methodologies where applicable. Use precise language and maintain a formal, analytical approach to the subject matter.".to_string(),
            },
        );

        presets.insert(
            "comparative".to_string(),
            InstructionPreset {
                name: "Comparative".to_string(),
                description: "Highlight differences and similarities".to_string(),
                instructions: "Focus on comparing and contrasting elements. Identify key differences and similarities, and explain their significance or implications.".to_string(),
            },
        );

        presets.insert(
            "future-oriented".to_string(),
            InstructionPreset {
                name: "Future-Oriented".to_string(),
                description: "Emphasize future implications and possibilities".to_string(),
                instructions: "Frame information in terms of its future impact. Discuss potential developments, long-term consequences, and how current changes might shape future scenarios.".to_string(),
            },
        );

        presets.insert(
            "time-traveler".to_string(),
            InstructionPreset {
                name: "Time Traveler".to_string(),
                description: "Narrate from different points in time".to_string(),
                instructions: "Imagine you're a time traveler, jumping between past, present, and future. Describe current information as if you're reporting from different time periods. Use appropriate historical or futuristic language and references, and highlight how perspectives change across time.".to_string(),
            },
        );

        presets.insert(
            "chef-special".to_string(),
            InstructionPreset {
                name: "Chef's Special".to_string(),
                description: "Present information as a culinary experience".to_string(),
                instructions: "Treat the information as ingredients in a gourmet meal. Describe changes or updates as if you're crafting a recipe or presenting a dish. Use culinary terms, cooking metaphors, and sensory descriptions to make the content more flavorful and engaging.".to_string(),
            },
        );

        presets.insert(
            "superhero-saga".to_string(),
            InstructionPreset {
                name: "Superhero Saga".to_string(),
                description: "Frame information in a superhero universe".to_string(),
                instructions: "Imagine the project or product as a superhero universe. Describe features, changes, or updates as if they're superpowers, epic battles, or heroic adventures. Use dramatic, comic-book style language and frame developments in terms of heroes, villains, and saving the day.".to_string(),
            },
        );

        presets.insert(
            "nature-documentary".to_string(),
            InstructionPreset {
                name: "Nature Documentary".to_string(),
                description: "Narrate as if observing a natural phenomenon".to_string(),
                instructions: "Channel your inner David Attenborough and describe the information as if you're narrating a nature documentary. Treat code, features, or processes as flora and fauna in a complex ecosystem. Use a tone of fascination and wonder, and explain interactions and developments as if observing them in their natural habitat.".to_string(),
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
