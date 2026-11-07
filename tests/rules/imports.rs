use std::fs;
use std::path::Path;

use tempfile::TempDir;
use test_case::test_case;

use crate::common;

#[test_case("future-annotations", "future_annotations.py", "py314" ; "future_annotations")]
#[test_case("future-annotations", "future_annotations.py", "py310" ; "future_annotations_py310")]
#[test_case("future-annotations", "future_annotations_checking.py", "py314" ; "future_annotations_type_checking")]
#[test_case("future-annotations", "future_annotations_joined.py", "py314" ; "future_annotations_joined_statement")]
#[test_case("future-annotations", "future_annotations_class_bindings.py", "py314" ; "future_annotations_class_bindings")]
#[test_case("future-annotations", "future_annotations_class_shadow.py", "py314" ; "future_annotations_class_shadow")]
#[test_case("import-cycle", "import_cycle.py", "py314" ; "import_cycle")]
#[test_case("import-cycle", "import_cycle_partner.py", "py314" ; "import_cycle_partner")]
#[test_case("import-cycle", "import_cycle_ring_a.py", "py314" ; "import_cycle_ring")]
#[test_case("import-cycle", "import_cycle_exceptions.py", "py314" ; "import_cycle_exceptions")]
#[test_case("import-cycle", "import_cycle_lazy.py", "py315" ; "import_cycle_lazy")]
#[test_case("import-cycle", "import_cycle_pkg/core.py", "py314" ; "import_cycle_package")]
#[test_case("import-cycle", "import_cycle_dead.py", "py314" ; "import_cycle_dead")]
#[test_case("import-cycle", "import_cycle_platform.py", "py314" ; "import_cycle_platform")]
#[test_case("import-cycle", "import_cycle_platform_partner.py", "py314" ; "import_cycle_platform_partner")]
#[test_case("import-cycle", "test_import_cycle_fixture.py", "py314" ; "import_cycle_test_module")]
#[test_case("star-import", "star_import_listed.py", "py314" ; "star_import_listed")]
#[test_case("star-import", "star_import_unlisted.py", "py314" ; "star_import_unlisted")]
#[test_case("star-import", "star_import_unlisted_import.py", "py314" ; "star_import_unlisted_import")]
#[test_case("star-import", "star_import_own_all.py", "py314" ; "star_import_own_all")]
#[test_case("star-import", "star_import_multiple.py", "py314" ; "star_import_multiple")]
#[test_case("star-import", "star_import_dynamic.py", "py314" ; "star_import_dynamic")]
#[test_case("star-import", "star_import_unread.py", "py314" ; "star_import_unread")]
#[test_case("star-import", "star_import_external.py", "py314" ; "star_import_external")]
#[test_case("star-import", "star_pkg/consumer.py", "py314" ; "star_import_package")]
#[test_case("star-import", "star_import_reexport.py", "py314" ; "star_import_reexport")]
#[test_case("star-import", "star_import_reexporter.py", "py314" ; "star_import_reexporter")]
#[test_case("star-import", "star_import_builtin.py", "py314" ; "star_import_builtin")]
#[test_case("star-import", "star_import_undefined.py", "py314" ; "star_import_undefined")]
#[test_case("star-import", "star_import_conditional.py", "py314" ; "star_import_conditional")]
#[test_case("star-import", "star_import_dynamic_all.py", "py314" ; "star_import_dynamic_all")]
#[test_case("star-import", "star_import_fallback.py", "py314" ; "star_import_fallback")]
#[test_case("star-import", "star_import_comment.py", "py314" ; "star_import_comment")]
#[test_case("star-import", "star_reexport_pkg/__init__.py", "py314" ; "star_import_init")]
#[test_case("star-import", "star_import_hidden.py", "py314" ; "star_import_hidden")]
#[test_case("star-import", "star_import_partial.py", "py314" ; "star_import_partial")]
#[test_case("absolute-sibling-import", "package/sub/module.py", "py314" ; "absolute_sibling_import")]
#[test_case("absolute-sibling-import", "package/__init__.py", "py314" ; "absolute_sibling_import_init")]
#[test_case("absolute-sibling-import", "package/sub/script.py", "py314" ; "absolute_sibling_import_script")]
#[test_case("absolute-sibling-import", "package/sub/channel_exec.py", "py314" ; "absolute_sibling_import_channel_exec")]
#[test_case("absolute-sibling-import", "package/sub/remote_worker.py", "py314" ; "absolute_sibling_import_remote_worker")]
#[test_case("absolute-sibling-import", "package/sub/reversed_guard.py", "py314" ; "absolute_sibling_import_reversed_guard")]
#[test_case(
    "absolute-sibling-import", "multiline_module_docstring_ok.py", "py314" ; "absolute_sibling_import_outside_package"
)]
#[test_case("module-attribute-import", "writers/module_fix.py", "py314" ; "module_attribute_import_fix")]
#[test_case("module-attribute-import", "writers/module_alias.py", "py314" ; "module_attribute_import_alias")]
#[test_case("module-attribute-import", "writers/module_mixed.py", "py314" ; "module_attribute_import_store_patches_module")]
#[test_case("module-attribute-import", "writers/module_undefined.py", "py314" ; "module_attribute_import_undefined")]
#[test_case("module-attribute-import", "writers/module_value.py", "py314" ; "module_attribute_import_value")]
#[test_case("module-attribute-import", "writers/module_type_param.py", "py314" ; "module_attribute_import_type_param")]
#[test_case("module-attribute-import", "writers/module_parameter.py", "py314" ; "module_attribute_import_parameter")]
#[test_case("module-attribute-import", "writers/module_clash.py", "py314" ; "module_attribute_import_clash")]
#[test_case("module-attribute-import", "writers/module_rebound.py", "py314" ; "module_attribute_import_rebound")]
#[test_case("module-attribute-import", "writers/module_dunder.py", "py314" ; "module_attribute_import_dunder")]
#[test_case("module-attribute-import", "writers/module_third_party.py", "py314" ; "module_attribute_import_third_party")]
#[test_case("module-attribute-import", "writers/module_patched.py", "py314" ; "module_attribute_import_patched")]
#[test_case("module-attribute-import", "writers/module_lazy.py", "py315" ; "module_attribute_import_lazy")]
#[test_case("module-attribute-import", "writers/module_computed.py", "py314" ; "module_attribute_import_computed_write")]
#[test_case("dependency-import-guard", "dependency_import_guard.py", "py314" ; "dependency_import_guard")]
#[test_case("dependency-import-guard", "dependency_import_guard.py", "py310" ; "dependency_import_guard_py310")]
#[test_case("up-backport-import", "up_backport_import.py", "py311" ; "backport_import")]
#[test_case("up-backport-import", "up_backport_import.py", "py310" ; "backport_import_py310")]
#[test_case("up-backport-import", "up_backport_import_single.py", "py311" ; "backport_import_single")]
#[test_case("up-backport-import", "up_backport_import_inline.py", "py311" ; "backport_import_inline")]
#[test_case("up-type-checking-lazy-import", "up_type_checking_lazy_import.py", "py315" ; "type_checking_lazy_import")]
#[test_case("up-type-checking-lazy-import", "up_type_checking_lazy_import.py", "py314" ; "type_checking_lazy_import_py314")]
#[test_case("up-type-checking-lazy-import", "up_type_checking_lazy_import_shared.py", "py315" ; "type_checking_lazy_import_shared")]
#[test_case("up-type-checking-lazy-import", "up_type_checking_lazy_import_attribute.py", "py315" ; "type_checking_lazy_import_attribute")]
#[test_case("up-type-checking-lazy-import", "up_type_checking_lazy_import_comment.py", "py315" ; "type_checking_lazy_import_comment")]
#[test_case("up-type-checking-lazy-import", "up_type_checking_lazy_import_typeshed.py", "py315" ; "type_checking_lazy_import_typeshed")]
#[test_case("up-type-checking-lazy-import", "up_type_checking_lazy_import_inline.py", "py315" ; "type_checking_lazy_import_inline")]
#[test_case("up-type-checking-lazy-import", "up_type_checking_lazy_import_ignored.py", "py315" ; "type_checking_lazy_import_ignored")]
#[test_case("up-type-checking-lazy-import", "up_type_checking_lazy_import_module.py", "py315" ; "type_checking_lazy_import_module")]
#[test_case("up-type-checking-lazy-import", "up_type_checking_lazy_import_typeshed_module.py", "py315" ; "type_checking_lazy_import_typeshed_module")]
#[test_case("up-lazy-modules-list", "up_lazy_modules_list.py", "py315" ; "lazy_modules_list")]
#[test_case("up-lazy-modules-list", "up_lazy_modules_list.py", "py314" ; "lazy_modules_list_py314")]
#[test_case("up-lazy-modules-list", "up_lazy_modules_list_partial.py", "py315" ; "lazy_modules_list_partial")]
#[test_case("up-lazy-modules-list", "up_lazy_modules_list_mixed.py", "py315" ; "lazy_modules_list_mixed")]
#[test_case("up-lazy-modules-list", "up_lazy_modules_list_nonliteral.py", "py315" ; "lazy_modules_list_nonliteral")]
#[test_case("up-lazy-modules-list", "up_lazy_modules_list_comprehension.py", "py315" ; "lazy_modules_list_comprehension")]
#[test_case("up-lazy-modules-list", "up_lazy_modules_list_guarded.py", "py315" ; "lazy_modules_list_guarded")]
#[test_case("up-lazy-modules-list", "up_lazy_modules_list_inline.py", "py315" ; "lazy_modules_list_inline")]
#[test_case("up-lazy-modules-list", "up_lazy_modules_list_tuple.py", "py315" ; "lazy_modules_list_tuple")]
#[test_case("up-lazy-modules-list", "up_lazy_modules_list_set.py", "py315" ; "lazy_modules_list_set")]
fn rule(rule: &str, fixture: &str, target_version: &str) {
    common::assert_rule_snapshot(rule, fixture, target_version);
}

