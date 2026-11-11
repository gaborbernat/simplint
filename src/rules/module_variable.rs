// The module-scope variables the case and prefix rules check. Names a framework or tool reads by their exact spelling,
// names a loop, `with`, `except` or `match` binds, and annotations alone under `if TYPE_CHECKING:` are left out.

use ruff_python_ast::helpers::is_dunder;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashSet;

use crate::lint::checker::Checker;
use crate::rules::typing::type_parameter_assignment;
use crate::rules::{is_class_like, is_sphinx_conf, single_assignment};
use crate::semantic::{Binding, BindingKind};
use crate::syntax::{Exports, Visibility};

pub struct ModuleVariable<'a> {
    pub name: &'a str,
    pub range: TextRange,
    // Whether the name should have a leading underscore. An imported name keeps the underscore it has, and any other
    // name is private unless `__all__` lists it.
    pub private: bool,
}

// Sorted by position, so the rules report them in the order they appear.
pub fn module_variables<'a>(checker: &Checker<'a>) -> Vec<ModuleVariable<'a>> {
    const MAGIC: &[&str] = &["pytestmark", "pytest_plugins", "collect_ignore", "collect_ignore_glob", "TYPE_CHECKING"];
    if is_sphinx_conf(checker) {
        return Vec::new();
    }
    let exported = match checker.dunder_all() {
        Exports::Absent => None,
        Exports::Listed(names) => Some(names),
        Exports::Dynamic => return Vec::new(),
    };
    let read_by_name = framework_names(checker);
    let aliases = function_aliases(checker);
    let type_parameters: FxHashSet<&str> = checker
        .body()
        .iter()
        .filter_map(|stmt| type_parameter_assignment(checker, stmt))
        .map(|target| target.id.as_str())
        .collect();
    let kept = &checker.settings.project.knobs.keep_names;
    let mut variables = Vec::new();
    let stub = |binding: &Binding| checker.semantic().is_type_checking_stub(binding.range);
    for (name, bindings) in checker.semantic().all_bindings() {
        let Some(first) = bindings.iter().find(|binding| !stub(binding)) else {
            continue;
        };
        if is_dunder(name)
            || is_class_like(name)
            || MAGIC.contains(&name)
            || read_by_name.contains(name)
            || aliases.contains(name)
            || (type_parameters.contains(name)
                && bindings.iter().filter(|binding| !stub(binding)).all(|binding| binding.range == first.range))
            || kept.iter().any(|kept| kept == name)
            || bindings.iter().any(|binding| {
                matches!(
                    binding.kind,
                    BindingKind::Import | BindingKind::Function | BindingKind::Class | BindingKind::TypeAlias
                )
            })
            || bindings.iter().filter(|binding| !stub(binding)).all(|binding| binding.kind == BindingKind::Target)
            || is_block_local(checker, name, bindings)
        {
            continue;
        }
        let private = if checker.project().is_used_elsewhere(name) {
            Visibility::of(name) != Visibility::Public
        } else {
            !exported.is_some_and(|names| names.contains(name))
        };
        variables.push(ModuleVariable { name, range: first.range, private });
    }
    variables.sort_by_key(|variable| variable.range.start());
    variables
}

// A name that one module-level `if`, `try` or `with` block binds and reads, and nothing outside the block reads, works as
// a local of that block, as `msg` does before a `raise` in a version check. A name read by another file stays a module
// variable.
fn is_block_local(checker: &Checker, name: &str, bindings: &[Binding]) -> bool {
    let body = checker.body();
    let references = checker.semantic().references_to(name);
    let Some(block) = references.first().map(|reference| reference.statement) else {
        return false;
    };
    matches!(body.get(block), Some(Stmt::If(_) | Stmt::Try(_) | Stmt::With(_)))
        && references.iter().all(|reference| reference.statement == block)
        && bindings.iter().all(|binding| body.partition_point(|stmt| stmt.end() <= binding.range.start()) == block)
        && !checker.project().is_used_elsewhere(name)
}

