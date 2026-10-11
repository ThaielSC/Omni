use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use super::statusline::modal_title_style;
use crate::config::ThemeColors;

pub fn render(
    frame: &mut Frame,
    area: Rect,
    theme: &ThemeColors,
    _border: ratatui::widgets::BorderType,
    dependency_key: &str,
) {
    let popup = centered(area, 82, 90);
    frame.render_widget(Clear, popup);
    let rounded = ratatui::widgets::BorderType::Rounded;
    let block = Block::default()
        .title(" Omni · Keyboard help ")
        .borders(Borders::ALL)
        .border_type(rounded)
        .border_style(Style::default().fg(theme.border_focused_color()))
        .style(Style::default().bg(theme.bg_color()).fg(theme.fg_color()));
    let content = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(popup);
    let rows = [
        ("NAVIGATION", ""),
        ("h / l", "Move between manager, package, and details panes"),
        ("j / k", "Move selection or scroll details"),
        ("g / G", "Jump to first / last item"),
        ("1..6", "Select All, Pacman, AUR, Flatpak, Cargo, Python"),
        ("SEARCH & VIEW", ""),
        ("/", "Filter packages by name or description"),
        ("Esc", "Clear search; close help; or cancel prompt"),
        ("s", "Cycle size, name, and date ordering"),
        (
            dependency_key,
            "Toggle dependencies; OFF hides confirmed deps only, unknown stays visible",
        ),
        ("OMNI", ""),
        ("?", "Toggle this help"),
        ("q / Ctrl-C", "Quit Omni"),
        ("d / x", "Uninstall is not enabled in this alpha"),
        ("Enter", "Open package URL in details is not available yet"),
    ];
    let lines = rows
        .iter()
        .map(|(key, description)| {
            if description.is_empty() {
                Line::from(Span::styled(*key, modal_title_style(theme)))
            } else {
                Line::from(vec![
                    Span::styled(
                        format!(" {key:<11} "),
                        theme.selection_style().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  "),
                    Span::styled(*description, Style::default().fg(theme.fg_color())),
                ])
            }
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("KEYS", modal_title_style(theme)),
            Span::styled(
                "  Navigate Omni with a keyboard-first workflow",
                Style::default().fg(theme.border_color()),
            ),
        ]))
        .style(Style::default().bg(theme.bg_color())),
        content[0],
    );
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .style(Style::default().bg(theme.bg_color())),
        content[1],
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            " Esc / q  close help ",
            theme.selection_style().add_modifier(Modifier::BOLD),
        )))
        .alignment(Alignment::Center)
        .style(Style::default().bg(theme.bg_color())),
        content[2],
    );
    frame.render_widget(block, popup);
}

fn centered(area: Rect, width_percent: u16, height_percent: u16) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - height_percent) / 2),
            Constraint::Percentage(height_percent),
            Constraint::Percentage((100 - height_percent) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - width_percent) / 2),
            Constraint::Percentage(width_percent),
            Constraint::Percentage((100 - width_percent) / 2),
        ])
        .split(vertical[1])[1]
}
