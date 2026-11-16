//! `file:` addresses, which is how a client names a document or a folder.

use std::path::{Path, PathBuf};

use lsp_types::Uri;

/// The path a `file:` address names, or `None` for another scheme, such as `untitled:`.
pub(crate) fn to_path(uri: &Uri) -> Option<PathBuf> {
    if !uri.scheme()?.eq_lowercase("file") {
        return None;
    }
    let path = uri.path().as_estr().decode().into_string().ok()?;
    if !cfg!(windows) {
        return Some(PathBuf::from(path.into_owned()));
    }
    // `file://server/share/a.py` is `\\server\share\a.py`, and `file:///C:/a.py` is `C:\a.py`.
    let host = uri
        .authority()
        .map(|authority| authority.host().as_str())
        .filter(|host| !host.is_empty() && !host.eq_ignore_ascii_case("localhost"));
    let path = match host {
        Some(host) => format!("//{host}{path}"),
        None => match path.strip_prefix('/') {
            Some(rest) if rest.as_bytes().get(1) == Some(&b':') => rest.to_owned(),
            _ => path.into_owned(),
        },
    };
    Some(PathBuf::from(path.replace('/', "\\")))
}

/// The digit of a value below 16, as a percent escape writes it.
fn hex(value: u8) -> char {
    char::from_digit(u32::from(value), 16)
        .expect("half a byte is below 16")
        .to_ascii_uppercase()
}

/// The `file:` address of an absolute path, or `None` when the path is not Unicode.
pub(crate) fn from_path(path: &Path) -> Option<Uri> {
    let path = path.to_str()?;
    let path = if cfg!(windows) {
        path.replace('\\', "/")
    } else {
        path.to_owned()
    };
    // `C:/a.py` becomes `file:///C:/a.py`, and `//server/share` becomes `file://server/share`.
    let mut uri = String::from(match path.strip_prefix("//") {
        Some(_) if cfg!(windows) => "file:",
        _ if path.starts_with('/') => "file://",
        _ => "file:///",
    });
    for byte in path.bytes() {
        let plain = byte.is_ascii_alphanumeric()
            || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'/')
            || (byte == b':' && cfg!(windows));
        if plain {
            uri.push(char::from(byte));
        } else {
            uri.extend(['%', hex(byte >> 4), hex(byte & 0xF)]);
        }
    }
    uri.parse().ok()
}
