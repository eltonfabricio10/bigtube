//! Format-selection dialog, mirroring `format_dialog.py`. Lists the parsed
//! video/audio formats; picking one invokes `on_pick(format_id, ext, subs)`.

use std::cell::Cell;
use std::rc::Rc;

use adw::prelude::*;

use bigtube_core::downloader::{FormatOption, ParsedInfo, SubtitleOverride};

use crate::i18n::tr;

/// Callback: `(format_id, ext, subtitle_override)` — download now. The override
/// is `None` when the user left subtitles on "Follow Settings".
pub type PickFn = Rc<dyn Fn(String, String, Option<SubtitleOverride>)>;
/// Callback: same fields — open the schedule flow for this format.
pub type ScheduleFn = Rc<dyn Fn(String, String, Option<SubtitleOverride>)>;
/// Callback: the dialog was closed without picking a format (go back).
pub type CloseFn = Rc<dyn Fn()>;
/// Reads the current subtitle choice from the dialog's subtitle section.
type SubQuery = Rc<dyn Fn() -> Option<SubtitleOverride>>;

pub fn show(
    parent: &impl IsA<gtk::Window>,
    info: &ParsedInfo,
    audio_only: bool,
    on_pick: PickFn,
    on_schedule: ScheduleFn,
    on_close: CloseFn,
) {
    // Normal sources show Video + Audio side by side (two columns, one screen,
    // no Video/Audio prompt); YouTube Music shows the single Audio column.
    let two_col = !audio_only && !info.videos.is_empty();

    let win = adw::Window::builder()
        .transient_for(parent)
        .modal(true)
        .default_width(if two_col { 860 } else { 520 })
        .title(tr("Select Quality"))
        .build();
    crate::app::apply_theme_classes(&win);

    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());

    // True once a format is picked/scheduled, so closing the window then doesn't
    // count as "cancelled".
    let picked = Rc::new(Cell::new(false));

    // Per-video subtitle choice (languages this video actually offers). The
    // query closure is read at pick time; None when the section isn't shown or
    // the user kept "Follow Settings".
    let (sub_group, sub_query) = subtitle_section(info);

    // Builds one column's PreferencesGroup from a list of formats.
    let make_group = |title: String, description: Option<String>, formats: &[FormatOption]| {
        let builder = adw::PreferencesGroup::builder().title(title);
        let group = match description {
            Some(d) => builder.description(d).build(),
            None => builder.build(),
        };
        for f in formats {
            group.add(&format_row(
                f,
                &win,
                &on_pick,
                &on_schedule,
                &picked,
                &sub_query,
            ));
        }
        group
    };

    // When every audio row is a virtual "convert" option, the source had no
    // separate audio track — tell the user the audio is extracted/converted.
    let audio_desc = (!info.audios.is_empty()
        && info.audios.iter().all(|f| f.codec.ends_with("_convert")))
    .then(|| {
        tr("This source has no separate audio track. The options below extract and convert its audio.")
    });

    if two_col {
        // Two columns: video (left) + audio (right), equal width and top-aligned.
        // Only the VIDEO column scrolls (capped height), so its long codec ladder
        // doesn't blow up the dialog while the short audio column stays fully
        // visible. No outer scroll — the dialog height tracks the taller column.
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 18);
        row.set_margin_top(12);
        row.set_margin_bottom(12);
        row.set_margin_start(12);
        row.set_margin_end(12);
        row.set_homogeneous(true);
        // Subtitle section (when the video has any) sits under both columns.
        let outer = gtk::Box::new(gtk::Orientation::Vertical, 0);

        let video = make_group(tr("Video Formats"), None, &info.videos);
        video.set_valign(gtk::Align::Start);
        let video_scroll = gtk::ScrolledWindow::new();
        video_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        video_scroll.set_propagate_natural_height(true);
        video_scroll.set_max_content_height(460);
        video_scroll.set_hexpand(true);
        video_scroll.set_valign(gtk::Align::Start);
        video_scroll.set_child(Some(&video));
        crate::app::redraw_on_scroll(&video_scroll);
        row.append(&video_scroll);

        if !info.audios.is_empty() {
            let audio = make_group(tr("Audio Formats"), audio_desc, &info.audios);
            audio.set_valign(gtk::Align::Start);
            audio.set_hexpand(true);
            row.append(&audio);
        }
        outer.append(&row);
        if let Some(g) = &sub_group {
            g.set_margin_bottom(12);
            g.set_margin_start(12);
            g.set_margin_end(12);
            outer.append(g);
        }
        toolbar.set_content(Some(&outer));
    } else {
        // Single column (YouTube Music audio, or fallback) inside one scroll that
        // grows with the content up to a cap, then scrolls.
        let page = gtk::Box::new(gtk::Orientation::Vertical, 18);
        page.set_margin_top(12);
        page.set_margin_bottom(12);
        page.set_margin_start(12);
        page.set_margin_end(12);

        if !info.audios.is_empty() {
            let audio = make_group(tr("Audio Formats"), audio_desc, &info.audios);
            audio.set_hexpand(true);
            page.append(&audio);
        } else {
            // Empty fallback so the dialog never renders blank.
            let group = adw::PreferencesGroup::new();
            group.add(
                &adw::ActionRow::builder()
                    .title(tr("No formats found"))
                    .build(),
            );
            page.append(&group);
        }
        if let Some(g) = &sub_group {
            page.append(g);
        }

        let scrolled = gtk::ScrolledWindow::new();
        scrolled.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scrolled.set_propagate_natural_height(true);
        scrolled.set_max_content_height(640);
        scrolled.set_child(Some(&page));
        crate::app::redraw_on_scroll(&scrolled);
        toolbar.set_content(Some(&scrolled));
    }
    win.set_content(Some(&toolbar));

    // Closing without a pick → notify the caller.
    {
        let on_close = on_close.clone();
        let picked = picked.clone();
        win.connect_close_request(move |_| {
            if !picked.get() {
                on_close();
            }
            gtk::glib::Propagation::Proceed
        });
    }
    win.present();
}

