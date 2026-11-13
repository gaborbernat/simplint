use std::convert::Infallible;
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc::{Sender, channel};

use anyhow::Result;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher, recommended_watcher};
use ruff::ExitStatus;

use crate::cli::args::{CheckCommand, GlobalConfigArgs};
use crate::cli::printer::Printer;
use crate::project::Ending;
use crate::run::{CacheUse, Resolved, check_files};
use crate::settings::HOOK_CONFIGS;

// Ruff's watch mode: it checks again on every change to a watched Python file, and resolves the configuration again
// when a `.toml` file changes. It runs until watching fails or a check cannot run.
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
    // Kept until watching ends, as dropping the watcher stops the events.
    let _watcher = watcher(
        sender,
        files.iter().chain(&resolved.pyproject_config.path).chain(&resolved.settings_resolver.root.path),
    )?;
    let Err(error) = rounds(
        || receiver.recv().expect("the watcher keeps its channel open"),
        |change| {
            if change == Some(ChangeKind::Configuration) {
                resolved = Resolved::new(cli, global_options)?;
            }
            clearscreen::clear()?;
            printer.write_to_user(if change.is_some() {
                "File change detected...\n"
            } else {
                "Starting linter in watch mode...\n"
            });
            printer.write_continuously(&mut writer, &check_files(files, &resolved, printer.mode, usage, Ending::Drop)?)
        },
    );
    Err(error)
}

fn watcher<'a>(
    sender: Sender<notify::Result<Event>>,
    paths: impl IntoIterator<Item = &'a PathBuf>,
) -> notify::Result<RecommendedWatcher> {
    let mut watcher = recommended_watcher(sender)?;
    for file in paths {
        watcher.watch(file, RecursiveMode::Recursive)?;
    }
    Ok(watcher)
}

// Runs a round, then another on each change the events report, until a round or the events fail.
fn rounds(
    mut next_event: impl FnMut() -> notify::Result<notify::Event>,
    mut round: impl FnMut(Option<ChangeKind>) -> Result<()>,
) -> Result<Infallible> {
    round(None)?;
    loop {
        if let Some(change) = change_detected(&next_event()?) {
            round(Some(change))?;
        }
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

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use anyhow::anyhow;
    use notify::event::{AccessKind, ModifyKind};
    use notify::{Event, EventKind};

    use super::{ChangeKind, rounds, watcher};

    fn modified(path: &str) -> Event {
        Event::new(EventKind::Modify(ModifyKind::Any)).add_path(path.into())
    }

    // The rounds run so far, and the events still to come; a round fails once the events run out.
    fn run(events: Vec<notify::Result<Event>>) -> (String, Vec<Option<ChangeKind>>) {
        let events = RefCell::new(events.into_iter());
        let mut seen = Vec::new();
        let Err(error) = rounds(
            || events.borrow_mut().next().expect("an event"),
            |change| {
                seen.push(change);
                if events.borrow().len() == 0 { Err(anyhow!("round failed")) } else { Ok(()) }
            },
        );
        (error.to_string(), seen)
    }

    #[test]
    fn a_failed_first_round_ends_watching() {
        assert_eq!(run(Vec::new()), ("round failed".to_owned(), vec![None]));
    }

    #[test]
    fn a_failed_event_ends_watching() {
        let (error, seen) = run(vec![Err(notify::Error::generic("lost")), Ok(modified("module.py"))]);

        assert_eq!((error.as_str(), seen), ("lost", vec![None]));
    }

    #[test]
    fn each_relevant_change_runs_a_round() {
        let access = Ok(Event::new(EventKind::Access(AccessKind::Any)).add_path("module.py".into()));

        let (error, seen) = run(vec![access, Ok(modified("pyproject.toml")), Ok(modified("module.py"))]);

        assert_eq!(
            (error.as_str(), seen),
            ("round failed", vec![None, Some(ChangeKind::Configuration), Some(ChangeKind::SourceFile)])
        );
    }

    // A path that does not exist cannot be watched, which fails watching before any round.
    #[test]
    fn watching_a_missing_path_fails() {
        let directory = tempfile::TempDir::new().expect("temporary directory");
        let (sender, _receiver) = std::sync::mpsc::channel();
        assert!(watcher(sender, [&directory.path().join("missing")]).is_err());
    }

    // The watcher needs file descriptors of its own, so it fails to start in a process that has none left.
    #[cfg(target_os = "linux")]
    #[test]
    fn watcher_without_a_free_file_descriptor_fails() {
        crate::limits::limited(
            concat!(module_path!(), "::watcher_without_a_free_file_descriptor_fails"),
            crate::limits::Limit::OpenFiles,
            || (),
            |()| {
                let (sender, _receiver) = std::sync::mpsc::channel();
                assert!(watcher(sender, std::iter::empty()).is_err());
            },
        );
    }
}