// The module-scope names bound to a function the module defines, as `_build_cli = build_cli`. Such an alias reads as the
// function, so it follows the function's name as a class alias does. An import is left out: the semantic model cannot
// tell whether it binds a function.
fn function_aliases<'a>(checker: &Checker<'a>) -> FxHashSet<&'a str> {
    let semantic = checker.semantic();
    checker
        .body()
        .iter()
        .filter_map(single_assignment)
        .filter(|(_, _, value)| match value {
            Expr::Name(name) => {
                semantic.bindings(name.id.as_str()).iter().any(|binding| binding.kind == BindingKind::Function)
            }
            _ => false,
        })
        .map(|(target, _, _)| target.id.as_str())
        .collect()
}

// The module-scope names a framework reads off this module by their exact names, so renaming them breaks it.
fn framework_names<'a>(checker: &Checker<'a>) -> FxHashSet<&'a str> {
    let semantic = checker.semantic();
    let bound = |name: &str| !semantic.bindings(name).is_empty();
    let mut names = FxHashSet::default();
    // An Alembic revision file names its place in the migration graph, next to its `upgrade` function.
    if bound("revision")
        && bound("down_revision")
        && semantic.bindings("upgrade").iter().any(|binding| binding.kind == BindingKind::Function)
    {
        names.extend(["revision", "down_revision", "branch_labels", "depends_on"]);
    }
    // A gunicorn configuration file holds its settings as module variables. Any file can be one, so outside a file
    // named after gunicorn it takes three settings, one of them a core server setting, to tell it from a module that
    // happens to define a `timeout`.
    let settings: Vec<&str> = GUNICORN_SETTINGS.iter().copied().filter(|setting| bound(setting)).collect();
    if checker.file_name().contains("gunicorn")
        || settings.len() >= 3 && settings.iter().any(|setting| matches!(*setting, "bind" | "workers" | "worker_class"))
    {
        names.extend(settings);
    }
    // setuptools-scm and hatch-vcs write a version file whose lowercase names tools and callers read, and projects that
    // write their own keep the same names alongside `__version__`.
    if bound("__version__") {
        names.extend(["version", "version_tuple", "commit_id"].into_iter().filter(|name| bound(name)));
    }
    // PEP 249 has a database driver publish its API level, thread safety and parameter style as module globals.
    if DB_API_GLOBALS.iter().all(|name| bound(name)) {
        names.extend(DB_API_GLOBALS);
    }
    // PLY writes its parser and lexer tables as modules, and reads every table back by name next to `_tabversion`.
    if bound("_tabversion") {
        names.extend(semantic.all_bindings().map(|(name, _)| name));
    }
    if uses_ply(checker) {
        names.extend(semantic.all_bindings().map(|(name, _)| name).filter(|name| is_ply_name(name)));
    }
    // WSGI and ASGI servers, and the `typer` command, load the application by its module path and the name `app` or
    // `application`.
    for stmt in checker.body() {
        if let Some((target, _, value)) = single_assignment(stmt)
            && matches!(target.id.as_str(), "app" | "application")
            && let Expr::Call(ast::ExprCall { func, .. }) = value
            && (APPLICATIONS.iter().any(|application| semantic.resolves_to(func, application))
                || matches!(func.as_ref(), Expr::Attribute(ast::ExprAttribute { attr, .. }) if attr == "make_wsgi_app"))
        {
            names.insert(target.id.as_str());
        }
    }
    names
}

// PLY's `lex` and `yacc` build a lexer and a parser from the names the calling module defines, so a module that imports
// either reads its grammar by name.
pub fn uses_ply(checker: &Checker) -> bool {
    checker.body().iter().any(|stmt| match stmt {
        Stmt::Import(ast::StmtImport { names, .. }) => names.iter().any(|alias| is_ply_module(alias.name.as_str())),
        Stmt::ImportFrom(ast::StmtImportFrom { module, names, .. }) => {
            let module = module.as_ref().map_or("", ast::Identifier::as_str);
            is_ply_module(module) || names.iter().any(|alias| is_ply_module(&format!("{module}.{}", alias.name)))
        }
        _ => false,
    })
}

// PLY is often vendored, as `jsonpath_ng._ply`, so any module under a package named after it counts.
fn is_ply_module(module: &str) -> bool {
    let mut parts = module.rsplit('.');
    parts.next().is_some_and(|last| matches!(last, "lex" | "yacc")) && parts.any(|part| part.contains("ply"))
}

