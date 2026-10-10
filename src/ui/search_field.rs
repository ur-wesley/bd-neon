use eframe::egui::{self, Response, TextEdit, Ui};

use super::{icons, theme};

pub struct SearchFieldProps {
    pub value: String,
    pub focused: bool,
    pub hint: &'static str,
}

pub fn search_field(ui: &mut Ui, props: &mut SearchFieldProps) -> (Response, bool) {
    let width = ui.available_width().min(theme::SEARCH_MAX);
    let mut clear_clicked = false;
    let inner = theme::glass_slot_compact(
        ui,
        egui::vec2(width, theme::CONTROL_H),
        egui::Sense::hover(),
        |ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            ui.label(
                egui::RichText::new(icons::MAGNIFYING_GLASS)
                    .size(14.0)
                    .color(theme::dim_text()),
            );
            let edit_response = ui.add(
                TextEdit::singleline(&mut props.value)
                    .hint_text(egui::RichText::new(props.hint).color(theme::dim_text()))
                    .text_color(theme::foreground())
                    .frame(egui::Frame::NONE)
                    .desired_width(f32::INFINITY)
                    .margin(egui::Margin::same(4)),
            );
            if !props.value.is_empty() {
                let clear_btn = theme::pointer(
                    ui.add(
                        egui::Button::new(
                            egui::RichText::new(icons::X)
                                .small()
                                .color(theme::dim_text()),
                        )
                        .fill(egui::Color32::TRANSPARENT)
                        .stroke(egui::Stroke::NONE)
                        .min_size(egui::vec2(16.0, 16.0)),
                    ),
                );
                if clear_btn.clicked() {
                    clear_clicked = true;
                }
            }
            edit_response
        },
    );
    let response = inner.inner;
    let changed = if clear_clicked {
        props.value.clear();
        response.request_focus();
        true
    } else {
        response.changed()
    };
    if props.focused {
        response.request_focus();
        props.focused = false;
    }
    (response, changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_field_renders_initial_value() {
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                let mut props = SearchFieldProps {
                    value: "needle".to_owned(),
                    focused: false,
                    hint: "Filter…",
                };
                let (_, _) = search_field(ui, &mut props);
                assert_eq!(props.value, "needle");
            });
        });
        output.textures_delta.clear();
    }
}
