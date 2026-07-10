//! Backup / restore / reset of the on-disk stores, wired to the Settings page
//! buttons: export & import a full backup, clear search history, wipe finished
//! items, and the full "reset all data + restart" flow.

use std::rc::Rc;

use adw::prelude::*;

use bigtube_core::config;

use super::{
    apply_theme, apply_theme_classes, config_saver, load_converter_history, load_download_history,
    search_history_path, AppState,
};
use crate::i18n::tr;

/// Delete the on-disk history / finished-item stores, used by the "Clear All
/// Data on Exit" setting. Deliberately narrower than `reset_all`: the config
/// and `favorites.json` survive — favorites are curated bookmarks, not
/// "finished items", and wiping them on every exit would be hostile.
pub(crate) fn wipe_finished_data() {
    let dir = bigtube_core::paths::config_dir();
    for name in [
        "history.json",
        "search_history.json",
        "converter_history.json",
        "scheduled_downloads.json",
        "converter_pending.json",
        "playlist_cache.json",
    ] {
        let f = dir.join(name);
        if f.exists() {
            let _ = std::fs::remove_file(&f);
        }
    }
}

/// Export a full backup: config + every history and the playlist cache, bundled
/// into one JSON file the user picks.
pub(crate) fn export_history(state: &Rc<AppState>) {
    let Some(window) = state.window.borrow().clone() else {
        return;
    };
    // Flush any debounced config write so the bundle captures the latest values.
    config_saver().flush();
    let dialog = gtk::FileDialog::builder()
        .title(tr("Export Backup"))
        .initial_name("bigtube_backup.json")
        .build();
    let state = state.clone();
    dialog.save(Some(&window), gtk::gio::Cancellable::NONE, move |res| {
        if let Ok(file) = res {
            if let Some(path) = file.path() {
                let bundle = bigtube_core::backup::build_backup(&bigtube_core::paths::config_dir());
                let ok = match serde_json::to_string_pretty(&bundle) {
                    Ok(s) => match std::fs::write(&path, s) {
                        Ok(()) => true,
                        Err(e) => {
                            tracing::error!("Failed to write backup to {}: {e}", path.display());
                            false
                        }
                    },
                    Err(e) => {
                        tracing::error!("Failed to serialize backup: {e}");
                        false
                    }
                };
                state.toast(&tr(if ok {
                    "Backup exported successfully!"
                } else {
                    "Failed to export backup"
                }));
            }
        }
    });
}

