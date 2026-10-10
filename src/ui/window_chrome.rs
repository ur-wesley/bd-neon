use eframe::egui::{self, CornerRadius, PointerButton, Sense, Ui, ViewportCommand};

use super::{icons, theme};
use crate::platform::glass_debug_enabled;

pub const TITLE_H: f32 = 36.0;

pub fn shell<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    let rect = ui.max_rect();
    // TRANSPARENT base so DWM Acrylic/Mica shows through. Do NOT paint an
    // opaque cover here. Only a faint tint for readability (0,0,0,40).
    // `--glass-debug` swaps in a magenta tint so a checker wallpaper
    // visibly proves whether the surface is truly transparent.
    ui.painter()
        .rect_filled(rect, CornerRadius::ZERO, egui::Color32::TRANSPARENT);
    let tint = if glass_debug_enabled() {
        theme::glass_debug_tint()
    } else {
        theme::glass_tint()
    };
    ui.painter().rect_filled(rect, CornerRadius::ZERO, tint);
    ui.painter().rect(
        rect,
        CornerRadius::ZERO,
        egui::Color32::TRANSPARENT,
        theme::glass_stroke(),
        egui::StrokeKind::Inside,
    );

    let title_rect =
        egui::Rect::from_min_max(rect.min, egui::pos2(rect.max.x, rect.min.y + TITLE_H));
    title_bar(ui, title_rect);

    let body_rect = egui::Rect::from_min_max(egui::pos2(rect.min.x, title_rect.max.y), rect.max);
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(body_rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    child.set_clip_rect(body_rect);
    add_contents(&mut child)
}

fn title_bar(ui: &mut Ui, rect: egui::Rect) {
    ui.painter()
        .rect_filled(rect, CornerRadius::ZERO, theme::title_bar_fill());
    ui.painter().hline(
        rect.left_bottom().x..=rect.right_bottom().x,
        rect.bottom(),
        theme::glass_stroke(),
    );

    // Drag area excludes the right-side window controls (~3*TITLE_H wide)
    // so the controls don't compete with the drag gesture. Double-click
    // maximize stays on the drag area.
    let controls_w = 3.0 * TITLE_H;
    let drag_rect = egui::Rect::from_min_max(
        rect.min,
        egui::pos2((rect.max.x - controls_w).max(rect.min.x), rect.max.y),
    );
    let drag = ui.interact(
        drag_rect,
        ui.id().with("title_drag"),
        Sense::click_and_drag(),
    );
    if drag.double_clicked() {
        let maximized = ui.input(|input| input.viewport().maximized.unwrap_or(false));
        ui.ctx()
            .send_viewport_cmd(ViewportCommand::Maximized(!maximized));
    }
    if drag.drag_started_by(PointerButton::Primary) {
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }
    // While the user drags the title bar, paint at display rate so the
    // contents track the OS window move instead of idling on the 250ms
    // fallback repaint scheduled in `App::update`.
    if drag.dragged_by(PointerButton::Primary) {
        ui.ctx().request_repaint();
    }

    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        |ui| {
            ui.add_space(theme::PAD_CONTENT);
            // Crisp, modern brand mark
            let (dot_rect, _) = ui.allocate_exact_size(egui::vec2(6.0, 6.0), Sense::hover());
            ui.painter()
                .circle_filled(dot_rect.center(), 3.0_f32, theme::neon_cyan());
            ui.add_space(theme::SPACING);
            ui.add(
                egui::Label::new(
                    egui::RichText::new("bd-neon")
                        .size(13.0)
                        .strong()
                        .color(theme::foreground()),
                )
                .selectable(false),
            );
        },
    );

    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
        |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            window_controls(ui);
        },
    );
}

fn window_control_btn(ui: &mut Ui, icon: &str, is_close: bool) -> egui::Response {
    let size = egui::vec2(TITLE_H, TITLE_H);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());

    let is_hovered = response.hovered();
    let (bg_color, fg_color) = if is_hovered {
        if is_close {
            (egui::Color32::from_rgb(196, 43, 28), egui::Color32::WHITE)
        } else {
            (theme::surface_hover(), theme::foreground())
        }
    } else {
        (egui::Color32::TRANSPARENT, theme::dim_text())
    };

    if is_hovered {
        ui.painter().rect_filled(rect, CornerRadius::ZERO, bg_color);
    }

    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        icon,
        egui::FontId::proportional(13.0),
        fg_color,
    );

    theme::pointer(response)
}

fn window_controls(ui: &mut Ui) {
    if window_control_btn(ui, icons::X, true).clicked() {
        ui.ctx().send_viewport_cmd(ViewportCommand::Close);
    }
    let maximized = ui.input(|input| input.viewport().maximized.unwrap_or(false));
    let max_icon = if maximized {
        icons::CORNERS_IN
    } else {
        icons::CORNERS_OUT
    };
    if window_control_btn(ui, max_icon, false).clicked() {
        ui.ctx()
            .send_viewport_cmd(ViewportCommand::Maximized(!maximized));
    }
    if window_control_btn(ui, icons::MINUS, false).clicked() {
        ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titlebar_shell_renders_without_panic() {
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                shell(ui, |ui| {
                    ui.label("child contents");
                });
            });
        });
        output.textures_delta.clear();
    }
}