const PROJECT: &str = "[project]\nname = \"x\"\nversion = \"0\"\n";
const HOOK: &str = "repos:\n  - repo: https://github.com/henryiii/flake8-lazy\n    hooks:\n      - id: flake8-lazy\n";

// The package whose module imports its sibling absolutely, under `prefix`.
fn sibling_package(prefix: &str) -> Vec<(String, &'static str)> {
    [
        ("package/__init__.py", ""),
        ("package/sub/__init__.py", ""),
        ("package/sub/sibling.py", "VALUE = 1\n"),
        ("package/sub/module.py", "from package.sub import sibling\n"),
    ]
    .map(|(name, content)| (format!("{prefix}{name}"), content))
    .into()
}

fn project_of(files: &[(String, &str)]) -> TempDir {
    let borrowed: Vec<(&str, &str)> = files.iter().map(|(name, content)| (name.as_str(), *content)).collect();
    common::project(&borrowed)
}

fn reported(root: &Path) -> bool {
    common::check(root, &["--select", "absolute-sibling-import"]).1.contains("SMP302")
}

// flake8-lazy keeps each module's `__lazy_modules__` in step with its absolute imports, so a project that runs it keeps
// them absolute.
#[test_case("[tool.flake8-lazy]\n", None, false ; "pyproject_table")]
#[test_case("", Some((".pre-commit-config.yaml", HOOK)), false ; "pre_commit_hook")]
#[test_case("", Some((".pre-commit-config.yml", HOOK)), false ; "pre_commit_hook_yml")]
#[test_case("", Some(("prek.toml", "[[repos]]\nrepo = \"x\"\nhooks = [{ id = \"flake8-lazy\" }]\n")), false ; "prek_hook")]
#[test_case(
    "",
    Some((".pre-commit-config.yaml", "repos:\n  - repo: x\n    hooks:\n      - id: 'flake8-lazy'  # lazy imports\n")),
    false
    ; "pre_commit_hook_trailing_comment"
)]
#[test_case(
    "",
    Some((".pre-commit-config.yaml", "repos:\n  - repo: x\n    hooks:\n      - id: flake8-lazy\t# lazy imports\n")),
    false
    ; "pre_commit_hook_tab_comment"
)]
#[test_case(
    "",
    Some((".pre-commit-config.yaml", "repos:\n  - repo: x\n    hooks:\n      - id: flake8\n        additional_dependencies: [flake8-lazy]\n")),
    true
    ; "additional_dependency_reports"
)]
#[test_case(
    "",
    Some((".pre-commit-config.yaml", "repos:\n  - repo: x\n    hooks:\n      # - id: flake8-lazy\n      - id: other\n")),
    true
    ; "commented_hook_reports"
)]
#[test_case("", None, true ; "unconfigured_reports")]
fn absolute_sibling_import_follows_flake8_lazy(pyproject: &str, hook: Option<(&str, &str)>, reported_: bool) {
    let mut files = sibling_package("");
    files.push(("pyproject.toml".to_owned(), pyproject));
    files.extend(hook.map(|(name, text)| (name.to_owned(), text)));
    assert_eq!(reported(project_of(&files).path()), reported_);
}

