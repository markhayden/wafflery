//! Validation boundary for host-owned quotes. Estimates never create attempts.
use super::{Error, Result, session::validate_observation, validate_contract, validator};
use serde_json::{Value, json};
use time::{Duration, OffsetDateTime, format_description::well_known::Rfc3339};

mod control;

const USD_TOLERANCE: f64 = 1e-6;

/// An attributed observation, not a reservation or permission to dispatch by itself.
/// The coordinator must retain it with the exact prepared call and recheck its lifetime.
pub struct ValidatedEstimate {
    value: Value,
    expires: OffsetDateTime,
    deadline: tokio::time::Instant,
}
impl ValidatedEstimate {
    pub fn validate(
        value: Value,
        tool: &Value,
        action: &str,
        cap: f64,
        session: &Value,
        received: OffsetDateTime,
    ) -> Result<Self> {
        let received_monotonic = tokio::time::Instant::now();
        validate_contract("urn:table-manners:core:estimate", &value)?;
        if !cap.is_finite() || cap <= 0.0 {
            return Err(Error::Invalid(
                "An estimate needs a positive finite user cap".into(),
            ));
        }
        if value["tool"]["name"] != tool["name"]
            || value["tool"]["version"] != tool["version"]
            || value["action"] != action
        {
            return Err(Error::Invalid(
                "Estimate belongs to a different tool or action".into(),
            ));
        }
        // A CLI quote has no serving session and cannot authorize an MCP call.
        if value["session_id"].as_str().is_none()
            || value["session_id"] != session["id"]
            || value["session_id"] != value["session"]["id"]
        {
            return Err(Error::Invalid(
                "Estimate belongs to a different serving session".into(),
            ));
        }
        validate_observation(&value["session"], tool, Some(session))?;
        let expires = OffsetDateTime::parse(value["expires"].as_str().unwrap(), &Rfc3339)
            .map_err(|_| Error::Invalid("Invalid estimate expiry".into()))?;
        if expires <= received || expires - received > Duration::hours(24) {
            return Err(Error::Invalid(
                "Estimate is expired or lasts longer than 24 hours".into(),
            ));
        }
        let ready = value["readiness"]["state"] == "ready";
        let refused = value["refused"].as_str();
        if (refused == Some("readiness")) == ready {
            return Err(Error::Invalid(
                "Estimate refusal contradicts readiness".into(),
            ));
        }
        // Missing setup may make the effective cap, configuration and price unknowable.
        // Preserve that valid refusal, without granting any authorization or inventing zero.
        if refused != Some("readiness") {
            let effective_cap = value["cap_usd"]
                .as_f64()
                .filter(|cap| cap.is_finite() && *cap > 0.0)
                .ok_or_else(|| {
                    Error::Invalid("Estimate has no positive finite effective cap".into())
                })?;
            if (effective_cap - cap).abs() > USD_TOLERANCE {
                return Err(Error::Invalid(
                    "Estimate changed the requested spending cap".into(),
                ));
            }
            let worst = value["worst_case_usd"]
                .as_f64()
                .filter(|cost| cost.is_finite())
                .ok_or_else(|| Error::Invalid("Estimate has no finite worst-case cost".into()))?;
            let remaining = value["session"]["remaining_usd"].as_f64();
            let consistent = match refused {
                None => {
                    worst <= effective_cap + USD_TOLERANCE
                        && remaining.is_none_or(|left| worst <= left + USD_TOLERANCE)
                }
                // Do not reject a conservative refusal over sub-tolerance rounding.
                Some("cap") => worst + USD_TOLERANCE >= effective_cap,
                Some("budget") => remaining.is_some_and(|left| worst + USD_TOLERANCE >= left),
                _ => false, // The schema restricts the reasons; keep this boundary closed.
            };
            if !consistent {
                return Err(Error::Invalid(
                    "Estimate decision contradicts its spending limits".into(),
                ));
            }
        }
        let lifetime = std::time::Duration::try_from(expires - received)
            .map_err(|_| Error::Invalid("Invalid estimate lifetime".into()))?;
        Ok(Self {
            value,
            expires,
            deadline: received_monotonic + lifetime,
        })
    }
    /// MCP estimates require structuredContent; a text-only response or run is not a quote.
    pub fn from_response(
        response: &Value,
        tool: &Value,
        action: &str,
        cap: f64,
        session: &Value,
        received: OffsetDateTime,
    ) -> Result<Self> {
        if response["isError"] == true {
            return Err(Error::Invalid(
                "Estimate returned an error result instead of a quote".into(),
            ));
        }
        let value = response
            .get("structuredContent")
            .ok_or_else(|| Error::Invalid("Estimate is missing structuredContent".into()))?;
        Self::validate(value.clone(), tool, action, cap, session, received)
    }
    /// Server-side evidence, including the opaque binding. Never serialize this directly
    /// into a browser response; the presentation layer selects display fields separately.
    pub(crate) fn expired(&self) -> bool {
        time::OffsetDateTime::now_utc() >= self.expires
            || tokio::time::Instant::now() >= self.deadline
    }
    pub fn observation(&self) -> &Value {
        &self.value
    }
}

/// The only named estimate failure is invalid_arguments. Missing setup is a quote refusal,
/// not not_ready, and an estimate cannot claim or reuse a client run ID.
pub fn validate_refusal(error: &rmcp::model::ErrorData) -> Result<()> {
    if error.code != rmcp::model::ErrorCode::INVALID_PARAMS {
        return Err(Error::Invalid("Unexpected estimate RPC error code".into()));
    }
    if let Some(data) = error.data.as_ref().filter(|data| !data.is_null()) {
        validator(&json!({"$ref":"urn:table-manners:core:refusal"}))?
            .validate(data)
            .map_err(|_| Error::Invalid("Malformed estimate refusal".into()))?;
        if data["reason"] != "invalid_arguments" {
            return Err(Error::Invalid("Unexpected estimate refusal reason".into()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
