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
    let mut paths = Vec::new();
    for (slot, index, _) in attempt.request.bound_images() {
        let argument = &attempt.arguments[slot];
        let value = if argument.is_array() {
            &argument[index]
        } else {
            argument
        };
        if let Some(path) = value["path"].as_str() {
            let path = Path::new(path);
            if path.parent() != Some(directory.as_path()) {
                return Err(io::Error::other("Input path escaped its owned directory"));
            }
            paths.push(path);
        }
    }
    // Validate the entire set first; an invalid later path must not delete an earlier copy.
    super::prepared::files::unmark(&directory, &attempt.id)?;
    for path in paths {
        match std::fs::remove_file(path) {
            Ok(()) => (),
            Err(error) if error.kind() == io::ErrorKind::NotFound => (),
            Err(error) => return Err(error),
        }
    }
    match std::fs::remove_dir(directory) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn attempt(arguments: serde_json::Value) -> Attempt {
        serde_json::from_value(json!({
            "id":"test", "sequence":1, "created_ms":0,
            "request":{"id":"test","tool":"fixture","action":"mark","revision":"r","params":{},"images":{"source":["a"],"references":["b","b"]}},
            "arguments":arguments,"declaration":{},"inputs":{},"wire":null,
            "execution":"completed","result":null,"preservation":{},"outputs":[],"journal":null,"notices":[]
        })).unwrap()
    }

    #[test]
    fn arrays_are_removed_but_unrelated_files_are_preserved() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path().canonicalize().unwrap();
        let dir = root.join("execution/test");
        std::fs::create_dir_all(&dir).unwrap();
        for name in ["scalar", "first", "second", "unrelated"] {
            std::fs::write(dir.join(name), b"fixture").unwrap();
        }
        let saved = attempt(
            json!({"source":{"path":dir.join("scalar")},"references":[{"path":dir.join("first")},{"path":dir.join("second")}]}),
        );
        assert!(discard(&root, &saved).is_err()); // Nonempty directory is deliberately retained.
        for name in ["scalar", "first", "second"] {
            assert!(!dir.join(name).exists(), "{name}");
        }
        assert!(dir.join("unrelated").exists());
        std::fs::remove_file(dir.join("unrelated")).unwrap();
        discard(&root, &saved).unwrap();
        assert!(!dir.exists());
    }

    #[test]
    fn escaped_array_path_refuses_cleanup_before_any_deletion() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path().canonicalize().unwrap();
        let dir = root.join("execution/test");
        std::fs::create_dir_all(&dir).unwrap();
        let inside = dir.join("first");
        let outside = root.join("original");
        std::fs::write(&inside, b"copy").unwrap();
        std::fs::write(&outside, b"original").unwrap();
        let saved = attempt(json!({"references":[{"path":inside},{"path":outside}]}));
        assert!(
            discard(&root, &saved)
                .unwrap_err()
                .to_string()
                .contains("escaped")
        );
        assert!(inside.exists());
        assert!(outside.exists());
    }
}
