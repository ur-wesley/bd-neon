use eframe::egui::{
    self, Color32, FontFamily, FontId, InnerResponse, Margin, Response, Sense, Stroke, Style,
    TextStyle, Ui, Visuals,
};

pub const SPACING: f32 = 8.0;
pub const PAD: f32 = 12.0;
pub const PAD_CONTENT: f32 = 12.0;
pub const PAD_I8: i8 = 12;
pub const SPACING_I8: i8 = 8;
pub const HEADER_H: f32 = 38.0;
pub const CONTROL_H: f32 = 32.0;
pub const ROW_HEIGHT: f32 = 44.0;
pub const TAB_H: f32 = 32.0;
pub const TAB_MIN_W: f32 = 120.0;
pub const TAB_MAX_W: f32 = 220.0;
pub const TAB_PAD_X: f32 = 8.0;
pub const TAB_CLOSE_W: f32 = 18.0;
pub const WORKSPACE_W: f32 = 210.0;
pub const WORKSPACE_POPUP_W: f32 = 340.0;
pub const SEARCH_MAX: f32 = 280.0;
pub const LANE_COLLAPSED_W: f32 = 38.0;
pub const LANE_COLLAPSED_PAD: f32 = 8.0;
pub const LANE_PAD: f32 = 12.0;
pub const LANE_GAP: f32 = 8.0;
pub const LANE_MAX_W: f32 = 380.0;
pub const FOOTER_H: f32 = 28.0;

pub const fn radius() -> egui::CornerRadius {
    egui::CornerRadius::same(6)
}

pub const fn radius_card() -> egui::CornerRadius {
    egui::CornerRadius::same(8)
}

pub const fn radius_sm() -> egui::CornerRadius {
    egui::CornerRadius::same(4)
}

pub fn apply(style: &mut Style) {
    let mut visuals = Visuals::dark();
    visuals.dark_mode = true;
    // OLED: TRANSPARENT panel so DWM Acrylic/Mica shows through.
    // CentralPanel is Frame::NONE in app.rs, but popups/Tooltips fall back
    // to panel_fill — TRANSPARENT lets the shell tint rule the stack.
    // All paint-stack alphas must stay low for DWM bleed-through.
    visuals.panel_fill = Color32::TRANSPARENT;
    visuals.window_fill = popup_bg();
    visuals.window_stroke = Stroke::new(1.0_f32, border());
    visuals.window_shadow = popup_shadow();
    visuals.popup_shadow = popup_shadow();
    visuals.window_corner_radius = radius_card();
    visuals.faint_bg_color = surface_elevated();
    visuals.extreme_bg_color = Color32::BLACK;
    visuals.widgets.noninteractive.bg_fill = surface_elevated();
    visuals.widgets.noninteractive.weak_bg_fill = surface_elevated();
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, border());
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, text_dim());
    visuals.widgets.noninteractive.corner_radius = radius();
    visuals.widgets.inactive.bg_fill = surface_elevated();
    visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, border());
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, text_muted());
    visuals.widgets.inactive.corner_radius = radius();
    visuals.widgets.inactive.expansion = 0.0;
    visuals.widgets.hovered.bg_fill = surface_hover();
    visuals.widgets.hovered.weak_bg_fill = surface_hover();
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, border_hover());
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, foreground());
    visuals.widgets.hovered.corner_radius = radius();
    visuals.widgets.hovered.expansion = 0.0;
    visuals.widgets.active.bg_fill = surface_active();
    visuals.widgets.active.weak_bg_fill = surface_active();
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, neon_cyan().gamma_multiply(0.55));
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, foreground());
    visuals.widgets.active.corner_radius = radius();
    visuals.widgets.active.expansion = 0.0;
    visuals.widgets.open.bg_fill = popup_bg();
    visuals.widgets.open.weak_bg_fill = popup_bg();
    visuals.widgets.open.bg_stroke = hairline();
    visuals.widgets.open.fg_stroke = Stroke::new(1.0_f32, foreground());
    visuals.widgets.open.corner_radius = radius();
    visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(56, 189, 248, 45);
    visuals.selection.stroke = Stroke::new(1.0_f32, neon_cyan());
    visuals.override_text_color = Some(foreground());
    style.visuals = visuals;
    style.spacing.item_spacing = egui::vec2(SPACING, SPACING);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    style.text_styles.insert(
        TextStyle::Heading,
        FontId::new(18.0, FontFamily::Proportional),
    );
    style
        .text_styles
        .insert(TextStyle::Body, FontId::new(13.0, FontFamily::Proportional));
    style.text_styles.insert(
        TextStyle::Small,
        FontId::new(11.0, FontFamily::Proportional),
    );
}

