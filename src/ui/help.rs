use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::config::ThemeColors;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    Navigation,
    Browse,
    Omni,
}

impl Section {
    fn title(self) -> &'static str {
        match self {
            Self::Navigation => "NAVIGATION",
            Self::Browse => "BROWSE & FILTER",
            Self::Omni => "OMNI",
        }
    }

    fn style(self, theme: &ThemeColors) -> Style {
        match self {
            Self::Navigation => theme.mode_normal_style(),
            Self::Browse => theme.mode_search_style(),
            Self::Omni => theme.mode_alert_style(),
        }
    }
}

struct Shortcut {
    section: Section,
    keys: String,
    description: &'static str,
}

pub fn render(
    frame: &mut Frame,
    area: Rect,
    theme: &ThemeColors,
    border: ratatui::widgets::BorderType,
    dependency_key: &str,
    package_dependency_key: &str,
    updates_key: &str,
    selection_key: &str,
    clear_selection_key: &str,
    update_packages_key: &str,
    add_project_key: &str,
    copy_package_name_key: &str,
    filter: &str,
    filtering: bool,
) {
    let popup = centered(area, 74, 80);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(" Help ")
        .title_style(
            Style::default()
                .fg(theme.border_focused_color())
                .add_modifier(Modifier::BOLD),
        )
        .title_bottom(Line::from(Span::styled(
            " ? · keyboard reference ",
            Style::default().fg(theme.border_color()),
        )))
        .borders(Borders::ALL)
        .border_type(border)
        .border_style(Style::default().fg(theme.border_focused_color()))
        .style(Style::default().bg(theme.bg_color()).fg(theme.fg_color()));
    let inner = block.inner(popup);
    let content = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(inner);

    let shortcuts = shortcuts(
        dependency_key,
        package_dependency_key,
        updates_key,
        selection_key,
        clear_selection_key,
        update_packages_key,
        add_project_key,
        copy_package_name_key,
    );
    let filter = filter.trim().to_lowercase();
    let mut lines = Vec::new();
    let mut active_section = None;
    for shortcut in shortcuts.iter().filter(|item| {
        filter.is_empty()
            || item.keys.to_lowercase().contains(&filter)
            || item.description.to_lowercase().contains(&filter)
            || item.section.title().to_lowercase().contains(&filter)
    }) {
        if active_section != Some(shortcut.section) {
            active_section = Some(shortcut.section);
            lines.push(section_line(
                shortcut.section,
                theme,
                content[1].width as usize,
            ));
        }
        lines.push(Line::from(vec![
            Span::styled(
                format!(" {:<13} ", shortcut.keys),
                theme.selection_style().add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(shortcut.description, Style::default().fg(theme.fg_color())),
        ]));
    }
    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "No matching shortcuts",
            Style::default()
                .fg(theme.border_color())
                .add_modifier(Modifier::ITALIC),
        )));
    }

    let intro = if filtering {
        Line::from(vec![
            Span::styled(
                " / ",
                theme.mode_search_style().add_modifier(Modifier::BOLD),
            ),
            Span::styled(filter.as_str(), Style::default().fg(theme.fg_color())),
            Span::styled("▏", Style::default().fg(theme.border_focused_color())),
        ])
    } else {
        Line::from(vec![
            Span::styled(
                "OMNI",
                Style::default()
                    .fg(theme.border_focused_color())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "  Package inventory · keyboard-first controls",
                Style::default().fg(theme.border_color()),
            ),
        ])
    };
    frame.render_widget(
        Paragraph::new(intro).style(Style::default().bg(theme.bg_color())),
        content[0],
    );
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(theme.bg_color())),
        content[1],
    );

    let footer = if filtering {
        " Enter · finish filter     Esc · close help "
    } else {
        " / · filter     Esc / q · close "
    };
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            footer,
            theme.selection_style().add_modifier(Modifier::BOLD),
        )))
        .alignment(ratatui::layout::Alignment::Center)
        .style(Style::default().bg(theme.bg_color())),
        content[2],
    );
    frame.render_widget(block, popup);
}

fn shortcuts(
    dependency_key: &str,
    package_dependency_key: &str,
    updates_key: &str,
    selection_key: &str,
    clear_selection_key: &str,
    update_packages_key: &str,
    add_project_key: &str,
    copy_package_name_key: &str,
) -> Vec<Shortcut> {
    use Section::{Browse, Navigation, Omni};

    vec![
        Shortcut {
            section: Navigation,
            keys: "h / l".into(),
            description: "Move between the three panes",
        },
        Shortcut {
            section: Navigation,
            keys: "j / k".into(),
            description: "Move selection or scroll details",
        },
        Shortcut {
            section: Navigation,
            keys: "g / G".into(),
            description: "Jump to first / last item",
        },
        Shortcut {
            section: Navigation,
            keys: "1..6".into(),
            description: "Select All or a package manager",
        },
        Shortcut {
            section: Browse,
            keys: "/".into(),
            description: "Filter packages by name or description",
        },
        Shortcut {
            section: Browse,
            keys: "Esc".into(),
            description: "Clear the package filter",
        },
        Shortcut {
            section: Browse,
            keys: "s".into(),
            description: "Cycle size, name, and date ordering",
        },
        Shortcut {
            section: Browse,
            keys: updates_key.into(),
            description: "Check for and show packages with available updates",
        },
        Shortcut {
            section: Browse,
            keys: selection_key.into(),
            description: "Select package and advance to the next row",
        },
        Shortcut {
            section: Browse,
            keys: clear_selection_key.into(),
            description: "Clear all selected packages",
        },
        Shortcut {
            section: Browse,
            keys: update_packages_key.into(),
            description: "Update selected packages or the package under cursor",
        },
        Shortcut {
            section: Browse,
            keys: add_project_key.into(),
            description: "Add a dependency to the selected project manager",
        },
        Shortcut {
            section: Browse,
            keys: copy_package_name_key.into(),
            description: "Copy the selected package name to the system clipboard",
        },
        Shortcut {
            section: Browse,
            keys: dependency_key.into(),
            description: "Show or hide packages installed as dependencies",
        },
        Shortcut {
            section: Browse,
            keys: package_dependency_key.into(),
            description: "Open dependency list for selected package",
        },
        Shortcut {
            section: Omni,
            keys: "?".into(),
            description: "Open or close this help",
        },
        Shortcut {
            section: Omni,
            keys: "q / Ctrl-C".into(),
            description: "Quit Omni",
        },
        Shortcut {
            section: Omni,
            keys: "d / x".into(),
            description: "Uninstall selected packages; output appears in Details",
        },
        Shortcut {
            section: Omni,
            keys: "Esc".into(),
            description: "Close completed uninstall output",
        },
    ]
}

fn section_line(section: Section, theme: &ThemeColors, width: usize) -> Line<'static> {
    let title = format!(" {} ", section.title());
    let rule_width = width.saturating_sub(title.chars().count());
    Line::from(vec![
        Span::styled(title, section.style(theme).add_modifier(Modifier::BOLD)),
        Span::styled(
            "─".repeat(rule_width),
            Style::default().fg(theme.border_color()),
        ),
    ])
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
