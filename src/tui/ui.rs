use super::state::{Mode, TuiState, UserInfoFocus};
use crate::context::format_commit_message;
use crate::ui::*;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};
use unicode_width::UnicodeWidthStr;

pub fn draw_ui(f: &mut Frame, state: &mut TuiState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(3), // Title
                Constraint::Length(2), // Navigation bar
                Constraint::Length(2), // User info
                Constraint::Min(5),    // Commit message
                Constraint::Length(8), // Instructions
                Constraint::Length(3), // Emoji and Preset
                Constraint::Length(1), // Status
            ]
            .as_ref(),
        )
        .split(f.size());

    draw_title(f, chunks[0]);
    draw_nav_bar(f, chunks[1]);
    draw_user_info(f, state, chunks[2]);
    draw_commit_message(f, state, chunks[3]);
    draw_instructions(f, state, chunks[4]);
    draw_emoji_preset(f, state, chunks[5]);
    draw_status(f, state, chunks[6]);

    if state.mode == Mode::SelectingEmoji {
        draw_emoji_popup(f, state);
    } else if state.mode == Mode::SelectingPreset {
        draw_preset_popup(f, state);
    } else if state.mode == Mode::EditingUserInfo {
        draw_user_info_popup(f, state);
    }
}

fn draw_title(f: &mut Frame, area: Rect) {
    let title = vec![
        Line::from(Span::styled(
            "    .  *  .  ✨  .  *  .    ",
            Style::default().fg(GALAXY_PINK),
        )),
        Line::from(vec![
            Span::styled("  *    ", Style::default().fg(GALAXY_PINK)),
            Span::styled(
                "✨🔮 Git-Iris v0.1.0 - Cosmic Commit 🔮✨",
                Style::default()
                    .fg(GALAXY_PINK)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("   * ", Style::default().fg(GALAXY_PINK)),
        ]),
    ];

    let title_widget = Paragraph::new(title).alignment(ratatui::layout::Alignment::Center);
    f.render_widget(title_widget, area);
}

fn draw_nav_bar(f: &mut Frame, area: Rect) {
    let nav_items = vec![
        ("←→", "Navigate", CELESTIAL_BLUE),
        ("E", "Message", SOLAR_YELLOW),
        ("I", "Instructions", AURORA_GREEN),
        ("G", "Emoji", PLASMA_CYAN),
        ("P", "Preset", COMET_ORANGE),
        ("U", "User Info", GALAXY_PINK),
        ("R", "Regenerate", METEOR_RED),
        ("⏎", "Commit", STARLIGHT),
    ];
    let nav_spans: Vec<Span> = nav_items
        .into_iter()
        .flat_map(|(key, desc, color)| {
            vec![
                Span::styled(
                    format!("{}", key),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!(": {} ", desc), Style::default().fg(NEBULA_PURPLE)),
            ]
        })
        .collect();
    let nav_bar =
        Paragraph::new(Line::from(nav_spans)).alignment(ratatui::layout::Alignment::Center);
    f.render_widget(nav_bar, area);
}

fn draw_user_info(f: &mut Frame, state: &TuiState, area: Rect) {
    let user_info = Paragraph::new(Line::from(vec![
        Span::styled("👤 ", Style::default().fg(PLASMA_CYAN)),
        Span::styled(
            &state.user_name,
            Style::default()
                .fg(AURORA_GREEN)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" | "),
        Span::styled("✉️ ", Style::default().fg(PLASMA_CYAN)),
        Span::styled(
            &state.user_email,
            Style::default()
                .fg(AURORA_GREEN)
                .add_modifier(Modifier::BOLD),
        ),
    ]))
    .style(Style::default())
    .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(user_info, area);
}

fn draw_commit_message(f: &mut Frame, state: &TuiState, area: Rect) {
    let message_title = format!(
        "✦ Commit Message ({}/{})",
        state.current_index + 1,
        state.messages.len()
    );
    let message_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CELESTIAL_BLUE))
        .title(Span::styled(
            message_title,
            Style::default()
                .fg(GALAXY_PINK)
                .add_modifier(Modifier::BOLD),
        ));

    let message_content = if state.mode == Mode::EditingMessage {
        state.message_textarea.lines().join("\n")
    } else {
        format_commit_message(&state.messages[state.current_index])
    };

    let message = Paragraph::new(message_content)
        .block(message_block)
        .style(Style::default().fg(SOLAR_YELLOW))
        .wrap(Wrap { trim: true });

    f.render_widget(message, area);
}

fn draw_instructions(f: &mut Frame, state: &mut TuiState, area: Rect) {
    let instructions_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CELESTIAL_BLUE))
        .title(Span::styled(
            "✧ Custom Instructions",
            Style::default()
                .fg(GALAXY_PINK)
                .add_modifier(Modifier::BOLD),
        ));

    match state.mode {
        Mode::EditingInstructions => {
            state.instructions_textarea.set_block(instructions_block);
            state
                .instructions_textarea
                .set_style(Style::default().fg(PLASMA_CYAN));
            f.render_widget(&state.instructions_textarea, area);
        }
        _ => {
            let instructions = Paragraph::new(state.custom_instructions.clone())
                .block(instructions_block)
                .style(Style::default().fg(PLASMA_CYAN))
                .wrap(Wrap { trim: true });
            f.render_widget(instructions, area);
        }
    }
}

