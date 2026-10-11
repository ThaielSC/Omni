use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use super::statusline::modal_title_style;
use crate::config::ThemeColors;

pub fn render(
    frame: &mut Frame,
    area: Rect,
    theme: &ThemeColors,
    border: ratatui::widgets::BorderType,
    dependency_key: &str,
) {
    let popup = centered(area, 72, 76);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(" Keyboard Shortcuts ")
        .borders(Borders::ALL)
        .border_type(border)
        .border_style(Style::default().fg(theme.border_focused_color()))
        .style(Style::default().bg(theme.bg_color()).fg(theme.fg_color()));
    let content = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(popup);
    let rows = [
        ("Navigation", ""),
        ("h / l", "Move between manager, package, and details panes"),
        ("j / k", "Move selection or scroll details"),
        ("g / G", "Jump to first / last item"),
        ("1..6", "Select All, Pacman, AUR, Flatpak, Cargo, Python"),
        ("", ""),
        ("Search", ""),
        ("/", "Filter packages by name or description"),
        ("Esc", "Clear search; close help; or cancel prompt"),
        ("s", "Cycle size, name, and date ordering"),
        (
            dependency_key,
            "Toggle dependencies; OFF hides confirmed deps only, unknown stays visible",
        ),
        ("", ""),
        ("Application", ""),
        ("?", "Toggle this help"),
        ("q / Ctrl-C", "Quit Omni"),
        ("d / x", "Uninstall is not enabled in this alpha"),
        ("", ""),
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
                        format!("{key:<13}"),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(*description),
                ])
            }
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), content[0]);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "Press Esc or q to close",
            Style::default().fg(theme.border_color()),
        )))
        .alignment(ratatui::layout::Alignment::Center),
        content[1],
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