/// Build the per-video subtitle section: an expander with a mode combo
/// ("Follow Settings" + the four explicit modes), one checkbox per language
/// the video actually offers, and an auto-captions switch. Returns `None`
/// (plus an always-`None` query) when the video has no subtitles at all.
fn subtitle_section(info: &ParsedInfo) -> (Option<adw::PreferencesGroup>, SubQuery) {
    if info.subs.is_empty() && !info.has_auto_subs {
        return (None, Rc::new(|| None));
    }
    let (g_langs, g_auto) = {
        let cfg = bigtube_core::config::global()
            .read()
            .unwrap_or_else(|e| e.into_inner());
        (
            cfg.get_string("subtitle_langs"),
            cfg.get_bool("subtitle_auto"),
        )
    };
    let defaults: Vec<String> = g_langs
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();

    let group = adw::PreferencesGroup::builder()
        .title(tr("Subtitles"))
        .build();
    let expander = adw::ExpanderRow::builder()
        .title(tr("Subtitles for this download"))
        .subtitle(tr("Follow Settings"))
        .build();

    // Mode: index 0 follows the global Settings; 1..4 are explicit overrides.
    const MODES: [&str; 5] = ["", "off", "embed", "file", "both"];
    let mode_labels = [
        tr("Follow Settings"),
        tr("Off"),
        tr("Embed in video"),
        tr("Separate file"),
        tr("Embed + file"),
    ];
    let mode_row = adw::ComboRow::builder().title(tr("Mode")).build();
    mode_row.set_model(Some(&gtk::StringList::new(
        &mode_labels.iter().map(String::as_str).collect::<Vec<_>>(),
    )));
    mode_row.set_selected(0);
    // Mirror the choice on the collapsed expander's subtitle line.
    {
        let expander = expander.clone();
        let labels = mode_labels.clone();
        mode_row.connect_selected_notify(move |r| {
            let i = (r.selected() as usize).min(labels.len() - 1);
            expander.set_subtitle(&labels[i]);
        });
    }
    expander.add_row(&mode_row);

    // One checkbox per manually-authored language; pre-checked when it's in
    // the user's configured language list.
    let checks: Rc<Vec<(String, gtk::CheckButton)>> = Rc::new(
        info.subs
            .iter()
            .map(|lang| {
                let chk = gtk::CheckButton::with_label(lang);
                let base = lang.split('-').next().unwrap_or(lang).to_lowercase();
                chk.set_active(defaults.contains(&lang.to_lowercase()) || defaults.contains(&base));
                (lang.clone(), chk)
            })
            .collect(),
    );
    if !checks.is_empty() {
        let langs_row = adw::ActionRow::builder().title(tr("Languages")).build();
        let flow = gtk::FlowBox::new();
        flow.set_selection_mode(gtk::SelectionMode::None);
        flow.set_max_children_per_line(6);
        flow.set_column_spacing(10);
        flow.set_valign(gtk::Align::Center);
        for (_, chk) in checks.iter() {
            flow.append(chk);
        }
        langs_row.add_suffix(&flow);
        expander.add_row(&langs_row);
    }

    // Auto-captions switch (shown only when the video has them).
    let auto_switch = if info.has_auto_subs {
        let row = adw::ActionRow::builder()
            .title(tr("Include auto-generated"))
            .build();
        let sw = gtk::Switch::new();
        sw.set_active(g_auto);
        sw.set_valign(gtk::Align::Center);
        row.add_suffix(&sw);
        row.set_activatable_widget(Some(&sw));
        expander.add_row(&row);
        Some(sw)
    } else {
        None
    };

    group.add(&expander);

    let query: SubQuery = Rc::new(move || {
        let idx = mode_row.selected() as usize;
        if idx == 0 || idx >= MODES.len() {
            return None; // follow the global Settings
        }
        let picked: Vec<String> = checks
            .iter()
            .filter(|(_, c)| c.is_active())
            .map(|(l, _)| l.clone())
            .collect();
        let langs = if picked.is_empty() {
            g_langs.clone() // auto-only video (or nothing checked): user's list
        } else {
            picked.join(",")
        };
        Some(SubtitleOverride {
            mode: MODES[idx].to_string(),
            langs,
            auto: auto_switch
                .as_ref()
                .map(|s| s.is_active())
                .unwrap_or(g_auto),
        })
    });
    (Some(group), query)
}

