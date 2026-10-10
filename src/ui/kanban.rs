use eframe::egui::{self, Ui};

use crate::model::{Action, Filter, KanbanLane};

use super::{kanban_column, theme};

pub struct KanbanView {
    pub lanes: Vec<KanbanLane>,
    pub collapsed: Vec<Filter>,
}

pub fn kanban(ui: &mut Ui, view: &KanbanView) -> Vec<Action> {
    let mut actions = Vec::new();
    let size = ui.available_size();
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let expanded = view
        .lanes
        .iter()
        .filter(|lane| !view.collapsed.contains(&lane.filter))
        .count();
    let collapsed = view.lanes.len().saturating_sub(expanded);
    let (lane_w, board_w) = lane_metrics(rect.width(), expanded, collapsed);
    let inner = egui::Rect::from_center_size(rect.center(), egui::vec2(board_w, rect.height()))
        .intersect(rect);
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::left_to_right(egui::Align::Min)),
    );
    child.set_clip_rect(inner);
    child.set_height(inner.height());
    child.spacing_mut().item_spacing = egui::vec2(theme::LANE_GAP, 0.0);
    let lane_h = inner.height();
    for lane in &view.lanes {
        let is_collapsed = view.collapsed.contains(&lane.filter);
        let width = if is_collapsed {
            theme::LANE_COLLAPSED_W
        } else {
            lane_w
        };
        actions.extend(kanban_column::kanban_column(
            &mut child,
            &kanban_column::KanbanColumnProps {
                lane: lane.clone(),
                collapsed: is_collapsed,
                height: lane_h,
                width,
            },
        ));
    }
    actions
}

/// Lane widths for the kanban board. Integer narrowing uses saturating ops
/// plus `try_from(...).unwrap_or(0)`, so counts can never wrap or panic.
/// The `f32` ops below carry per-statement allows: IEEE-754 arithmetic
/// cannot panic or wrap to UB (worst case is inf/NaN, clamped by `.max`/`.min`
/// at each step), so the lint's integer-overflow rationale does not apply.
fn lane_metrics(available: f32, expanded: usize, collapsed: usize) -> (f32, f32) {
    let gap_count = expanded.saturating_add(collapsed).saturating_sub(1);
    #[allow(clippy::arithmetic_side_effects)]
    let gaps = theme::LANE_GAP * f32::from(u8::try_from(gap_count).unwrap_or(0));
    #[allow(clippy::arithmetic_side_effects)]
    let collapsed_w = theme::LANE_COLLAPSED_W * f32::from(u8::try_from(collapsed).unwrap_or(0));
    #[allow(clippy::arithmetic_side_effects)]
    let rest = (available - collapsed_w - gaps).max(0.0);
    #[allow(clippy::arithmetic_side_effects)]
    let expanded_w = match u8::try_from(expanded) {
        Ok(0) | Err(_) => theme::LANE_MAX_W,
        Ok(count) => (rest / f32::from(count)).min(theme::LANE_MAX_W),
    };
    #[allow(clippy::arithmetic_side_effects)]
    let board_w = expanded_w * f32::from(u8::try_from(expanded).unwrap_or(0)) + collapsed_w + gaps;
    (expanded_w, board_w.min(available))
}

#[cfg(test)]
mod tests {
    use super::lane_metrics;
    use crate::ui::theme;

    #[test]
    fn flex_fills_then_caps() {
        let (wide_lane, wide_board) = lane_metrics(2400.0, 5, 0);
        assert!(wide_lane <= theme::LANE_MAX_W);
        assert!(wide_lane > 300.0);
        assert!(wide_board < 2400.0);
        let (fill_lane, fill_board) = lane_metrics(1200.0, 5, 0);
        assert!(fill_lane < theme::LANE_MAX_W);
        assert!(fill_board > 1199.0);
        assert!(fill_board < 1201.0);
    }
}
