use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::config::ThemeColors;
use crate::model::SortMode;

pub fn render(
    frame: &mut Frame,
    area: Rect,
    theme: &ThemeColors,
    searching: bool,
    query: &str,
    scope: &str,
    selected: usize,
    count: usize,
    total_size: u64,
    sort: SortMode,
    show_dependencies: bool,
    notice: Option<&str>,
    loading: bool,
    spinner_frame: usize,
    update_count: usize,
    updates_only: bool,
    selected_count: usize,
) {
    let base = theme.status_bar_style();
    let mode_style = if searching || updates_only {
        theme.mode_search_style()
    } else {
        theme.mode_normal_style()
    };
    let mut left = if searching {
        format!(" /{}", query)
    } else if updates_only {
        " UPD ".to_owned()
    } else {
        " NOR ".to_owned()
    };
    if searching {
        left.push('▏');
    }
    if loading {
        const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        left.push(' ');
        left.push_str(SPINNER[spinner_frame % SPINNER.len()]);
    }
    let position = if count == 0 {
        "0/0".to_owned()
    } else {
        format!("{}/{}", selected.saturating_add(1).min(count), count)
    };
    let size = crate::utils::format::format_bytes(Some(total_size));
    let right = notice
        .map(str::to_owned)
        .unwrap_or_else(|| format!("{}  Size: {}  utf-8", sort.display_name(), size));
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
    let update_label = if updates_only {
        format!(" UPDATES {update_count}")
    } else {
        String::new()
    };
    let selection_label = if selected_count > 0 {
        format!("  SEL {selected_count}")
    } else {
        String::new()
    };
    let middle = format!(
        " [{}] {} {}{}{}",
        scope, position, package_scope, update_label, selection_label
    );
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