/// Import a full backup: restore every file to disk, then reload the config and
/// re-apply the theme live. (History/converter lists are read on the next visit
/// or restart.)
pub(crate) fn import_history(state: &Rc<AppState>) {
    let Some(window) = state.window.borrow().clone() else {
        return;
    };
    // Importing rebuilds the download/converter lists from disk; doing that
    // under an active download would orphan its row (progress messages no
    // longer find it — no cancel button, never recorded in history). Refuse
    // up front instead of corrupting the session.
    if !bigtube_core::download_manager::global().is_idle() || state.has_active_conversion() {
        state.toast(&tr(
            "Finish or cancel active downloads and conversions before importing a backup",
        ));
        return;
    }
    let dialog = gtk::FileDialog::builder()
        .title(tr("Import Backup"))
        .build();
    let state = state.clone();
    dialog.open(Some(&window), gtk::gio::Cancellable::NONE, move |res| {
        if let Ok(file) = res {
            if let Some(path) = file.path() {
                let parsed = std::fs::read_to_string(&path)
                    .map_err(|e| e.to_string())
                    .and_then(|s| {
                        serde_json::from_str::<serde_json::Value>(&s).map_err(|e| e.to_string())
                    });
                let msg = match parsed {
                    Ok(bundle) => {
                        match bigtube_core::backup::restore_backup(
                            &bigtube_core::paths::config_dir(),
                            &bundle,
                        ) {
                            Some(_) => {
                                // Reload config from the restored file and re-theme.
                                if let Ok(mut c) = config::global().write() {
                                    c.load();
                                }
                                if let Some(w) = state.window.borrow().clone() {
                                    apply_theme(&w);
                                }
                                // Rebuild the visible history lists from the restored files.
                                reload_history_views(&state);
                                "Backup imported successfully!"
                            }
                            None => {
                                tracing::warn!(
                                    "Backup rejected (not a valid bigtube backup): {}",
                                    path.display()
                                );
                                "Invalid backup file"
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Could not read backup {}: {e}", path.display());
                        "Error reading backup file"
                    }
                };
                state.toast(&tr(msg));
            }
        }
    });
}

/// Rebuild the download and converter history lists from disk — used after an
/// import so the restored entries show without a restart. Clears the current
/// rows first so nothing is duplicated. Scheduled-download timers are left to
/// re-arm on the next launch (re-arming live would double existing timers).
fn reload_history_views(state: &Rc<AppState>) {
    while let Some(c) = state.downloads_box.first_child() {
        state.downloads_box.remove(&c);
    }
    state.download_rows.borrow_mut().clear();
    load_download_history(state);

    while let Some(c) = state.converter_box.first_child() {
        state.converter_box.remove(&c);
    }
    load_converter_history(state);
}

pub(crate) fn clear_search_history(state: &Rc<AppState>) {
    let Some(window) = state.window.borrow().clone() else {
        return;
    };
    let dialog = adw::MessageDialog::new(
        Some(&window),
        Some(&tr("Clear search history?")),
        Some(&tr("Delete all previous search entries")),
    );
    dialog.add_response("cancel", &tr("Cancel"));
    dialog.add_response("clear", &tr("Clear"));
    dialog.set_response_appearance("clear", adw::ResponseAppearance::Destructive);
    dialog.set_default_response(Some("cancel"));
    dialog.set_close_response("cancel");
    apply_theme_classes(&dialog);

    let state = state.clone();
    dialog.connect_response(None, move |dlg, resp| {
        dlg.close();
        if resp == "clear" {
            bigtube_core::search_history::SearchHistory::new(search_history_path()).clear();
            state.toast(&tr("History cleared successfully!"));
        }
    });
    dialog.present();
}

pub(crate) fn reset_all_data(state: &Rc<AppState>) {
    let Some(window) = state.window.borrow().clone() else {
        return;
    };
    let confirm = adw::MessageDialog::new(
        Some(&window),
        Some(&tr("Reset all app data?")),
        Some(&tr(
            "This permanently deletes all settings, history and scheduled downloads. The app will restart.",
        )),
    );
    confirm.add_response("cancel", &tr("Cancel"));
    confirm.add_response("reset", &tr("Reset & Restart"));
    confirm.set_response_appearance("reset", adw::ResponseAppearance::Destructive);
    confirm.set_default_response(Some("cancel"));
    confirm.set_close_response("cancel");
    apply_theme_classes(&confirm);

    let window_for_info = window.clone();
    confirm.connect_response(None, move |dlg, resp| {
        dlg.close();
        if resp != "reset" {
            return;
        }
        // Wipe config + every on-disk store (history, search, converter,
        // scheduled). reset_all() recreates the (now-default) config dir.
        config::global()
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .reset_all();

        // Confirm to the user, then restart on close so the fresh process loads
        // the default state (matches the dialog's promise).
        let info = adw::MessageDialog::new(
            Some(&window_for_info),
            Some(&tr("Done")),
            Some(&tr(
                "All application data has been cleared. The app will now restart.",
            )),
        );
        info.add_response("ok", &tr("Restart Now"));
        info.set_default_response(Some("ok"));
        info.set_close_response("ok");
        apply_theme_classes(&info);
        info.connect_response(None, |dlg, _| {
            dlg.close();
            restart_app();
        });
        info.present();
    });
    confirm.present();
}

/// Re-launch the application from scratch (after a full data reset). Uses
/// `exec()` to replace the current process image: the single-instance D-Bus
/// socket is close-on-exec, so its name is released and the fresh process takes
/// over instead of just forwarding `activate` to the dying one.
fn restart_app() {
    use std::os::unix::process::CommandExt;
    // Persist any debounced config write before replacing the process image.
    config_saver().flush();
    let Ok(exe) = std::env::current_exe() else {
        std::process::exit(0);
    };
    let args: Vec<String> = std::env::args().skip(1).collect();
    // exec() only returns if it failed; otherwise it never comes back.
    let err = std::process::Command::new(exe).args(args).exec();
    tracing::error!("restart exec failed: {err}");
    std::process::exit(0);
}
