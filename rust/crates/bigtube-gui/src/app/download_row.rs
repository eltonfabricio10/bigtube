//! The `DownloadRow` widget: one visible row in the Downloads list, with its
//! transport buttons (pause/resume/cancel/retry) and progress state machine.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use adw::prelude::*;
use gtk::glib;

use bigtube_core::downloader::VideoDownloader;
use bigtube_core::progress::{Progress, ProgressFn, StatusCode};

use super::{a11y_label, location_label, parse_percent, set_row_filter_key, status_label};
use crate::i18n::tr;

/// One visible download (a row in the Downloads list).
#[derive(Clone)]
pub(crate) struct DownloadRow {
    pub container: gtk::Box,
    pub status: gtk::Label,
    pub detail: gtk::Label,
    pub progress: gtk::ProgressBar,
    pub pause: gtk::Button,
    pub cancel: gtk::Button,
    // Pencil shown only for a pending scheduled row: opens the schedule editor.
    pub edit: gtk::Button,
    pub btn_delete: gtk::Button,
    pub actions: gtk::Box,
    pub btn_folder: gtk::Button,
    pub btn_play: gtk::Button,
    pub btn_convert: gtk::Button,
    pub btn_favorite: gtk::Button,
    pub file_path: Rc<RefCell<String>>,
    pub artist: Rc<RefCell<String>>,
    // Shared across clones so buttons and the Started handler see the same state.
    pub downloader: Rc<RefCell<Option<Arc<VideoDownloader>>>>,
    pub progress_fn: Rc<RefCell<Option<ProgressFn>>>,
    pub is_paused: Rc<Cell<bool>>,
    // True once the download has errored: the pause button becomes a retry button.
    pub is_error: Rc<Cell<bool>>,
    // The persisted schedule id, while this row is a pending scheduled download
    // (lets the "Scheduled" management tab find and cancel/edit the live row).
    pub sched_id: Rc<RefCell<Option<String>>>,
    // Last shown progress fraction, to keep the bar monotonic: yt-dlp's percent
    // is derived from a fluctuating size *estimate* on some streams, so it can
    // briefly go backwards. We ignore small regressions but allow a large drop
    // (a real new phase, e.g. video→audio in a DASH merge).
    pub last_fraction: Rc<Cell<f64>>,
    // Active timer that pulses the bar while there's no real percent yet
    // (resolving the format / connecting), so it reads as "working" instead of a
    // stuck empty bar. Cleared once real progress arrives or the row is terminal.
    pub pulse_timer: Rc<RefCell<Option<glib::SourceId>>>,
}

impl DownloadRow {
    pub(crate) fn new(title: &str, file_path: &str, artist: &str) -> Self {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 4);
        // Tag the card so the downloads filter can match it by title/artist/path.
        set_row_filter_key(&container, &format!("{title} {artist} {file_path}"));
        container.add_css_class("card");
        container.set_margin_top(6);
        container.set_margin_bottom(6);
        container.set_margin_start(8);
        container.set_margin_end(8);
        // Inner padding so the card border doesn't hug the content.
        let pad = gtk::Box::new(gtk::Orientation::Vertical, 4);
        pad.set_margin_top(8);
        pad.set_margin_bottom(8);
        pad.set_margin_start(12);
        pad.set_margin_end(12);

        let header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let title_lbl = gtk::Label::new(Some(title));
        title_lbl.set_xalign(0.0);
        title_lbl.set_hexpand(true);
        title_lbl.set_ellipsize(gtk::pango::EllipsizeMode::End);
        title_lbl.add_css_class("heading");
        let status = gtk::Label::new(Some(&tr("Queued")));
        status.add_css_class("dim-label");
        status.add_css_class("caption");
        let pause = gtk::Button::from_icon_name("bigtube-media-playback-pause-symbolic");
        pause.add_css_class("flat");
        pause.set_tooltip_text(Some(&tr("Pause")));
        a11y_label(&pause, &tr("Pause"));
        let cancel = gtk::Button::from_icon_name("bigtube-process-stop-symbolic");
        cancel.add_css_class("flat");
        cancel.add_css_class("destructive-action");
        cancel.set_tooltip_text(Some(&tr("Cancel")));
        a11y_label(&cancel, &tr("Cancel"));
        // Edit pencil: shown only while this row is a pending scheduled download.
        let edit = gtk::Button::from_icon_name("bigtube-document-edit-symbolic");
        edit.add_css_class("flat");
        edit.set_tooltip_text(Some(&tr("Edit")));
        a11y_label(&edit, &tr("Edit"));
        edit.set_visible(false);
        // Per-row delete (asks history-only vs file too); wired in wire_row_footer.
        let btn_delete = gtk::Button::from_icon_name("bigtube-user-trash-symbolic");
        btn_delete.add_css_class("flat");
        btn_delete.set_tooltip_text(Some(&tr("Remove from list")));
        a11y_label(&btn_delete, &tr("Remove from list"));
        header.append(&title_lbl);
        header.append(&status);
        header.append(&edit);
        header.append(&pause);
        header.append(&cancel);
        header.append(&btn_delete);