pub fn glass_tint() -> Color32 {
    // Faint darkening over the TRANSPARENT base so text stays readable
    // while DWM Acrylic/Mica shows through. Keep alpha minimal (40):
    // anything higher starts to cover the blur in the gutters.
    // With `--glass-debug`, window_chrome swaps this for a magenta tint
    // so transparency can be verified against a checker wallpaper.
    Color32::from_rgba_unmultiplied(0, 0, 0, 40)
}

/// Debug tint used only with `--glass-debug` / `BD_NEON_GLASS_DEBUG=1`:
/// magenta makes it obvious whether the surface is truly transparent
/// (wallpaper bleeds through) or composited opaque (flat magenta).
pub fn glass_debug_tint() -> Color32 {
    Color32::from_rgba_unmultiplied(255, 0, 255, 40)
}

pub fn title_bar_fill() -> Color32 {
    // Translucent (~43%) so DWM blur bleeds through the top bar.
    // Alphas must stay low for DWM bleed-through; opaque covers kill blur.
    Color32::from_rgba_unmultiplied(18, 21, 28, 110)
}

pub fn surface_elevated() -> Color32 {
    // Card fill: keep low so the DWM base blur bleeds through.
    Color32::from_rgba_unmultiplied(10, 12, 18, 110)
}

pub fn surface_hover() -> Color32 {
    // Hover fill: translucent, DWM bleed-through must survive hover.
    Color32::from_rgba_unmultiplied(18, 21, 30, 130)
}

pub fn surface_active() -> Color32 {
    // Active fill: translucent, DWM bleed-through must survive press.
    Color32::from_rgba_unmultiplied(23, 28, 40, 135)
}

pub fn muted() -> Color32 {
    // Lane / slot fill: lower alpha so gutters show the base blur.
    Color32::from_rgba_unmultiplied(18, 21, 28, 110)
}

pub fn popup_bg() -> Color32 {
    // Compat wrapper: frosted nested dialogs use `dialog_fill`.
    dialog_fill()
}

pub fn popup_shadow() -> egui::Shadow {
    // Compat wrapper: nested dialogs use the deeper `dialog_shadow`.
    dialog_shadow()
}

/// Nested fake-frosted dialog backdrop (Option B, no viewport).
/// Opaque enough to mute the board behind nested popups while staying
/// translucent so the DWM Acrylic/Mica base blur still bleeds through.
pub fn dialog_backdrop() -> Color32 {
    Color32::from_rgba_unmultiplied(0, 0, 0, 120)
}

/// Nested fake-frosted dialog fill: near-opaque dark slate. egui cannot
/// blur per-widget, so the alpha stack (backdrop 120a + fill 215a) fakes
/// frost over the DWM-blurred base.
pub fn dialog_fill() -> Color32 {
    Color32::from_rgba_unmultiplied(10, 12, 18, 215)
}

/// Nested dialog edge: white@32 catches light over the frosted fill.
pub fn dialog_stroke() -> Stroke {
    Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 32))
}

/// Nested dialog shadow: deeper than popups so stacked dialogs lift off
/// the dimmed board.
pub fn dialog_shadow() -> egui::Shadow {
    egui::Shadow {
        offset: [0, 16],
        blur: 48,
        spread: 0,
        color: Color32::from_rgba_unmultiplied(0, 0, 0, 110),
    }
}

/// Top inner-edge highlight for the frosted look: 1px white@40 line.
pub fn dialog_top_highlight() -> Color32 {
    Color32::from_rgba_unmultiplied(255, 255, 255, 40)
}

pub fn dialog_highlight_stroke() -> Stroke {
    Stroke::new(1.0_f32, dialog_top_highlight())
}

pub fn dialog_frame(style: &egui::Style) -> egui::Frame {
    egui::Frame::popup(style)
        .fill(dialog_fill())
        .stroke(dialog_stroke())
        .shadow(dialog_shadow())
        .corner_radius(radius_card())
}