// The hook of the repository that holds the project counts, as pre-commit reads it from the repository root.
#[test]
fn absolute_sibling_import_finds_hook_above_the_project() {
    let mut files = sibling_package("monorepo/pkg/");
    files.push(("monorepo/pkg/pyproject.toml".to_owned(), PROJECT));
    files.push(("monorepo/.pre-commit-config.yaml".to_owned(), HOOK));
    files.push(("monorepo/.git/HEAD".to_owned(), "ref: refs/heads/main\n"));
    assert!(!reported(project_of(&files).path().join("monorepo").as_path()));
}

// The search stops at the repository root, so a hook above it does not count.
#[test]
fn absolute_sibling_import_stops_at_the_repository_root() {
    let mut files = sibling_package("outer/repo/");
    files.push(("outer/.pre-commit-config.yaml".to_owned(), HOOK));
    files.push(("outer/repo/pyproject.toml".to_owned(), PROJECT));
    files.push(("outer/repo/.git/HEAD".to_owned(), "ref: refs/heads/main\n"));
    assert!(reported(project_of(&files).path().join("outer/repo").as_path()));
}

// A `[tool.flake8-lazy]` table in the settings pyproject, which a workspace member's own `pyproject.toml` does not hold.
#[test]
fn absolute_sibling_import_finds_table_in_settings_pyproject() {
    let mut files = sibling_package("workspace/member/");
    files.push(("workspace/member/pyproject.toml".to_owned(), PROJECT));
    files.push((
        "workspace/pyproject.toml".to_owned(),
        "[tool.simplint]\n[tool.flake8-lazy.standalone]\napply = \"list\"\n",
    ));
    assert!(!reported(project_of(&files).path().join("workspace").as_path()));
}