fn draw_emoji_preset(f: &mut Frame, state: &TuiState, area: Rect) {
    let preset_with_emoji = state.get_selected_preset_name_with_emoji();
    let emoji_preset = Paragraph::new(Line::from(vec![
        Span::styled("Emoji: ", Style::default().fg(NEBULA_PURPLE)),
        Span::styled(
            &state.selected_emoji,
            Style::default()
                .fg(SOLAR_YELLOW)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  |  "),
        Span::styled("Preset: ", Style::default().fg(NEBULA_PURPLE)),
        Span::styled(
            preset_with_emoji,
            Style::default()
                .fg(COMET_ORANGE)
                .add_modifier(Modifier::BOLD),
        ),
    ]))
    .style(Style::default())
    .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(emoji_preset, area);
}

fn draw_status(f: &mut Frame, state: &mut TuiState, area: Rect) {
    let (spinner_with_space, status_content, color, content_width) =
        if let Some(spinner) = &mut state.spinner {
            spinner.tick()
        } else {
            (
                "  ".to_string(),
                state.status.clone(),
                AURORA_GREEN,
                state.status.width() + 2,
            )
        };

    let terminal_width = f.size().width as usize;
    let left_padding = (terminal_width - content_width) / 2;
    let right_padding = terminal_width - content_width - left_padding;

    let status_line = Line::from(vec![
        Span::raw(" ".repeat(left_padding)),
        Span::styled(spinner_with_space, Style::default().fg(PLASMA_CYAN)),
        Span::styled(status_content, Style::default().fg(color)),
        Span::raw(" ".repeat(right_padding)),
    ]);

    let status_widget =
        Paragraph::new(vec![status_line]).alignment(ratatui::layout::Alignment::Left);
    f.render_widget(Clear, area); // Clear the entire status line
    f.render_widget(status_widget, area);
}

fn draw_emoji_popup(f: &mut Frame, state: &mut TuiState) {
    let popup_block = Block::default()
        .title(Span::styled(
            "✨ Select Emoji",
            Style::default()
                .fg(SOLAR_YELLOW)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(NEBULA_PURPLE));

    let area = f.size();
    let popup_area = Rect::new(
        area.x + 10,
        area.y + 5,
        area.width.saturating_sub(20).min(60),
        area.height.saturating_sub(10).min(20),
    );

    let items: Vec<ListItem> = state
        .emoji_list
        .iter()
        .map(|(emoji, description)| {
            ListItem::new(Line::from(vec![
                Span::styled(format!("{} ", emoji), Style::default().fg(SOLAR_YELLOW)),
                Span::styled(description, Style::default().fg(PLASMA_CYAN)),
            ]))
        })
        .collect();

    let list = List::new(items).block(popup_block).highlight_style(
        Style::default()
            .bg(CELESTIAL_BLUE)
            .fg(STARLIGHT)
            .add_modifier(Modifier::BOLD),
    );

    f.render_widget(Clear, popup_area);
    f.render_stateful_widget(list, popup_area, &mut state.emoji_list_state);
}

fn draw_preset_popup(f: &mut Frame, state: &mut TuiState) {
    let popup_block = Block::default()
        .title(Span::styled(
            "🌟 Select Preset",
            Style::default()
                .fg(COMET_ORANGE)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(NEBULA_PURPLE));

    let area = f.size();
    let popup_area = Rect::new(
        area.x + 5,
        area.y + 5,
        area.width.saturating_sub(10).min(70),
        area.height.saturating_sub(10).min(20),
    );

    let items: Vec<ListItem> = state
        .preset_list
        .iter()
        .map(|(_, emoji, name, description)| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{} {} ", emoji, name),
                    Style::default().fg(COMET_ORANGE),
                ),
                Span::styled(description, Style::default().fg(PLASMA_CYAN)),
            ]))
        })
        .collect();

    let list = List::new(items).block(popup_block).highlight_style(
        Style::default()
            .bg(CELESTIAL_BLUE)
            .fg(STARLIGHT)
            .add_modifier(Modifier::BOLD),
    );
    f.render_widget(Clear, popup_area);
    f.render_stateful_widget(list, popup_area, &mut state.preset_list_state);
}

fn draw_user_info_popup(f: &mut Frame, state: &mut TuiState) {
    let popup_block = Block::default()
        .title(Span::styled(
            "Edit User Info",
            Style::default()
                .fg(SOLAR_YELLOW)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(NEBULA_PURPLE));

    let area = f.size();
    let popup_area = Rect::new(
        area.x + 10,
        area.y + 5,
        area.width.saturating_sub(20).min(60),
        area.height.saturating_sub(10).min(10),
    );

    let popup_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(3), // Name
                Constraint::Length(3), // Email
            ]
            .as_ref(),
        )
        .split(popup_area);

    state.user_name_textarea.set_block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(
                Style::default().fg(if state.user_info_focus == UserInfoFocus::Name {
                    SOLAR_YELLOW
                } else {
                    CELESTIAL_BLUE
                }),
            )
            .title("Name"),
    );

    state.user_email_textarea.set_block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(
                Style::default().fg(if state.user_info_focus == UserInfoFocus::Email {
                    SOLAR_YELLOW
                } else {
                    CELESTIAL_BLUE
                }),
            )
            .title("Email"),
    );

    f.render_widget(Clear, popup_area);
    f.render_widget(popup_block, popup_area);
    f.render_widget(&state.user_name_textarea, popup_chunks[0]);
    f.render_widget(&state.user_email_textarea, popup_chunks[1]);
}
