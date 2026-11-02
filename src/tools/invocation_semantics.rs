//! Cross-field invocation checks after the exact result schema and run association checks.
use super::{Error, Result};
use serde_json::Value;

pub fn validate(result: &Value, images_profile: bool) -> Result<()> {
    let invocation = &result["invocation"];
    // The schema permits this only for invalid inputs that never resolved an invocation.
    if invocation.is_null() {
        return Ok(());
    }
    if invocation["action"] != result["action"] {
        return Err(Error::Invalid(
            "Resolved invocation belongs to a different action".into(),
        ));
    }
    if images_profile
        && ["return_images", "image_blocks"]
            .iter()
            .any(|name| invocation["arguments"].get(name).is_some())
    {
        return Err(Error::Invalid(
            "Resolved invocation includes image-delivery arguments".into(),
        ));
    }
    Ok(())
}
