//! Clipboard monitor: watch for copied links and offer to search/download them.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;

use bigtube_core::config;

use super::{apply_theme_classes, AppState};
use crate::i18n::tr;

pub(crate) fn start_clipboard_monitor(state: &Rc<AppState>) {
    use bigtube_core::validators::is_valid_url;

    let Some(window) = state.window.borrow().clone() else {
        return;
    };
    let clipboard = window.clipboard();
    let win = window.clone();
    let state = state.clone();
    let last = Rc::new(RefCell::new(String::new()));
    // True while the prompt is open, so we don't stack dialogs each tick.
    let prompting = Rc::new(Cell::new(false));

    // React to actual clipboard changes instead of polling once a second for the
    // whole session. The `changed` signal fires only when the clipboard contents
    // are replaced, so there's no steady background wakeup + allocation.
    clipboard.connect_changed(move |clipboard| {
        // Respect the live setting.
        if !config::global()
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get_bool("monitor_clipboard")
        {
            return;
        }
        if prompting.get() {
            return;
        }
        // Content set by this very app (e.g. a result row's "Copy URL" button):
        // prompting "Link detected — paste & search?" for a URL the user just
        // copied FROM the app would be absurd.
        if clipboard.is_local() {
            return;
        }
        let state = state.clone();
        let last = last.clone();
        let win = win.clone();
        let prompting = prompting.clone();
        clipboard.read_text_async(gtk::gio::Cancellable::NONE, move |res| {
            if let Ok(Some(text)) = res {
                let text = text.to_string();
                if text != *last.borrow() && is_valid_url(&text) {
                    last.replace(text.clone());
                    prompt_paste_link(&state, &win, text, prompting);
                }
            }
        });
    });
}

/// Ask whether to paste a clipboard link into the search and run it.
fn prompt_paste_link(
    state: &Rc<AppState>,
    window: &adw::ApplicationWindow,
    url: String,
    prompting: Rc<Cell<bool>>,
) {
    prompting.set(true);
    let dialog = adw::MessageDialog::new(
        Some(window),
        Some(&tr("Link detected")),
        Some(&format!(
            "{}\n\n{url}",
            tr("Paste this link in the search and download it?")
        )),
    );
    dialog.add_response("no", &tr("Not Now"));
    dialog.add_response("yes", &tr("Paste & Search"));
    dialog.set_response_appearance("yes", adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some("yes"));
    dialog.set_close_response("no");
    apply_theme_classes(&dialog);

    let state = state.clone();
    dialog.connect_response(None, move |dlg, resp| {
        dlg.close();
        prompting.set(false);
        if resp == "yes" {
            if let Some(f) = state.paste_and_search.borrow().as_ref() {
                f(url.clone());
            }
        }
    });
    dialog.present();
}
