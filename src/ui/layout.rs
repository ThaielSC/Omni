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

/// Below this width, keeping all three panes visible makes their contents
/// unreadable. In compact mode the focused pane gets the full content width;
/// h/l (or Tab/Shift-Tab) still cycles through the panes.
const COMPACT_WIDTH: u16 = 96;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPane {
    Managers,
    Packages,
    Details,
}

pub fn split(area: Rect, config: &LayoutConfig, focus: FocusedPane) -> PaneAreas {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);
    let content = rows[0];
    if content.width < COMPACT_WIDTH {
        let managers_visible = config.show_managers && focus == FocusedPane::Managers;
        let details_visible = config.show_details && focus == FocusedPane::Details;
        return PaneAreas {
            managers: managers_visible.then_some(content),
            packages: if managers_visible || details_visible {
                Rect::new(content.x, content.y, 0, content.height)
            } else {
                content
            },
            details: details_visible.then_some(content),
            dividers: Vec::new(),
            status: rows[1],
        };
    }
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

#[cfg(test)]
mod tests {
    use super::{FocusedPane, split};
    use crate::config::LayoutConfig;
    use ratatui::layout::Rect;

    #[test]
    fn compact_layout_gives_the_focused_pane_full_width() {
        let config = LayoutConfig::default();
        let area = Rect::new(0, 0, 60, 14);

        let managers = split(area, &config, FocusedPane::Managers);
        assert_eq!(managers.managers, Some(Rect::new(0, 0, 60, 13)));
        assert_eq!(managers.packages.width, 0);
        assert!(managers.details.is_none());

        let packages = split(area, &config, FocusedPane::Packages);
        assert!(packages.managers.is_none());
        assert_eq!(packages.packages, Rect::new(0, 0, 60, 13));
        assert!(packages.details.is_none());

        let details = split(area, &config, FocusedPane::Details);
        assert!(details.managers.is_none());
        assert_eq!(details.packages.width, 0);
        assert_eq!(details.details, Some(Rect::new(0, 0, 60, 13)));
    }

    #[test]
    fn wide_layout_keeps_all_three_panes_visible() {
        let areas = split(
            Rect::new(0, 0, 120, 36),
            &LayoutConfig::default(),
            FocusedPane::Packages,
        );

        assert!(areas.managers.is_some());
        assert!(areas.packages.width > 0);
        assert!(areas.details.is_some());
    }
}
