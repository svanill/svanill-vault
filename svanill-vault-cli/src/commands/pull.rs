use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct SanitizeError {
    bad_filename: String,
}

impl fmt::Display for SanitizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "cannot extract a proper filename from {}",
            self.bad_filename
        )
    }
}

impl Error for SanitizeError {}

/// Given a string, try to create a PathBuf with what could be seen as a filename
///
/// ```
/// # use std::path::PathBuf;
/// # use svanill_vault_cli::commands::pull::sanitize_possible_filename;
/// # use svanill_vault_cli::commands::pull::SanitizeError;
/// let possible_filename = "/foo/bar/baz";
///
/// assert_eq!(
///     PathBuf::from("baz"),
///     sanitize_possible_filename(possible_filename)?
/// );
/// # Ok::<(), SanitizeError>(())
/// ```
///
pub fn sanitize_possible_filename(filename: &str) -> Result<PathBuf, SanitizeError> {
    // attempt to convert the remote name to a filename
    Path::new(filename)
        .file_name()
        .map(PathBuf::from)
        .ok_or_else(|| SanitizeError {
            bad_filename: filename.to_owned(),
        })
}