// A nested project with its own `[project]` sees the hook of the project around it.
#[test]
fn absolute_sibling_import_nested_project_sees_outer_hook() {
    let mut files = sibling_package("");
    files.push(("pyproject.toml".to_owned(), PROJECT));
    files.push((".pre-commit-config.yaml".to_owned(), HOOK));
    files.extend(sibling_package("tests/fixture/"));
    files.push(("tests/fixture/pyproject.toml".to_owned(), PROJECT));
    assert!(!reported(project_of(&files).path()));
}

// A nested project without a `pyproject.toml` of its own takes the metadata of the project around it.
#[test]
fn absolute_sibling_import_nested_project_without_pyproject_sees_outer_hook() {
    let mut files = sibling_package("");
    files.push(("pyproject.toml".to_owned(), PROJECT));
    files.push((".pre-commit-config.yaml".to_owned(), HOOK));
    files.extend(sibling_package("tests/proj/"));
    files.push(("tests/proj/conf.py".to_owned(), ""));
    assert!(!reported(project_of(&files).path()));
}

// Without a `pyproject.toml`, the hook configuration of the directory the run starts from counts.
#[test]
fn absolute_sibling_import_without_pyproject_reads_hook_at_run_root() {
    let mut files = sibling_package("");
    files.push((".pre-commit-config.yaml".to_owned(), HOOK));
    assert!(!reported(project_of(&files).path()));
}

