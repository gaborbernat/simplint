use ruff_python_ast::helpers::is_dunder;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashSet;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::exports::Exports;
use crate::rules::is_class_like;
use crate::rules::naming::rename_fix;
use crate::semantic::BindingKind;

pub fn module_variable_case(checker: &Checker) {
    const MAGIC: &[&str] = &["pytestmark", "pytest_plugins", "collect_ignore", "collect_ignore_glob", "TYPE_CHECKING"];
    // Sphinx reads its settings off the `conf.py` module by their lowercase names.
    if checker.file_name() == "conf.py" {
        return;
    }
    let exported = match checker.dunder_all() {
        Exports::Absent => None,
        Exports::Listed(names) => Some(names),
        Exports::Dynamic => return,
    };
    let read_by_name = framework_names(checker);
    let kept = &checker.settings.project.keep_names;
    let mut found: Vec<(&str, TextRange, String)> = Vec::new();
    for (name, bindings) in checker.semantic().all_bindings() {
        if is_dunder(name)
            || is_class_like(name)
            || MAGIC.contains(&name)
            || read_by_name.contains(name)
            || kept.iter().any(|kept| kept == name)
            || bindings
                .iter()
                .any(|binding| matches!(binding.kind, BindingKind::Import | BindingKind::Function | BindingKind::Class))
            || bindings.iter().all(|binding| binding.kind == BindingKind::Target)
        {
            continue;
        }
        let private = if checker.project().is_referenced(name) {
            name.starts_with('_')
        } else {
            !exported.is_some_and(|names| names.contains(&name))
        };
        let expected =
            format!("{}{}", if private { "_" } else { "" }, name.trim_start_matches('_').to_ascii_uppercase());
        if expected != name {
            found.push((name, bindings[0].range, expected));
        }
    }
    found.sort_by_key(|(_, range, _)| range.start());
    for (name, range, expected) in found {
        let fix = rename_fix(checker, name, &expected);
        checker.report_fix(
            Rule::ModuleVariableCase,
            range,
            format!("Module-scope variable `{name}` should be named `{expected}`"),
            fix,
            format!("Rename `{name}` to `{expected}`"),
        );
    }
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
    // WSGI and ASGI servers load the application by its module path and the name `app` or `application`.
    for stmt in checker.body() {
        let (target, value) = match stmt {
            Stmt::Assign(ast::StmtAssign { targets, value, .. }) => (targets.as_slice(), Some(value)),
            Stmt::AnnAssign(ast::StmtAnnAssign { target, value, .. }) => {
                (std::slice::from_ref(target.as_ref()), value.as_ref())
            }
            _ => continue,
        };
        if let ([Expr::Name(ast::ExprName { id, .. })], Some(value)) = (target, value)
            && matches!(id.as_str(), "app" | "application")
            && let Expr::Call(ast::ExprCall { func, .. }) = value.as_ref()
            && (APPLICATIONS.iter().any(|application| semantic.resolves_to(func, application))
                || matches!(func.as_ref(), Expr::Attribute(ast::ExprAttribute { attr, .. }) if attr == "make_wsgi_app"))
        {
            names.insert(id.as_str());
        }
    }
    names
}

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

// The calls that build a WSGI or ASGI application; Pyramid's comes from `Configurator.make_wsgi_app()` instead.
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
];
