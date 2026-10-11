use ratatui::layout::{Constraint, Direction, Layout, Rect};

use crate::config::LayoutConfig;

#[derive(Debug, Clone)]
pub struct PaneAreas {
    pub managers: Option<Rect>,
    pub packages: Rect,
    pub details: Option<Rect>,
    pub dividers: Vec<Rect>,
    pub status: Rect,
}

pub fn split(area: Rect, config: &LayoutConfig) -> PaneAreas {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);
    let content = rows[0];
    let mut constraints = Vec::with_capacity(3);
    if config.show_managers {
        constraints.push(Constraint::Ratio(
            config.ratio[0] as u32,
            visible_total(config),
        ));
    }
    constraints.push(Constraint::Ratio(
        config.ratio[1] as u32,
        visible_total(config),
    ));
    if config.show_details {
        constraints.push(Constraint::Ratio(
            config.ratio[2] as u32,
            visible_total(config),
        ));
    }
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(content);
    let mut index = 0;
    let mut managers = if config.show_managers {
        let pane = Some(columns[index]);
        index += 1;
        pane
    } else {
        None
    };
    let mut packages = columns[index];
    index += 1;
    let mut details = config.show_details.then(|| columns[index]);
    let mut dividers = Vec::new();
    if config.has_vertical_separators() {
        let mut visible = Vec::new();
        if let Some(area) = managers {
            visible.push((0, area));
        }
        visible.push((1, packages));
        if let Some(area) = details {
            visible.push((2, area));
        }
        for (panel_index, area) in visible.iter().take(visible.len().saturating_sub(1)) {
            if area.width == 0 {
                continue;
            }
            dividers.push(Rect::new(
                area.x + area.width - 1,
                content.y,
                1,
                content.height,
            ));
            let reduced = Rect::new(area.x, area.y, area.width - 1, area.height);
            match panel_index {
                0 => managers = Some(reduced),
                1 => packages = reduced,
                _ => details = Some(reduced),
            }
        }
    }
    PaneAreas {
        managers,
        packages,
        details,
        dividers,
        status: rows[1],
    }
}

fn visible_total(config: &LayoutConfig) -> u32 {
    (if config.show_managers {
        config.ratio[0]
    } else {
        0
    } + config.ratio[1]
        + if config.show_details {
            config.ratio[2]
        } else {
            0
        })
    .max(1) as u32
}