// `--isolated` reads no pyproject and no hook configuration, so the hook of the project does not count.
#[test]
fn isolated_run_does_not_read_hook_configurations() {
    let mut files = sibling_package("");
    files.push(("pyproject.toml".to_owned(), PROJECT));
    files.push(("prek.toml".to_owned(), "[[repos]]\nrepo = \"x\"\nhooks = [{ id = \"flake8-lazy\" }]\n"));
    let root = project_of(&files);
    let isolated = common::check(root.path(), &["--isolated", "--select", "absolute-sibling-import"]).1;
    assert_eq!((isolated.contains("SMP302"), reported(root.path())), (true, false));
}

// A hook configuration that cannot be parsed only stops the runs whose rules read it.
#[test_case("SMP101", 0 ; "rule_off_does_not_read_it")]
#[test_case("absolute-sibling-import", 2 ; "rule_on_reads_it")]
fn unparsable_hook_configuration_stops_only_runs_that_read_it(select: &str, code: i32) {
    let mut files = sibling_package("");
    files.push(("prek.toml".to_owned(), "[[repos]\n"));
    let root = project_of(&files);
    let (status, _, stderr) = common::check(root.path(), &["--select", select]);
    assert_eq!((status, stderr.contains("Failed to parse")), (code, code == 2));
}

// A hook configuration that cannot be read only stops the runs whose rules read it.
#[test_case("SMP101", 0 ; "rule_off_does_not_read_it")]
#[test_case("absolute-sibling-import", 2 ; "rule_on_reads_it")]
fn unreadable_hook_configuration_stops_only_runs_that_read_it(select: &str, code: i32) {
    let mut files = sibling_package("");
    files.push((".pre-commit-config.yaml".to_owned(), HOOK));
    let root = project_of(&files);
    common::set_mode(&root.path().join(".pre-commit-config.yaml"), 0o000);
    let (status, _, stderr) = common::check(root.path(), &["--select", select]);
    assert_eq!((status, stderr.contains("Failed to read")), (code, code == 2));
}

// A hook configuration in Latin-1 is read as far as its ids go, so its other bytes do not stop the run.
#[test]
fn latin1_hook_configuration_still_counts() {
    let mut files = sibling_package("");
    files.push(("pyproject.toml".to_owned(), PROJECT));
    let root = project_of(&files);
    fs::write(
        root.path().join(".pre-commit-config.yaml"),
        b"repos:\n  - repo: x # caf\xe9\n    hooks:\n      - id: flake8-lazy\n",
    )
    .expect("write hook configuration");
    assert!(!reported(root.path()));
}

// A function that rebinds a module variable through `global` changes it after an import would have bound it.
#[test]
fn module_attribute_import_keeps_names_a_function_rebinds_through_global() {
    let root = common::project(&[
        ("pkg/__init__.py", ""),
        ("pkg/config.py", "_run = True\n\n\ndef set_run(run: bool) -> None:\n    global _run\n    _run = run\n"),
        ("pkg/setters.py", "from . import config\n\n\ndef check() -> bool:\n    return config._run\n"),
    ]);
    let stdout = common::check(root.path(), &["--unsafe-fixes", "--select", "SMP304"]).1;
    assert!(stdout.contains("SMP304 Import `_run`"));
    assert!(!stdout.contains("SMP304 [*]"));
}

// Ruff's F403 reports every star import, so SMP309 leaves the ones it cannot fix to it while the project's Ruff
// configuration runs F403. A file with two star imports fixes neither.
#[test_case("F403", "`from star_a import *` hides" ; "unfixable")]
fn star_import_follows_ruff_f403(code: &str, fragment: &str) {
    for selected in [false, true] {
        let select = if selected { format!("\"{code}\"") } else { String::new() };
        let root = common::project(&[
            ("pyproject.toml", &format!("[tool.ruff.lint]\nselect = [{select}]\n")),
            ("star_a.py", "def area(size: int) -> int:\n    return size\n"),
            ("star_b.py", "def double(size: int) -> int:\n    return 2 * size\n"),
            ("module.py", "from star_a import *\nfrom star_b import *\nprint(area(2), double(2))\n"),
        ]);
        let stdout = common::check(root.path(), &["--select", "SMP309"]).1;
        assert_eq!(stdout.contains(fragment), !selected, "{stdout}");
    }
}
