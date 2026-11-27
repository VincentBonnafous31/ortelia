//! The session state of an application: the layout chosen by the user (size of the window,
//! order and widths of the columns, sort, filter, shares of the splits), written next to its
//! `app.json` when the window closes and restored at the next opening. `app.json` itself is
//! never written.
//!
//! This module holds the format of the file (serde types), its path, and its reading and
//! writing ([`load`], [`save`]): the only inputs and outputs. What goes in the file is computed
//! by [`crate::TableView::session`] and [`crate::AppState`].
//!
//! The file is read with indulgence: `app.json` may have changed since it was written, and a
//! newer version of Ortelia may have written more. Unlike `app.json`, unknown keys are ignored
//! (no `deny_unknown_fields`), and every key but `formatVersion` is optional.

use std::io;
use std::path::{Path, PathBuf};

use ortelia_config::SortOrder;
use serde::{Deserialize, Serialize};

/// The version of the format written, and the only one read.
pub const FORMAT_VERSION: &str = "1";
/// The longest version of a refused file cited in the log: the file may have been edited by
/// hand.
const MAX_LOGGED_VERSION: usize = 16;

/// The whole file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionFile {
    pub format_version: String,
    /// `None` until the window gives its size.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<WindowSession>,
    #[serde(default)]
    pub views: Vec<ViewSession>,
    /// Missing in the files written before the splits: the version of the format did not
    /// change, the key being new and optional.
    #[serde(default)]
    pub containers: Vec<ContainerSession>,
}

/// The size of the window, in points. A missing or `null` value is ignored.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowSession {
    pub width: Option<f32>,
    pub height: Option<f32>,
}

/// A container of the window, named by its identifier. Each kind of container has its own key
/// (`split` today; tab stacks later), like the widgets of a view.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ContainerSession {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub split: Option<SplitSession>,
}

/// The shares of the children of a split, in order. Written adding up to 1; read as weights,
/// made to add up to 1, and ignored if they are not one per child, each strictly positive.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SplitSession {
    pub weights: Vec<f32>,
}

/// A view, named by its identifier. Each kind of widget has its own key (`table` today; a form
/// or a tree later), next to the others.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ViewSession {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table: Option<TableSession>,
}

/// The layout of a table.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TableSession {
    /// In the order shown.
    pub columns: Vec<ColumnSession>,
    /// `None` (key missing): the sort of the presentation stays; `Some` replaces it, even
    /// empty (the user removed the sort).
    pub sort: Option<Vec<SortSession>>,
    pub filter: String,
}

/// A column, named by the path of its attribute, as in the presentation.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ColumnSession {
    pub path: String,
    pub width: Option<f32>,
}

/// A criterion of the sort.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SortSession {
    pub path: String,
    pub order: SortOrder,
}

/// The session state file of the application file `app_file`: same folder, same name, with
/// `.json` replaced by `.session.json` (`app.json` gives `app.session.json`). A file without
/// the `.json` extension gets `.session.json` after its whole name (`app.txt` gives
/// `app.txt.session.json`), so that the application file is never overwritten.
pub fn file_for(app_file: &Path) -> PathBuf {
    // `APP.JSON` too: names are not case-sensitive on Windows.
    let is_json = app_file
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("json"));
    if is_json {
        app_file.with_extension("session.json")
    } else {
        with_suffix(app_file, ".session.json")
    }
}

/// `path` with `suffix` added to its file name.
fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// Reads the session state file at `path`. `None` without file (no warning), or for a file
/// that cannot be read, is not valid JSON, has another version, or has a value of a wrong type
/// (a `WARN` then names the file, but never cites its values: the filter is a text typed by the
/// user).
pub fn load(path: &Path) -> Option<SessionFile> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return None,
        Err(error) => {
            // An `io::Error` holds no data of the file.
            tracing::warn!(path = %path.display(), %error, "session state ignored: cannot read it");
            return None;
        }
    };

    // First the version alone (serde skips the other keys): a file of version "2" is refused
    // as such, not for a key that version 2 changed.
    #[derive(Deserialize)]
    struct Version {
        #[serde(rename = "formatVersion", default)]
        format_version: serde_json::Value,
    }
    let version = match serde_json::from_str::<Version>(&text) {
        Ok(version) => version.format_version,
        Err(error) => {
            warn_invalid(path, &error);
            return None;
        }
    };
    if version.as_str() != Some(FORMAT_VERSION) {
        let version = match &version {
            serde_json::Value::String(text) => text.chars().take(MAX_LOGGED_VERSION).collect(),
            serde_json::Value::Null => "none".to_owned(),
            _ => "not a text".to_owned(),
        };
        tracing::warn!(
            path = %path.display(),
            %version,
            "session state ignored: unsupported format version"
        );
        return None;
    }

    match serde_json::from_str::<SessionFile>(&text) {
        Ok(file) => Some(file),
        Err(error) => {
            warn_invalid(path, &error);
            None
        }
    }
}

/// The warning of a file that serde_json refuses: its category and position, not its message,
/// which cites the value of a wrong type (`invalid type: string "dup", expected …`).
fn warn_invalid(path: &Path, error: &serde_json::Error) {
    let category = match error.classify() {
        serde_json::error::Category::Io => "read error",
        serde_json::error::Category::Syntax => "invalid JSON",
        serde_json::error::Category::Data => "value of a wrong type",
        serde_json::error::Category::Eof => "unexpected end of file",
    };
    tracing::warn!(
        path = %path.display(),
        line = error.line(),
        column = error.column(),
        "session state ignored: {category}"
    );
}

/// Writes `file` at `path`, through a temporary file of the same folder renamed afterwards: an
/// existing file is replaced whole, never left truncated. A failure is logged (`WARN`) and does
/// not stop the caller: the window closes anyway.
pub fn save(path: &Path, file: &SessionFile) {
    // `rename` replaces the target at once: `rename(2)` on Linux, `MoveFileExW` with
    // `MOVEFILE_REPLACE_EXISTING` on Windows. No `fsync`: after a power cut, the previous state
    // or none, which is acceptable for a layout. The temporary file is named after the process:
    // two instances closing together do not write into the same one.
    let temporary = with_suffix(path, &format!(".{}.tmp", std::process::id()));
    let result = serde_json::to_vec_pretty(file)
        .map_err(io::Error::other)
        .and_then(|bytes| std::fs::write(&temporary, bytes))
        .and_then(|()| std::fs::rename(&temporary, path));
    match result {
        Ok(()) => tracing::info!(path = %path.display(), "session state saved"),
        Err(error) => {
            // The temporary file may not exist: nothing to report then.
            let _ = std::fs::remove_file(&temporary);
            tracing::warn!(path = %path.display(), %error, "cannot save the session state");
        }
    }
}

#[cfg(test)]
mod tests;
