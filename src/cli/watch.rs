use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc::channel;

use anyhow::Result;
use notify::{RecursiveMode, Watcher, recommended_watcher};
use ruff::ExitStatus;

use crate::cli::args::{CheckCommand, GlobalConfigArgs};
use crate::cli::printer::Printer;
use crate::project::Ending;
use crate::run::{CacheUse, Resolved, check_files};
use crate::settings::HOOK_CONFIGS;

pub(super) fn watch(
    cli: &CheckCommand,
    global_options: &GlobalConfigArgs,
    files: &[PathBuf],
    mut resolved: Resolved,
    usage: &CacheUse,
    printer: &Printer,
    mut writer: Box<dyn Write>,
) -> Result<ExitStatus> {
    let (sender, receiver) = channel();
    let mut watcher = recommended_watcher(sender)?;
    for file in files.iter().chain(&resolved.pyproject_config.path).chain(&resolved.settings_resolver.root.path) {
        watcher.watch(file, RecursiveMode::Recursive)?;
    }
    clearscreen::clear()?;
    printer.write_to_user("Starting linter in watch mode...\n");
    loop {
        printer.write_continuously(&mut writer, &check_files(files, &resolved, printer.mode, usage, Ending::Drop)?)?;
        let change = loop {
            if let Some(change) = change_detected(&receiver.recv()??) {
                break change;
            }
        };
        if change == ChangeKind::Configuration {
            resolved = Resolved::new(cli, global_options)?;
        }
        clearscreen::clear()?;
        printer.write_to_user("File change detected...\n");
    }
}

// A change to a `.toml` file or a hook configuration can change the configuration; one to a Python file only needs
// another check. Reads and other events need nothing.
fn change_detected(event: &notify::Event) -> Option<ChangeKind> {
    // notify flags a rescan only on an event it reports as other, so the rescan branch Ruff keeps after this check
    // never runs.
    if event.kind.is_access() || event.kind.is_other() {
        return None;
    }
    let mut source_file = false;
    for path in &event.paths {
        if path.file_name().and_then(|name| name.to_str()).is_some_and(|name| HOOK_CONFIGS.contains(&name)) {
            return Some(ChangeKind::Configuration);
        }
        match path.extension().and_then(|extension| extension.to_str()) {
            Some("toml") => return Some(ChangeKind::Configuration),
            Some("py" | "pyi" | "pyw" | "ipynb") => source_file = true,
            _ => {}
        }
    }
    source_file.then_some(ChangeKind::SourceFile)
}

// Ruff's watch mode change detection, which the `ruff` crate keeps private.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChangeKind {
    Configuration,
    SourceFile,
}
