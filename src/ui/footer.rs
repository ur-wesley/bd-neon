use eframe::egui::{self, Ui};

use super::{icons, live_dot, theme};

pub struct FooterProps {
    pub live: bool,
    pub loading: bool,
    pub stale: bool,
    pub updated: String,
    pub project: String,
    pub issues: usize,
}

pub fn footer(ui: &mut Ui, props: &FooterProps) {
    let width = ui.available_width();
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(width, theme::FOOTER_H), egui::Sense::hover());
    ui.painter().rect(
        rect,
        egui::CornerRadius::ZERO,
        theme::muted(),
        theme::glass_stroke(),
        egui::StrokeKind::Inside,
    );
    let inner = rect.shrink2(egui::vec2(theme::PAD, 0.0));
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    child.set_height(theme::FOOTER_H);
    child.spacing_mut().item_spacing.x = theme::SPACING;
    // No "cached — updating" text: showing a different-length status string
    // while stale shifts the footer/project-path layout. Stale-with-cache
    // renders the same "updated …" label as fresh; the live-dot color
    // already carries liveness with a fixed 8px layout.
    let has_cached = props.stale && props.issues > 0;
    let status = if props.loading && !has_cached {
        "loading…".to_owned()
    } else {
        format!("updated {}", props.updated)
    };
    live_dot::live_dot(
        &mut child,
        &live_dot::LiveDotProps {
            live: props.live && (!props.loading || has_cached),
            label: status,
        },
    );
    child.separator();
    // Reserve the right side before the (potentially long) project path so
    // the middle label truncates into the remaining space instead of
    // overlapping the version / issue counts at narrow widths.
    child.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.add(
            egui::Label::new(
                egui::RichText::new(concat!("v", env!("CARGO_PKG_VERSION")))
                    .small()
                    .color(theme::dim_text()),
            )
            .truncate()
            .selectable(false),
        );
        ui.separator();
        ui.add(
            egui::Label::new(
                egui::RichText::new(format!("{} issues", props.issues))
                    .small()
                    .color(theme::dim_text()),
            )
            .truncate()
            .selectable(false),
        );
    });
    child.add(
        egui::Label::new(
            egui::RichText::new(icons::with_icon(icons::FOLDER, &props.project))
                .small()
                .color(theme::dim_text()),
        )
        .truncate()
        .selectable(false),
    );
}
