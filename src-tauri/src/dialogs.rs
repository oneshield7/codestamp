use std::path::PathBuf;

use tauri::WebviewWindow;
use tauri_plugin_dialog::{DialogExt, FilePath};
use tokio::sync::oneshot;

/// Native "open file" dialog on top of `window`; `None` when the user cancels.
pub async fn pick_file(
    window: &WebviewWindow,
    title: &str,
    filter: &str,
    extensions: &[&str],
) -> Option<PathBuf> {
    let (sender, receiver) = oneshot::channel();
    window
        .dialog()
        .file()
        .set_parent(window)
        .set_title(title)
        .add_filter(filter, extensions)
        .pick_file(move |picked| {
            // If the command was dropped meanwhile, nobody needs the answer.
            let _ = sender.send(picked);
        });
    local_path(receiver.await)
}

/// Native "save file" dialog on top of `window`; `None` when the user cancels.
pub async fn pick_save_path(
    window: &WebviewWindow,
    title: &str,
    file_name: &str,
    filter: &str,
    extensions: &[&str],
) -> Option<PathBuf> {
    let (sender, receiver) = oneshot::channel();
    window
        .dialog()
        .file()
        .set_parent(window)
        .set_title(title)
        .set_file_name(file_name)
        .add_filter(filter, extensions)
        .save_file(move |picked| {
            // If the command was dropped meanwhile, nobody needs the answer.
            let _ = sender.send(picked);
        });
    local_path(receiver.await)
}

fn local_path(answer: Result<Option<FilePath>, oneshot::error::RecvError>) -> Option<PathBuf> {
    answer.ok().flatten().and_then(|path| path.into_path().ok())
}