/// Paint a 1px top inner-edge highlight inside a `dialog_frame` body.
/// Call first inside the dialog contents so the frosted edge catches
/// light. `Frame` cannot layer the highlight itself, so each dialog
/// body paints it explicitly.
pub fn paint_dialog_top_highlight(ui: &Ui) {
    let rect = ui.max_rect();
    ui.painter()
        .hline(rect.x_range(), rect.top() + 1.0, dialog_highlight_stroke());
}

pub const fn foreground() -> Color32 {
    Color32::from_rgb(240, 243, 248)
}

pub fn border() -> Color32 {
    Color32::from_rgba_unmultiplied(255, 255, 255, 18)
}

pub fn border_hover() -> Color32 {
    Color32::from_rgba_unmultiplied(255, 255, 255, 38)
}

pub fn hairline() -> Stroke {
    Stroke::new(1.0_f32, border())
}

pub fn glass_stroke() -> Stroke {
    Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 14))
}

pub const fn neon_cyan() -> Color32 {
    Color32::from_rgb(56, 189, 248)
}

pub const fn neon_magenta() -> Color32 {
    Color32::from_rgb(244, 114, 182)
}

pub const fn neon_amber() -> Color32 {
    Color32::from_rgb(251, 191, 36)
}

pub const fn neon_blue() -> Color32 {
    Color32::from_rgb(56, 189, 248)
}

pub const fn neon_green() -> Color32 {
    Color32::from_rgb(74, 222, 128)
}

pub const fn neon_violet() -> Color32 {
    Color32::from_rgb(168, 85, 247)
}

pub const fn text_dim() -> Color32 {
    Color32::from_rgb(115, 128, 148)
}

pub const fn dim_text() -> Color32 {
    text_dim()
}

pub const fn text_muted() -> Color32 {
    Color32::from_rgb(156, 169, 188)
}

pub fn flash_bg() -> Color32 {
    Color32::from_rgba_unmultiplied(56, 189, 248, 25)
}

pub fn card_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(surface_elevated())
        .stroke(hairline())
        .inner_margin(Margin::same(PAD_I8))
        .corner_radius(radius_card())
}

pub fn muted_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(muted())
        .stroke(hairline())
        .inner_margin(Margin::same(PAD_I8))
        .corner_radius(radius_card())
}

pub fn remaining_body<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    let size = ui.available_size();
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    child.set_clip_rect(rect);
    add_contents(&mut child)
}

pub fn pointer(response: Response) -> Response {
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn fill_width(ui: &mut Ui) {
    let w = ui.available_width();
    ui.set_min_width(w);
    ui.set_max_width(w);
    ui.set_width(w);
}

pub fn exact_card<R>(
    ui: &mut Ui,
    size: egui::Vec2,
    fill: Color32,
    pad: f32,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<R> {
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    ui.painter()
        .rect(rect, radius(), fill, hairline(), egui::StrokeKind::Inside);
    let inner = rect.shrink(pad);
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    child.set_clip_rect(inner);
    let inner_res = add_contents(&mut child);
    InnerResponse::new(inner_res, response)
}

pub fn glass_slot<R>(
    ui: &mut Ui,
    size: egui::Vec2,
    sense: Sense,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<R> {
    let (rect, response) = ui.allocate_exact_size(size, sense);
    ui.painter().rect(
        rect,
        radius(),
        muted(),
        glass_stroke(),
        egui::StrokeKind::Inside,
    );
    // Small inner padding (8 x-pad, 4 y-pad). Clip to the OUTER rect so
    // 13px body text / 14px icons are never cut by the clip rect.
    let inner = rect.shrink2(egui::vec2(8.0, 4.0));
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    child.set_clip_rect(rect);
    let inner_res = add_contents(&mut child);
    InnerResponse::new(inner_res, response)
}

/// Compact header-slot variant: same small padding + outer clip as
/// `glass_slot`, kept as a named helper so header call sites
/// (`repo_switcher`, `search_field`) stay explicit without affecting cards.
pub fn glass_slot_compact<R>(
    ui: &mut Ui,
    size: egui::Vec2,
    sense: Sense,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<R> {
    glass_slot(ui, size, sense, add_contents)
}