        // Destination folder shown under the title ("Location: <folder>"); the
        // full path stays available as the tooltip.
        let path_lbl = gtk::Label::new(Some(&location_label(file_path)));
        path_lbl.set_xalign(0.0);
        path_lbl.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
        path_lbl.set_tooltip_text(Some(file_path));
        path_lbl.add_css_class("dim-label");
        path_lbl.add_css_class("caption");

        let progress = gtk::ProgressBar::new();
        progress.set_fraction(0.0);

        // Live transfer detail ("12.3MiB / 45.6MiB · 2.1MiB/s · ETA 00:15") while
        // running, and the media summary ("Video MP4 · h264 · 1920×1080 · …")
        // once done. Sits at the bottom-left, on the same row as the actions.
        let detail = gtk::Label::new(None);
        detail.set_xalign(0.0);
        detail.set_hexpand(true);
        detail.set_ellipsize(gtk::pango::EllipsizeMode::End);
        detail.add_css_class("dim-label");
        detail.add_css_class("caption");
        detail.set_visible(false);

        // Bottom row: status detail on the left, action buttons on the right.
        let footer = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        actions.set_halign(gtk::Align::End);
        // Revealed on completion (open folder / play / convert).
        actions.set_visible(false);
        let btn_folder = gtk::Button::from_icon_name("bigtube-folder-open-symbolic");
        btn_folder.add_css_class("flat");
        btn_folder.set_tooltip_text(Some(&tr("Open Folder")));
        a11y_label(&btn_folder, &tr("Open Folder"));
        let btn_play = gtk::Button::from_icon_name("bigtube-media-playback-start-symbolic");
        btn_play.add_css_class("flat");
        btn_play.set_tooltip_text(Some(&tr("Play Video")));
        a11y_label(&btn_play, &tr("Play Video"));
        let btn_convert = gtk::Button::from_icon_name("bigtube-emblem-synchronizing-symbolic");
        btn_convert.add_css_class("flat");
        btn_convert.set_tooltip_text(Some(&tr("Add to Converter")));
        a11y_label(&btn_convert, &tr("Add to Converter"));
        let btn_favorite = gtk::Button::from_icon_name("bigtube-emblem-favorite-symbolic");
        btn_favorite.add_css_class("flat");
        btn_favorite.set_tooltip_text(Some(&tr("Add to Favorites")));
        a11y_label(&btn_favorite, &tr("Add to Favorites"));
        actions.append(&btn_folder);
        actions.append(&btn_play);
        actions.append(&btn_favorite);
        actions.append(&btn_convert);
        footer.append(&detail);
        footer.append(&actions);

        pad.append(&header);
        pad.append(&path_lbl);
        pad.append(&progress);
        pad.append(&footer);
        container.append(&pad);

        let downloader: Rc<RefCell<Option<Arc<VideoDownloader>>>> = Rc::new(RefCell::new(None));
        let progress_fn: Rc<RefCell<Option<ProgressFn>>> = Rc::new(RefCell::new(None));
        let is_paused = Rc::new(Cell::new(false));
        let is_error = Rc::new(Cell::new(false));