/// Pretty, vendor-neutral codec name for display (avc1 → H.264, mp4a → AAC…).
fn codec_display(codec: &str) -> String {
    let c = codec.to_lowercase();
    if c.contains("avc") || c.contains("h264") {
        "H.264".into()
    } else if c.contains("hev") || c.contains("h265") {
        "H.265".into()
    } else if c.contains("vp9") || c.contains("vp09") {
        "VP9".into()
    } else if c.contains("vp8") {
        "VP8".into()
    } else if c.contains("av01") || c.contains("av1") {
        "AV1".into()
    } else if c.contains("mp4a") || c.contains("aac") {
        "AAC".into()
    } else if c.contains("opus") {
        "Opus".into()
    } else if c.contains("vorbis") {
        "Vorbis".into()
    } else if c.contains("flac") {
        "FLAC".into()
    } else if c.contains("mp3") {
        "MP3".into()
    } else if c.contains("eac3") || c.contains("ac3") {
        "AC-3".into()
    } else if codec.is_empty() {
        String::new()
    } else {
        codec.to_uppercase()
    }
}

/// Compose the row title from the structured format fields, translating the few
/// human words (the codec/ext tokens are proper nouns and stay as-is). Built in
/// the GUI — not the core — so every language gets a localized label.
fn display_label(f: &FormatOption) -> String {
    // Virtual rows, identified by their synthetic codec markers.
    if f.codec == "mkv_merge" {
        return format!("{} · MKV ({}p)", tr("Best"), f.resolution);
    }
    if f.codec == "unknown" {
        return tr("Best available quality");
    }
    if f.codec.ends_with("_convert") {
        return format!("{} {}", tr("Convert to"), f.ext.to_uppercase());
    }
    if f.kind == "audio" {
        let mut s = codec_display(&f.codec);
        let kbps = f.quality as i64;
        if kbps > 0 {
            if !s.is_empty() {
                s.push_str(" · ");
            }
            s.push_str(&format!("{kbps} kbps"));
        }
        if !f.ext.is_empty() {
            s.push_str(&format!(" ({})", f.ext));
        }
        return s;
    }
    // Real video stream: "1080p 60fps · AV1 (webm)".
    let mut s = format!("{}p", f.resolution);
    if f.fps > 30 {
        s.push_str(&format!(" {}fps", f.fps));
    }
    let cd = codec_display(&f.codec);
    if !cd.is_empty() {
        s.push_str(&format!(" · {cd}"));
    }
    if !f.ext.is_empty() {
        s.push_str(&format!(" ({})", f.ext));
    }
    s
}

fn format_row(
    f: &FormatOption,
    win: &adw::Window,
    on_pick: &PickFn,
    on_schedule: &ScheduleFn,
    picked: &Rc<Cell<bool>>,
    sub_query: &SubQuery,
) -> adw::ActionRow {
    // Virtual "convert" rows have no real size — show a meaningful note instead.
    let subtitle = if f.codec.ends_with("_convert") || f.codec == "unknown" {
        tr("Best available quality")
    } else {
        f.size.clone()
    };
    let row = adw::ActionRow::builder()
        .title(display_label(f))
        .subtitle(subtitle)
        .build();

    let suffix = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    suffix.set_valign(gtk::Align::Center);

    // For video picks, use a height-aware selector so an unavailable exact id
    // falls back to the chosen resolution instead of silently dropping to ~360p.
    let sel_id = if f.kind == "video" {
        bigtube_core::downloader::video_selector(&f.id, f.resolution, &f.codec)
    } else {
        f.id.clone()
    };

    // Schedule for later.
    let schedule = gtk::Button::from_icon_name("bigtube-alarm-symbolic");
    schedule.add_css_class("flat");
    schedule.set_valign(gtk::Align::Center);
    schedule.set_tooltip_text(Some(&tr("Schedule Download")));
    {
        let id = sel_id.clone();
        let ext = f.ext.clone();
        let on_schedule = on_schedule.clone();
        let win = win.clone();
        let picked = picked.clone();
        let sub_query = sub_query.clone();
        schedule.connect_clicked(move |_| {
            picked.set(true);
            on_schedule(id.clone(), ext.clone(), sub_query());
            win.close();
        });
    }

    // Download now.
    let btn = gtk::Button::with_label(&tr("Download"));
    btn.add_css_class("suggested-action");
    btn.add_css_class("pill");
    btn.set_valign(gtk::Align::Center);
    {
        let id = sel_id.clone();
        let ext = f.ext.clone();
        let on_pick = on_pick.clone();
        let win = win.clone();
        let picked = picked.clone();
        let sub_query = sub_query.clone();
        btn.connect_clicked(move |_| {
            picked.set(true);
            on_pick(id.clone(), ext.clone(), sub_query());
            win.close();
        });
    }

    suffix.append(&schedule);
    suffix.append(&btn);
    row.add_suffix(&suffix);
    row
}
