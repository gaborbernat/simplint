use std::borrow::Cow;

use schemars::generate::SchemaSettings;
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde_json::{Value, json};

use crate::lint::registry::Rule;
use crate::settings::SimplintOptions;

pub fn config_schema() -> Schema {
    // SchemaStore and taplo read draft 7.
    let mut schema = SchemaSettings::draft07().into_generator().into_root_schema_for::<SimplintOptions>();
    schema.insert(
        "$id".to_owned(),
        "https://raw.githubusercontent.com/gaborbernat/simplint/main/simplint.schema.json".into(),
    );
    schema.insert("title".to_owned(), "simplint configuration".into());
    schema.insert("description".to_owned(), "simplint configuration ([tool.simplint] in pyproject.toml)".into());
    schema.insert(
        "x-taplo".to_owned(),
        json!({ "links": { "key": "https://simplint.readthedocs.io/en/latest/configuration.html" } }),
    );
    schema.ensure_object().values_mut().for_each(portable);
    schema
}

// TOML represents a missing option by omitting the key, so strip schemars' nullable types and null defaults.
// The schema already expresses unsigned integer bounds with `minimum`. Drop the nonstandard format names.
fn portable(value: &mut Value) {
    match value {
        Value::Object(map) => {
            // A property may be named `format`, but its schema is an object rather than the string a format is.
            map.retain(|key, value| !(key == "format" && value.is_string() || key == "default" && value.is_null()));
            if let Some(Value::Array(types)) = map.get_mut("type") {
                types.retain(|kind| kind != "null");
            }
            if let Some(Value::Array(variants)) = map.get_mut("anyOf") {
                variants.retain(|variant| variant != &json!({ "type": "null" }));
            }
            map.values_mut().for_each(portable);
        }
        Value::Array(items) => items.iter_mut().for_each(portable),
        _ => {}
    }
}

// Offer selectors in the `SMP` namespace to editors. The CLI also accepts the shorter prefixes `S` and `SM`.
pub(super) struct RuleSelectorSchema;

impl JsonSchema for RuleSelectorSchema {
    fn schema_name() -> Cow<'static, str> {
        "RuleSelector".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        let mut selectors: Vec<&str> = vec!["ALL"];
        for rule in Rule::ALL {
            selectors.push(rule.name());
            let code = rule.code();
            selectors.extend((3..=code.len()).map(|length| &code[..length]));
        }
        selectors.sort_unstable();
        selectors.dedup();
        json_schema!({ "type": "string", "enum": selectors })
    }
}
