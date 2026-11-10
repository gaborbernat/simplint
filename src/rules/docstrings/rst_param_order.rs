use rustc_hash::FxHashMap;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstrings::rst::FieldKind;

pub fn rst_param_order(checker: &Checker) {
    for target in checker.state().rst_targets(checker) {
        let Some(parameters) = target.parameters() else {
            continue;
        };
        let mut positions = FxHashMap::default();
        for (index, parameter) in parameters.iter().enumerate() {
            positions.entry(parameter.name).or_insert(index);
        }
        let documented: Vec<(usize, &str, &_)> = target
            .docstring
            .fields(&[FieldKind::Param])
            .filter_map(|field| {
                let subject = field.subject()?;
                Some((*positions.get(subject)?, subject, field))
            })
            .collect();
        let Some(pair) = documented.windows(2).find(|pair| pair[1].0 < pair[0].0) else {
            continue;
        };
        let ((_, earlier, _), (_, later, field)) = (pair[0], pair[1]);
        checker.report(
            Rule::RstParamOrder,
            field.range_in(&target.docstring),
            format!(
                "`:param {later}:` follows `:param {earlier}:`; document parameters in the order `{}` takes them",
                target.name()
            ),
        );
    }
}
