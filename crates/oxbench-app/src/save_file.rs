//! Saving generated output, such as a QR image or a key, through the system's save dialog.

use std::path::Path;

/// What happened after the save dialog closed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SaveOutcome {
    Saved { file_name: String },
    Failed { message: String },
}

impl SaveOutcome {
    pub fn describe(&self) -> String {
        match self {
            SaveOutcome::Saved { file_name } => format!("Saved {file_name}."),
            SaveOutcome::Failed { message } => message.clone(),
        }
    }
}

/// Who may read a saved file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Readers {
    Anyone,
    /// Owner only, as SSH requires for private keys.
    OwnerOnly,
}

/// A file type offered in the save dialog, such as `("PNG image", &["png"])`.
pub type FileFilter<'a> = (&'a str, &'a [&'a str]);

/// Asks where to save, then writes `bytes`. `None` when the dialog was cancelled.
pub fn save_with_dialog(
    title: &str,
    default_name: &str,
    filter: Option<FileFilter<'_>>,
    bytes: &[u8],
    readers: Readers,
) -> Option<SaveOutcome> {
    let mut dialog = rfd::FileDialog::new()
        .set_title(title)
        .set_file_name(default_name);
    if let Some((name, extensions)) = filter {
        dialog = dialog.add_filter(name, extensions);
    }
    let path = dialog.save_file()?;
    Some(write_file(&path, bytes, readers))
}

pub fn write_file(path: &Path, bytes: &[u8], readers: Readers) -> SaveOutcome {
    let file_name = path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    );
    let written = std::fs::write(path, bytes).and_then(|()| restrict_readers(path, readers));
    match written {
        Ok(()) => SaveOutcome::Saved { file_name },
        Err(error) => SaveOutcome::Failed {
            message: format!("Couldn't save {file_name}: {error}."),
        },
    }
}

#[cfg(unix)]
fn restrict_readers(path: &Path, readers: Readers) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    if readers == Readers::Anyone {
        return Ok(());
    }
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
}

/// Windows files inherit their folder's access rules, which already limit them to the user.
#[cfg(not(unix))]
fn restrict_readers(_path: &Path, _readers: Readers) -> std::io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_files_and_restricts_private_ones() {
        let directory = std::env::temp_dir().join(format!("oxbench-save-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("id_test");
        let outcome = write_file(&path, b"secret", Readers::OwnerOnly);
        assert_eq!(outcome.describe(), "Saved id_test.");
        assert_eq!(std::fs::read(&path).unwrap(), b"secret");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        let missing = write_file(&directory.join("no/such/dir/file"), b"x", Readers::Anyone);
        assert!(matches!(missing, SaveOutcome::Failed { .. }));
        std::fs::remove_dir_all(&directory).unwrap();
    }
}
