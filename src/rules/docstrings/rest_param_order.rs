use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstrings::rest::FieldKind;

pub fn rest_param_order(checker: &Checker) {
    for target in checker.rest_targets() {
        let Some(parameters) = target.parameters() else {
            continue;
        };
        let position = |name: &str| parameters.iter().position(|parameter| parameter.name == name);
        let documented: Vec<(usize, &str, &_)> = target
            .docstring
            .fields(&[FieldKind::Param])
            .filter_map(|field| {
                let subject = field.subject()?;
                Some((position(subject)?, subject, field))
            })
            .collect();
        let Some(pair) = documented.windows(2).find(|pair| pair[1].0 < pair[0].0) else {
            continue;
        };
        let ((_, earlier, _), (_, later, field)) = (pair[0], pair[1]);
        checker.report(
            Rule::RestParamOrder,
            field.marker.unwrap_or_else(|| target.docstring.range()),
            format!(
                "`:param {later}:` follows `:param {earlier}:`; document parameters in the order `{}` takes them",
                target.name()
            ),
        );
    }
}