        let slot = downloader.clone();
        let cancel_paused = is_paused.clone();
        let cancel_pf = progress_fn.clone();
        let cancel_fp = file_path.to_string();
        cancel.connect_clicked(move |_| {
            let Some(d) = slot.borrow().as_ref().cloned() else {
                return;
            };
            d.cancel();
            // A paused download has no live process, so the core won't emit a
            // Cancelled event and its partial files were kept for resume. Drive
            // the cancel to completion ourselves: clear the paused flag (so the
            // Cancelled below isn't swallowed as "Paused"), remove the leftover
            // partials/fragments, and reset the row via a synthetic Cancelled.
            if cancel_paused.get() {
                cancel_paused.set(false);
                bigtube_core::downloader::cleanup_download_artifacts(&cancel_fp);
                if let Some(cb) = cancel_pf.borrow().as_ref().cloned() {
                    cb(Progress::status(StatusCode::Cancelled));
                }
            }
        });

        // Pause / resume, or — after an error — retry. Both re-run the (blocking)
        // downloader on a thread via `resume`.
        let dl = downloader.clone();
        let pf = progress_fn.clone();
        let paused = is_paused.clone();
        let err = is_error.clone();
        let pause_btn = pause.clone();
        let status_c = status.clone();
        let progress_c = progress.clone();
        let cancel_c = cancel.clone();
        pause.connect_clicked(move |_| {
            let Some(d) = dl.borrow().as_ref().cloned() else {
                return;
            };
            if err.get() {
                // Retry a failed download: reset the row to a running look and
                // re-run from scratch.
                err.set(false);
                paused.set(false);
                pause_btn.set_icon_name("bigtube-media-playback-pause-symbolic");
                pause_btn.set_tooltip_text(Some(&tr("Pause")));
                status_c.set_text(&tr("Queued"));
                for c in ["success", "warning", "error"] {
                    progress_c.remove_css_class(c);
                }
                cancel_c.set_visible(true);
                cancel_c.set_sensitive(true);
                if let Some(cb) = pf.borrow().as_ref().cloned() {
                    std::thread::spawn(move || {
                        d.resume(&cb);
                    });
                }
                return;
            }
            if paused.get() {
                paused.set(false);
                pause_btn.set_icon_name("bigtube-media-playback-pause-symbolic");
                if let Some(cb) = pf.borrow().as_ref().cloned() {
                    std::thread::spawn(move || {
                        d.resume(&cb);
                    });
                }
            } else {
                paused.set(true);
                pause_btn.set_icon_name("bigtube-media-playback-start-symbolic");
                d.pause();
            }
        });

