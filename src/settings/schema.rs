use std::borrow::Cow;

use schemars::generate::SchemaSettings;
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde_json::{Value, json};

use crate::lint::registry::Rule;
use crate::settings::SimplintOptions;

// Where SchemaStore and editors find the schema, once the file is published on the default branch.
const ID: &str = "https://raw.githubusercontent.com/gaborbernat/simplint/main/simplint.schema.json";

// The schema of `[tool.simplint]`, which `simplint config-schema` prints and SchemaStore checks the table against.
// Draft 7 is what SchemaStore and taplo read.
pub fn config_schema() -> Schema {
    let mut schema = SchemaSettings::draft07().into_generator().into_root_schema_for::<SimplintOptions>();
    schema.insert("$id".to_owned(), ID.into());
    schema.insert("title".to_owned(), "simplint configuration".into());
    schema.insert("description".to_owned(), "simplint configuration ([tool.simplint] in pyproject.toml)".into());
    schema.insert(
        "x-taplo".to_owned(),
        json!({ "links": { "key": "https://simplint.readthedocs.io/en/latest/configuration.html" } }),
    );
    schema.ensure_object().values_mut().for_each(portable);
    schema
}

// TOML has no null, so an option's absent key is its default, which the descriptions state; schemars makes every
// option nullable with a `null` default. Its integer formats, such as `uint`, are not JSON Schema formats, which
// SchemaStore's validator rejects, and `minimum` already states the range.
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

// A selector as `RuleSet::from_selector` reads it: `ALL`, a rule's code or name, or a code prefix from the `SMP`
// namespace down. Ruff's selectors are left out, as simplint rejects them.
pub struct RuleSelectorSchema;

impl JsonSchema for RuleSelectorSchema {
    fn schema_name() -> Cow<'static, str> {
        "RuleSelector".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        let mut selectors: Vec<&str> = vec!["ALL"];
        for rule in Rule::ALL {
            selectors.push(rule.name());
            selectors.extend((3..=rule.code().len()).map(|length| &rule.code()[..length]));
        }
        selectors.sort_unstable();
        selectors.dedup();
        json_schema!({ "type": "string", "enum": selectors })
    }
}
