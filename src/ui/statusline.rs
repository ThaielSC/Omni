use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::config::ThemeColors;
use crate::model::{FilterTab, SortMode};

pub fn render(
    frame: &mut Frame,
    area: Rect,
    theme: &ThemeColors,
    searching: bool,
    query: &str,
    tab: FilterTab,
    selected: usize,
    count: usize,
    total_size: u64,
    sort: SortMode,
    show_dependencies: bool,
) {
    let base = theme.status_bar_style();
    let mode_style = if searching {
        theme.mode_search_style()
    } else {
        theme.mode_normal_style()
    };
    let mut left = if searching {
        format!(" /{}", query)
    } else {
        " NOR ".to_owned()
    };
    if searching {
        left.push('▏');
    }
    let position = if count == 0 {
        "0/0".to_owned()
    } else {
        format!("{}/{}", selected.saturating_add(1).min(count), count)
    };
    let size = crate::utils::format::format_bytes(Some(total_size));
    let right = format!("{}  Size: {}  utf-8", sort.display_name(), size);
    let available = area.width as usize;
    let right_width = right.chars().count().min(available);
    let left_width = left
        .chars()
        .count()
        .min(available.saturating_sub(right_width));
    let middle_width = available.saturating_sub(left_width + right_width);
    let package_scope = if show_dependencies {
        "DEPS ON"
    } else {
        "DEPS OFF"
    };
    let middle = format!(" [{}] {} {}", tab.display_name(), position, package_scope);
    let middle = truncate(&middle, middle_width);
    let spacer = " ".repeat(middle_width.saturating_sub(middle.chars().count()));
    let line = Line::from(vec![
        Span::styled(truncate(&left, left_width), mode_style),
        Span::styled(format!("{}{}", middle, spacer), base),
        Span::styled(right, base),
    ]);
    frame.render_widget(Paragraph::new(line).style(base), area);
}

fn truncate(value: &str, width: usize) -> String {
    value.chars().take(width).collect()
}

pub fn modal_title_style(theme: &ThemeColors) -> Style {
    Style::default()
        .fg(theme.border_focused_color())
        .add_modifier(ratatui::style::Modifier::BOLD)
}