// The token list, the `t_` lexer rules and `p_` grammar productions, and the settings PLY reads off a grammar module.
pub fn is_ply_name(name: &str) -> bool {
    name.starts_with("t_")
        || name.starts_with("p_")
        || matches!(name, "tokens" | "literals" | "precedence" | "states" | "start")
}

const DB_API_GLOBALS: [&str; 3] = ["apilevel", "threadsafety", "paramstyle"];

// The setting names of gunicorn's configuration file, from its `gunicorn/config.py`.
const GUNICORN_SETTINGS: &[&str] = &[
    "access_log_format",
    "accesslog",
    "asgi_disconnect_grace_period",
    "asgi_lifespan",
    "asgi_loop",
    "backlog",
    "bind",
    "ca_certs",
    "capture_output",
    "casefold_http_method",
    "cert_reqs",
    "certfile",
    "chdir",
    "check_config",
    "child_exit",
    "ciphers",
    "config",
    "control_socket",
    "control_socket_disable",
    "control_socket_mode",
    "daemon",
    "default_proc_name",
    "dirty_apps",
    "dirty_graceful_timeout",
    "dirty_post_fork",
    "dirty_threads",
    "dirty_timeout",
    "dirty_worker_exit",
    "dirty_worker_init",
    "dirty_workers",
    "disable_redirect_access_to_syslog",
    "do_handshake_on_connect",
    "dogstatsd_tags",
    "enable_backlog_metric",
    "enable_stdio_inheritance",
    "errorlog",
    "forwarded_allow_ips",
    "forwarder_headers",
    "graceful_timeout",
    "group",
    "header_map",
    "http2_cleartext",
    "http2_initial_window_size",
    "http2_max_concurrent_streams",
    "http2_max_frame_size",
    "http2_max_header_list_size",
    "http_parser",
    "http_protocols",
    "initgroups",
    "keepalive",
    "keyfile",
    "limit_request_field_size",
    "limit_request_fields",
    "limit_request_line",
    "logconfig",
    "logconfig_dict",
    "logconfig_json",
    "logger_class",
    "loglevel",
    "max_requests",
    "max_requests_jitter",
    "nworkers_changed",
    "on_dirty_starting",
    "on_exit",
    "on_reload",
    "on_starting",
    "paste",
    "permit_obsolete_folding",
    "permit_unconventional_http_method",
    "permit_unconventional_http_version",
    "pidfile",
    "post_fork",
    "post_request",
    "post_worker_init",
    "pre_exec",
    "pre_fork",
    "pre_request",
    "preload_app",
    "print_config",
    "proc_name",
    "protocol",
    "proxy_allow_ips",
    "proxy_protocol",
    "pythonpath",
    "raw_env",
    "raw_paste_global_conf",
    "reload",
    "reload_engine",
    "reload_extra_files",
    "reuse_port",
    "root_path",
    "secure_scheme_headers",
    "sendfile",
    "spew",
    "ssl_context",
    "ssl_version",
    "statsd_host",
    "statsd_prefix",
    "strip_header_spaces",
    "suppress_ragged_eofs",
    "syslog",
    "syslog_addr",
    "syslog_facility",
    "syslog_prefix",
    "threads",
    "timeout",
    "tmp_upload_dir",
    "umask",
    "user",
    "uwsgi_allow_ips",
    "when_ready",
    "worker_abort",
    "worker_class",
    "worker_connections",
    "worker_exit",
    "worker_int",
    "worker_tmp_dir",
    "workers",
    "wsgi_app",
];

// The calls that build a WSGI, ASGI or Typer application; Pyramid's comes from `Configurator.make_wsgi_app()` instead.
const APPLICATIONS: &[&[&str]] = &[
    &["aiohttp", "web", "Application"],
    &["bottle", "Bottle"],
    &["django", "core", "asgi", "get_asgi_application"],
    &["django", "core", "wsgi", "get_wsgi_application"],
    &["falcon", "API"],
    &["falcon", "App"],
    &["falcon", "asgi", "App"],
    &["fastapi", "FastAPI"],
    &["flask", "Flask"],
    &["litestar", "Litestar"],
    &["quart", "Quart"],
    &["sanic", "Sanic"],
    &["starlette", "applications", "Starlette"],
    &["typer", "Typer"],
    &["typer", "main", "Typer"],
];
