#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod bd;
mod cache;
mod discover;
mod model;
mod platform;
mod store;
mod watch;

mod ui;

use eframe::egui;

fn pick_renderer() -> eframe::Renderer {
    // Renderer vs transparency (eframe 0.36, Windows):
    // - Glow (OpenGL, default) composites per-pixel alpha more reliably
    //   with Windows DWM: `clear_color()` TRANSPARENT lets Acrylic/Mica
    //   behind the egui layers bleed through. Slightly higher CPU draw
    //   overhead than Wgpu, but alpha compositing is what matters here.
    // - Wgpu often presents opaque on Windows DWM despite TRANSPARENT
    //   clear color (flat black gutters, no wallpaper bleed) — usually a
    //   GPU/driver swapchain quirk, not our DWM attributes.
    // Both renderers are compiled in via eframe's default features, so no
    // extra Cargo feature is needed. Override with
    // `--renderer=glow|wgpu` or `BD_NEON_RENDERER=glow|wgpu`.
    let mut asked: Option<String> = None;
    for arg in std::env::args().skip(1) {
        if let Some(value) = arg.strip_prefix("--renderer=") {
            asked = Some(value.to_lowercase());
        } else if arg == "--renderer=glow" || arg == "--use-glow" {
            asked = Some("glow".to_owned());
        }
    }
    if asked.is_none() {
        if let Ok(value) = std::env::var("BD_NEON_RENDERER") {
            asked = Some(value.to_lowercase());
        }
    }
    match asked.as_deref() {
        Some("glow") => {
            eprintln!("[glass] renderer=Glow (override)");
            eframe::Renderer::Glow
        }
        Some("wgpu") => {
            eprintln!("[glass] renderer=Wgpu (override)");
            eframe::Renderer::Wgpu
        }
        Some(other) => {
            eprintln!("[glass] unknown renderer '{other}', falling back to Glow");
            eframe::Renderer::Glow
        }
        None => {
            eprintln!("[glass] renderer=Glow (default)");
            eframe::Renderer::Glow
        }
    }
}

fn main() -> eframe::Result<()> {
    let renderer = pick_renderer();
    if platform::glass_debug_enabled() {
        eprintln!("[glass] --glass-debug ON: verbose HRESULT logs + magenta tint");
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1120.0, 720.0])
            .with_min_inner_size([720.0, 480.0])
            .with_decorations(false)
            .with_transparent(true),
        renderer,
        multisampling: 0,
        ..Default::default()
    };
    eframe::run_native(
        "bd-neon",
        options,
        Box::new(|cc| Ok(Box::new(app::BdNeonApp::new(cc)))),
    )
}