        Self {
            container,
            status,
            detail,
            progress,
            pause,
            cancel,
            edit,
            actions,
            btn_folder,
            btn_play,
            btn_convert,
            btn_favorite,
            btn_delete,
            file_path: Rc::new(RefCell::new(file_path.to_string())),
            artist: Rc::new(RefCell::new(artist.to_string())),
            downloader,
            progress_fn,
            is_paused,
            is_error,
            sched_id: Rc::new(RefCell::new(None)),
            last_fraction: Rc::new(Cell::new(0.0)),
            pulse_timer: Rc::new(RefCell::new(None)),
        }
    }

    /// Animate the progress bar as indeterminate (pulsing) while there's no real
    /// percent yet. Idempotent; stopped by [`stop_pulse`](Self::stop_pulse).
    pub(crate) fn start_pulse(&self) {
        if self.pulse_timer.borrow().is_some() {
            return;
        }
        self.progress.set_pulse_step(0.12);
        self.progress.pulse();
        let bar = self.progress.downgrade();
        let id = glib::timeout_add_local(std::time::Duration::from_millis(120), move || match bar
            .upgrade()
        {
            Some(b) => {
                b.pulse();
                glib::ControlFlow::Continue
            }
            None => glib::ControlFlow::Break,
        });
        self.pulse_timer.replace(Some(id));
    }

    /// Stop the indeterminate pulse (real progress arrived, or the row is done).
    pub(crate) fn stop_pulse(&self) {
        if let Some(id) = self.pulse_timer.take() {
            id.remove();
        }
    }

    pub(crate) fn update(&self, percent: Option<&str>, status: StatusCode, detail: Option<&str>) {
        // A pause terminates the yt-dlp process, surfacing as "Cancelled"; keep
        // the row interactive while the user has it paused.
        if self.is_paused.get() && status == StatusCode::Cancelled {
            self.stop_pulse();
            self.status.set_text(&tr("Paused"));
            self.set_progress_class("warning");
            return;
        }
        self.status.set_text(&status_label(status));
        // No bytes yet (resolving the format / connecting): pulse so the empty
        // bar reads as "working". Real progress below cancels it.
        if matches!(status, StatusCode::Starting | StatusCode::Resuming) {
            self.start_pulse();
        }
        if let Some(p) = percent {
            if let Some(f) = parse_percent(p) {
                self.stop_pulse();
                // Keep the bar monotonic against estimate jitter; allow a big
                // drop (>30%) through as a genuine new phase.
                let last = self.last_fraction.get();
                let f = if f < last && (last - f) < 0.30 {
                    last
                } else {
                    f
                };
                self.last_fraction.set(f);
                self.progress.set_fraction(f);
            }
        }
        // Live size/speed/ETA line (shown only while it carries data).
        if let Some(d) = detail.filter(|d| !d.is_empty()) {
            self.detail.set_text(d);
            self.detail.set_visible(true);
        }
        // The Cancel button only makes sense while a transfer is actually
        // running — hide it in the idle "Queued" state. (A pending *scheduled*
        // row keeps its own Cancel: it never reaches update() until it starts.)
        let in_progress = matches!(
            status,
            StatusCode::Starting
                | StatusCode::Downloading
                | StatusCode::Processing
                | StatusCode::Merging
                | StatusCode::Extracting
                | StatusCode::Resuming
        );
        if in_progress {
            self.cancel.set_visible(true);
            self.cancel.set_sensitive(true);
        } else if status == StatusCode::Queued {
            self.cancel.set_visible(false);
        }
        if status == StatusCode::Completed {
            self.mark_completed();
        } else if status.is_error() {
            // Errored: keep the row interactive — Cancel stays, and Pause becomes
            // a Retry button (circular arrow).
            self.stop_pulse();
            self.set_progress_class("error");
            self.is_error.set(true);
            self.pause.set_visible(true);
            self.pause.set_sensitive(true);
            self.pause.set_icon_name("bigtube-view-refresh-symbolic");
            self.pause.set_tooltip_text(Some(&tr("Retry")));
            self.cancel.set_visible(true);
            self.cancel.set_sensitive(true);
        } else if status == StatusCode::Cancelled {
            // A real cancel (not a pause): don't leave a dead "Cancelled" row —
            // reset it to the initial, restartable look.
            self.reset_to_initial();
        }
    }

    /// Return a cancelled row to its initial "Queued" appearance: empty bar, no
    /// status colour, and the pause button turned into a Retry that re-runs the
    /// download from scratch (the core clears its cancelled flag on resume).
    pub(crate) fn reset_to_initial(&self) {
        self.stop_pulse();
        self.is_error.set(true); // routes the pause button to the retry path
        self.is_paused.set(false);
        self.last_fraction.set(0.0);
        self.progress.set_fraction(0.0);
        self.set_progress_class("");
        self.detail.set_visible(false);
        self.actions.set_visible(false);
        self.status.set_text(&tr("Queued"));
        self.pause.set_visible(true);
        self.pause.set_sensitive(true);
        self.pause.set_icon_name("bigtube-view-refresh-symbolic");
        self.pause.set_tooltip_text(Some(&tr("Retry")));
        // Idle initial state — nothing to cancel; the X reappears once Retry
        // restarts the transfer (update() shows it on the next progress tick).
        self.cancel.set_visible(false);
    }

    /// Apply exactly one of the success/warning/error progress styles.
    pub(crate) fn set_progress_class(&self, class: &str) {
        for c in ["success", "warning", "error"] {
            self.progress.remove_css_class(c);
        }
        if !class.is_empty() {
            self.progress.add_css_class(class);
        }
    }

    /// Switch the row to its completed look: full bar, no transport, footer shown.
    pub(crate) fn mark_completed(&self) {
        self.stop_pulse();
        self.is_error.set(false);
        self.progress.set_fraction(1.0);
        self.set_progress_class("success");
        self.detail.set_visible(false);
        self.pause.set_visible(false);
        self.cancel.set_visible(false);
        // Mark the row terminal so "Clear" recognizes it as removable.
        self.pause.set_sensitive(false);
        self.cancel.set_sensitive(false);
        self.status.set_text(&status_label(StatusCode::Completed));
        // Only offer file actions if the output really exists.
        let exists = std::path::Path::new(&*self.file_path.borrow()).exists();
        self.actions.set_visible(exists);
    }
}
