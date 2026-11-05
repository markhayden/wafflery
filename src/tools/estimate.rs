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
        cap: Option<f64>,
        session: &Value,
        received: OffsetDateTime,
    ) -> Result<Self> {
        let received_monotonic = tokio::time::Instant::now();
        validate_contract("urn:table-manners:core:estimate", &value)?;
        // A spending limit is the person's option (table-manners 4.5: a call MAY lower the
        // tool's cap); without one the tool's own limits decide.
        if cap.is_some_and(|cap| !cap.is_finite() || cap <= 0.0) {
            return Err(Error::Invalid(
                "A spending limit must be a positive finite amount".into(),
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
            // The tool's cap in force: the asked limit when one was given, else its own (none
            // when it configures none).
            let effective_cap = match &value["cap_usd"] {
                Value::Null => None,
                quoted => Some(
                    quoted
                        .as_f64()
                        .filter(|cap| cap.is_finite() && *cap > 0.0)
                        .ok_or_else(|| {
                            Error::Invalid("Estimate has no positive finite effective cap".into())
                        })?,
                ),
            };
            if let Some(asked) = cap
                && effective_cap.is_none_or(|effective| (effective - asked).abs() > USD_TOLERANCE)
            {
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
                    effective_cap.is_none_or(|cap| worst <= cap + USD_TOLERANCE)
                        && remaining.is_none_or(|left| worst <= left + USD_TOLERANCE)
                }
                // Do not reject a conservative refusal over sub-tolerance rounding.
                Some("cap") => effective_cap.is_some_and(|cap| worst + USD_TOLERANCE >= cap),
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
        cap: Option<f64>,
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
    pub(crate) fn authorize(
        &self,
        tool: &Value,
        action: &str,
        arguments: &Value,
        session: &Value,
    ) -> Result<()> {
        let cap = arguments["max_spend_usd"]
            .as_f64()
            .filter(|cap| cap.is_finite() && *cap > 0.0);
        if self.expired() || self.value["allowed"] != true {
            return Err(Error::Conflict(
                "A current allowed estimate is required before running",
            ));
        }
        if self.value["tool"]["name"] != tool["name"]
            || self.value["tool"]["version"] != tool["version"]
            || self.value["action"] != action
            || self.value["session_id"] != session["id"]
            || cap.is_some_and(|requested| {
                self.value["cap_usd"]
                    .as_f64()
                    .is_none_or(|quoted| (requested - quoted).abs() > USD_TOLERANCE)
            })
        {
            return Err(Error::Conflict(
                "Estimate belongs to different inputs, cap or session",
            ));
        }
        Ok(())
    }
    pub(crate) fn evidence(&self) -> Value {
        // Preserve authorization facts, not arbitrary provider/debug extensions.
        json!({"tool":self.value["tool"],"action":self.value["action"],"binding":self.value["binding"],
            "session_id":self.value["session_id"],"cap_usd":self.value["cap_usd"],
            "worst_case_usd":self.value["worst_case_usd"],"allowed":self.value["allowed"],
            "effective":{"model":self.value["effective"]["model"],"config_identity":self.value["effective"]["config_identity"]},
            "expires":self.value["expires"]})
    }
    pub fn observation(&self) -> &Value {
        &self.value
    }
}

/// Estimates may refuse invalid arguments or artifact bindings before any run exists.
/// Missing setup is a quote refusal, not not_ready; estimates never claim a client run ID.
pub fn validate_refusal(
    error: &rmcp::model::ErrorData,
    action: &Value,
    arguments: &Value,
) -> Result<()> {
    if error.code != rmcp::model::ErrorCode::INVALID_PARAMS {
        return Err(Error::Invalid("Unexpected estimate RPC error code".into()));
    }
    if let Some(data) = error.data.as_ref().filter(|data| !data.is_null()) {
        validator(&json!({"$ref":"urn:table-manners:core:refusal"}))?
            .validate(data)
            .map_err(|_| Error::Invalid("Malformed estimate refusal".into()))?;
        match data["reason"].as_str() {
            Some("invalid_arguments") => {}
            Some("binding") => {
                use super::binding_refusal_semantics::{Context, validate};
                validate(data, Context::Operation { action, arguments })?;
            }
            _ => return Err(Error::Invalid("Unexpected estimate refusal reason".into())),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
