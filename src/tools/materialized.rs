//! Remove only inputs this host materialized, after terminal evidence or confirmed child exit.
use crate::library::attempts::Attempt;
use std::{io, path::Path};
pub(super) fn discard(root: &Path, attempt: &Attempt) -> io::Result<()> {
    if attempt.id.is_empty()
        || !attempt
            .id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_-".contains(&byte))
    {
        return Err(io::Error::other("Invalid owned attempt directory"));
    }
    let directory = root.join("execution").join(&attempt.id);
    if !directory.try_exists()? {
        return Ok(());
    }
    if directory.canonicalize()? != directory {
        return Err(io::Error::other("Input directory is no longer owned"));
    }
    for slot in attempt.request.images.keys() {
        if let Some(path) = attempt.arguments[slot]["path"].as_str() {
            let path = Path::new(path);
            if path.parent() != Some(directory.as_path()) {
                return Err(io::Error::other("Input path escaped its owned directory"));
            }
            match std::fs::remove_file(path) {
                Ok(()) => (),
                Err(error) if error.kind() == io::ErrorKind::NotFound => (),
                Err(error) => return Err(error),
            }
        }
    }
    match std::fs::remove_dir(directory) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}
