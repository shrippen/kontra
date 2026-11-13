//! JSON Schemas of the files people write by hand or with other tools (W8): the manifests of
//! `.kontra` and course packages and the course file. Generated from the serde structures, so
//! they say what Kontra reads; `kontra-cli schemas <dir>` writes them, `schema/` holds them.

use crate::exercise::PiecePlan;
use crate::lesson::{course_schema, CourseManifest};
use crate::Manifest;

const PACKAGE_MANIFEST: &str = "package-manifest.schema.json";
const COURSE_MANIFEST: &str = "course-manifest.schema.json";
const COURSE: &str = "course.schema.json";
/// For the prompt of an LLM that writes piece plans (LH-BIB-13).
const PIECE_PLAN: &str = "piece-plan.schema.json";

/// Each schema as (file name, pretty JSON with a final newline).
pub fn json_schemas() -> Result<Vec<(&'static str, String)>, serde_json::Error> {
    let schemas = [
        (PACKAGE_MANIFEST, schemars::schema_for!(Manifest)),
        (COURSE_MANIFEST, schemars::schema_for!(CourseManifest)),
        (COURSE, course_schema()),
        (PIECE_PLAN, schemars::schema_for!(PiecePlan)),
    ];
    schemas
        .into_iter()
        .map(|(name, schema)| Ok((name, serde_json::to_string_pretty(&schema)? + "\n")))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn schema(name: &str) -> Value {
        let schemas = json_schemas().expect("schemas");
        let (_, text) = schemas.iter().find(|(n, _)| *n == name).expect(name);
        serde_json::from_str(text).expect("json")
    }

    #[test]
    fn follows_serde_names_and_defaults() {
        // `for` is the field's name in the file, `scope` only in Rust.
        let course = schema(COURSE);
        assert!(course["properties"]["for"].is_object(), "{course}");
        assert_eq!(course["additionalProperties"], Value::Bool(false));

        // Fields with a default are optional.
        let package = schema(PACKAGE_MANIFEST);
        let required = package["required"].as_array().expect("required");
        assert!(required.contains(&Value::from("score")));
        assert!(!required.contains(&Value::from("loops")));
    }
}
