//! Argument-object parameter names, independently of nested parameter values.
use super::{Embedded, Error, Result, validator};
use jsonschema::{Draft, Registry, Validator};
use serde_json::{Value, json};
use std::collections::BTreeSet;

const BASE: &str = "https://wafflery.invalid/parameters";
const MAX_SCHEMAS: usize = 4096;

pub(super) struct ParameterNames {
    names: BTreeSet<String>,
    patterns: Vec<Validator>,
}
fn invalid(error: impl std::fmt::Display) -> Error {
    Error::Invalid(format!("Cannot inspect parameter schema: {error}"))
}
// Keep equivalent reference contexts finite when schemas refer back to themselves.
fn scope_key(scopes: Vec<(String, bool)>) -> (Vec<String>, Vec<String>) {
    fn last_occurrences(mut names: Vec<String>) -> Vec<String> {
        let mut seen = BTreeSet::new();
        names.reverse();
        names.retain(|uri| seen.insert(uri.clone()));
        names.reverse();
        names
    }
    // 2019-09 stops at the first resource without a recursive anchor. Keep that
    // leading segment separately; whole-scope deduplication can cross its barrier.
    let recursive = scopes
        .iter()
        .take_while(|(_, anchored)| *anchored)
        .map(|(uri, _)| uri.clone())
        .collect();
    // 2020-12 dynamic anchors use the last matching resource in the whole scope.
    let dynamic = scopes.into_iter().map(|(uri, _)| uri).collect();
    (last_occurrences(dynamic), last_occurrences(recursive))
}
impl ParameterNames {
    pub fn new(schema: &Value) -> Result<Self> {
        // Use the same validator and embedded-only retriever as callable schemas.
        validator(schema)?;
        let draft = Draft::default().detect(schema);
        let registry = Registry::new()
            .draft(draft)
            .retriever(Embedded)
            .add(BASE, schema)
            .map_err(invalid)?
            .prepare()
            .map_err(invalid)?;
        let resolver = registry.resolver(jsonschema::uri::from_str(BASE).map_err(invalid)?);
        let mut pending = vec![(schema, resolver, draft, true)];
        let mut visited = BTreeSet::new();
        let mut result = Self {
            names: BTreeSet::new(),
            patterns: Vec::new(),
        };
        while let Some((node, resolver, draft, enter)) = pending.pop() {
            let draft = draft.detect(node);
            let resolver = if enter {
                resolver
                    .in_subresource(draft.create_resource_ref(node))
                    .map_err(invalid)?
            } else {
                resolver
            };
            let scopes = resolver.dynamic_scope().iter().map(|uri| {
                let anchored = resolver.lookup(uri.as_str()).map_err(invalid)?
                    .contents()["$recursiveAnchor"] == true;
                Ok((uri.as_str().to_owned(), anchored))
            }).collect::<Result<Vec<_>>>()?;
            let scopes = scope_key(scopes);
            let key = (
                std::ptr::from_ref(node).addr(),
                resolver.base_uri().to_string(),
                scopes,
            );
            if !visited.insert(key) {
                continue;
            }
            if visited.len() > MAX_SCHEMAS {
                return Err(invalid("schema inspection limit exceeded"));
            }
            let Some(object) = node.as_object() else {
                continue;
            };
            if let Some(reference) = node["$ref"].as_str() {
                let (target, next, target_draft) =
                    resolver.lookup(reference).map_err(invalid)?.into_inner();
                pending.push((target, next, target_draft, false));
                // Before 2019-09, siblings of $ref are ignored by the validator.
                if matches!(draft, Draft::Draft4 | Draft::Draft6 | Draft::Draft7) {
                    continue;
                }
            }
            if draft == Draft::Draft201909 && node["$recursiveRef"].as_str().is_some() {
                let (target, next, target_draft) = resolver
                    .lookup_recursive_ref()
                    .map_err(invalid)?
                    .into_inner();
                pending.push((target, next, target_draft, false));
            }
            if draft == Draft::Draft202012
                && let Some(reference) = node["$dynamicRef"].as_str()
            {
                let (target, next, target_draft) =
                    resolver.lookup(reference).map_err(invalid)?.into_inner();
                pending.push((target, next, target_draft, false));
            }
            if let Some(properties) = node["properties"].as_object() {
                result.names.extend(properties.keys().cloned());
            }
            if let Some(required) = node["required"].as_array() {
                result
                    .names
                    .extend(required.iter().filter_map(Value::as_str).map(str::to_owned));
            }
            if let Some(patterns) = node["patternProperties"].as_object() {
                for pattern in patterns.keys() {
                    result
                        .patterns
                        .push(validator(&json!({"type":"string","pattern":pattern}))?);
                }
            }
            // These applicators evaluate the argument object itself, not a property's value.
            for keyword in ["allOf", "anyOf", "oneOf"] {
                for child in node[keyword].as_array().into_iter().flatten() {
                    pending.push((child, resolver.clone(), draft, true));
                }
            }
            if let Some(child) = object.get("not") {
                pending.push((child, resolver.clone(), draft, true));
            }
            if !matches!(draft, Draft::Draft4 | Draft::Draft6) && object.contains_key("if") {
                for keyword in ["if", "then", "else"] {
                    if let Some(child) = object.get(keyword) {
                        pending.push((child, resolver.clone(), draft, true));
                    }
                }
            }
            let modern = matches!(draft, Draft::Draft201909 | Draft::Draft202012);
            for keyword in ["dependencies", "dependentSchemas", "dependentRequired"] {
                if keyword != "dependencies" && !modern {
                    continue;
                }
                for (name, child) in node[keyword].as_object().into_iter().flatten() {
                    result.names.insert(name.clone());
                    if let Some(names) = child.as_array() {
                        result
                            .names
                            .extend(names.iter().filter_map(Value::as_str).map(str::to_owned));
                    } else {
                        pending.push((child, resolver.clone(), draft, true));
                    }
                }
            }
        }
        Ok(result)
    }
    pub fn explicit_names(&self) -> impl Iterator<Item = &str> {
        self.names.iter().map(String::as_str)
    }
    pub fn contains(&self, name: &str) -> bool {
        self.names.contains(name)
            || self
                .patterns
                .iter()
                .any(|pattern| pattern.is_valid(&json!(name)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn names_follow_compositions_and_dependencies_but_not_nested_values() {
        let names = ParameterNames::new(&json!({
            "type":"object", "properties":{"options":{"properties":{"nested_only":{}}}},
            "allOf":[{"properties":{"direct":{}}},{"anyOf":[{"required":["required_name"]},
              {"if":{"properties":{"condition":{}}},"then":{"properties":{"then_name":{}}},
               "else":{"properties":{"else_name":{}}}}]}],
            "dependentSchemas":{"trigger":{"properties":{"dependent":{}}}},
            "dependentRequired":{"dependency":["needed"]},
            "$defs":{"unused":{"properties":{"unused_name":{}}}}
        }))
        .unwrap();
        for name in [
            "options",
            "direct",
            "required_name",
            "condition",
            "then_name",
            "else_name",
            "trigger",
            "dependent",
            "dependency",
            "needed",
        ] {
            assert!(names.contains(name), "{name}");
        }
        for name in ["nested_only", "unused_name", "unknown"] {
            assert!(!names.contains(name), "{name}");
        }
    }

    #[test]
    fn references_resolve_local_pointers_anchors_and_relative_resource_ids() {
        let names = ParameterNames::new(&json!({
            "$id":"https://example.invalid/params/root.json",
            "$defs":{"pointed":{"properties":{"pointer_name":{}}},
              "scoped":{"$id":"nested/","$defs":{"names":{"$anchor":"names","properties":{"anchor_name":{}}}},"$ref":"#names"}},
            "allOf":[{"$ref":"#/$defs/pointed"},{"$ref":"nested/"}]
        })).unwrap();
        assert!(names.contains("pointer_name"));
        assert!(names.contains("anchor_name"));
        assert!(!names.contains("names"));
    }

    #[test]
    fn patterns_claim_only_matching_argument_names() {
        let names = ParameterNames::new(&json!({"patternProperties":{"^source(_[0-9]+)?$":{}},
            "properties":{"options":{"patternProperties":{".*":{}}}}}))
        .unwrap();
        for name in ["source", "source_12", "options"] {
            assert!(names.contains(name));
        }
        for name in ["source_extra", "prefix_source", "other"] {
            assert!(!names.contains(name));
        }
    }

    #[test]
    fn recursive_same_instance_references_terminate_without_losing_names() {
        let names = ParameterNames::new(&json!({"$defs":{"node":{"allOf":[
            {"properties":{"source":{}}},{"$ref":"#/$defs/node"}]}},"$ref":"#/$defs/node"}))
        .unwrap();
        assert!(names.contains("source"));
        assert!(!names.contains("missing"));
    }

    #[test]
    fn legacy_reference_siblings_and_orphan_conditionals_do_not_claim_names() {
        let names = ParameterNames::new(&json!({
            "$schema":"http://json-schema.org/draft-07/schema#",
            "definitions":{"used":{"dependencies":{"trigger":["needed"]}}},
            "$ref":"#/definitions/used", "properties":{"ignored_sibling":{}}
        }))
        .unwrap();
        assert!(names.contains("trigger"));
        assert!(names.contains("needed"));
        assert!(!names.contains("ignored_sibling"));
        let names = ParameterNames::new(&json!({
            "then":{"properties":{"ignored_then":{}}},
            "else":{"required":["ignored_else"]}
        }))
        .unwrap();
        assert!(!names.contains("ignored_then"));
        assert!(!names.contains("ignored_else"));
    }

    #[test]
    fn dynamic_and_recursive_resource_cycles_keep_argument_names() {
        for schema in [
            json!({
                "$schema":"https://json-schema.org/draft/2020-12/schema",
                "$id":"https://example.invalid/outer", "$dynamicAnchor":"node",
                "properties":{"outer_name":{}}, "$ref":"inner",
                "$defs":{"inner":{"$id":"inner", "$dynamicAnchor":"node",
                    "properties":{"inner_name":{}}, "$dynamicRef":"#node"}}
            }),
            json!({
                "$schema":"https://json-schema.org/draft/2019-09/schema",
                "$id":"https://example.invalid/outer", "$recursiveAnchor":true,
                "properties":{"outer_name":{}}, "$ref":"inner",
                "$defs":{"inner":{"$id":"inner", "$recursiveAnchor":true,
                    "properties":{"inner_name":{}}, "$recursiveRef":"#"}}
            }),
        ] {
            let names = ParameterNames::new(&schema).unwrap();
            assert!(names.contains("outer_name"));
            assert!(names.contains("inner_name"));
            assert!(!names.contains("unknown"));
        }
    }

    #[test]
    fn excessive_inspection_is_refused_instead_of_accepting_partial_names() {
        let schema = json!({"allOf": vec![json!({}); MAX_SCHEMAS]});
        let error = ParameterNames::new(&schema).err().unwrap().to_string();
        assert!(error.contains("inspection limit"), "{error}");
    }

    #[test]
    fn recursive_barriers_distinguish_otherwise_equal_dynamic_scopes() {
        // Dynamic anchors use the last matching resource, but recursive anchors stop at B.
        let a = ("https://example.invalid/a".to_owned(), true);
        let b = ("https://example.invalid/b".to_owned(), false);
        let blocked = scope_key(vec![b.clone(), a.clone()]);
        let anchored = scope_key(vec![a.clone(), b.clone(), a.clone()]);
        assert_eq!(blocked.0, anchored.0, "same 2020-12 dynamic resolution");
        assert_ne!(
            blocked.1, anchored.1,
            "different 2019-09 recursive resolution"
        );
        assert_eq!(
            scope_key(vec![a.clone(), a.clone(), b.clone(), a.clone()]),
            anchored
        );
        assert_eq!(
            scope_key(vec![a.clone(), b.clone(), a.clone(), b, a]),
            anchored
        );
    }

    #[test]
    fn embedded_references_work_and_unknown_external_references_are_refused() {
        let names =
            ParameterNames::new(&json!({"$ref":"urn:table-manners:core:result#/$defs/tool"}))
                .unwrap();
        assert!(names.contains("name"));
        assert!(names.contains("version"));
        assert!(
            ParameterNames::new(&json!({"$ref":"https://example.invalid/unavailable.json"}))
                .is_err()
        );
    }
}
