use ruff_python_ast::{self as ast, Expr, Stmt};
use rustc_hash::FxHashSet;

use crate::lint::checker::Checker;
use crate::semantic::BindingKind;
use crate::syntax::single_assignment;

// The module-scope names a framework reads off this module by their exact names, so renaming them breaks it.
pub(super) fn framework_names<'a>(checker: &Checker<'a>) -> FxHashSet<&'a str> {
    const DB_API_GLOBALS: [&str; 3] = ["apilevel", "threadsafety", "paramstyle"];

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
pub(super) fn uses_ply(checker: &Checker) -> bool {
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

pub(super) fn is_ply_name(name: &str) -> bool {
    name.starts_with("t_")
        || name.starts_with("p_")
        || matches!(name, "tokens" | "literals" | "precedence" | "states" | "start")
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
