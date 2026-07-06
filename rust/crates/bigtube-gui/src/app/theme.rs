//! Theme application (light/dark + accent CSS classes) and default window size.

use adw::prelude::*;

use bigtube_core::config;

/// A comfortable default window size derived from the primary monitor: ~75% of
/// its width and ~82% of its height, clamped to sane bounds. Falls back to a
/// fixed size when the monitor geometry isn't available.
pub(crate) fn comfortable_window_size() -> (i32, i32) {
    let geo = gtk::gdk::Display::default()
        .and_then(|d| d.monitors().item(0))
        .and_then(|o| o.downcast::<gtk::gdk::Monitor>().ok())
        .map(|m| m.geometry());
    match geo {
        Some(g) if g.width() > 0 && g.height() > 0 => {
            let w = ((g.width() as f64 * 0.75) as i32).clamp(900, 1600);
            let h = ((g.height() as f64 * 0.82) as i32).clamp(600, 1040);
            (w, h)
        }
        _ => (1000, 700),
    }
}

pub(crate) fn apply_theme(window: &adw::ApplicationWindow) {
    let mode = config::global()
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .get_string("theme_mode");
    let sm = adw::StyleManager::default();
    match mode.as_str() {
        "dark" => sm.set_color_scheme(adw::ColorScheme::ForceDark),
        "light" => sm.set_color_scheme(adw::ColorScheme::ForceLight),
        _ => sm.set_color_scheme(adw::ColorScheme::Default),
    }

    apply_theme_classes(window);
    // Accent CSS classes only style the widget subtree they're set on, so every
    // separate top-level window (player, playlist, dialogs, about) needs them
    // too — update all currently-open toplevels.
    let toplevels = gtk::Window::toplevels();
    for i in 0..toplevels.n_items() {
        if let Some(w) = toplevels
            .item(i)
            .and_then(|o| o.downcast::<gtk::Window>().ok())
        {
            apply_theme_classes(&w);
        }
    }
}

/// Apply the configured light/dark + accent CSS classes to a single widget
/// (any top-level window). Call this when creating a secondary window so it
/// matches the selected theme.
pub(crate) fn apply_theme_classes(widget: &impl IsA<gtk::Widget>) {
    let (mode, color) = {
        let cfg = config::global().read().unwrap_or_else(|e| e.into_inner());
        (cfg.get_string("theme_mode"), cfg.get_string("theme_color"))
    };
    let w = widget.as_ref();
    w.remove_css_class("light");
    w.remove_css_class("dark");
    for c in bigtube_core::enums::ThemeColor::ALL {
        w.remove_css_class(&format!("accent-{}", c.as_value()));
    }
    if mode == "dark" {
        w.add_css_class("dark");
    } else if mode == "light" {
        w.add_css_class("light");
    }
    if !color.is_empty() {
        w.add_css_class(&format!("accent-{color}"));
    }
}
