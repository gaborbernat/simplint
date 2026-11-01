use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

// The `[project]` table of a `pyproject.toml`, reduced to what simplint's rules read: the modules its unconditional
// requirements install, and the objects its scripts and entry points name.
#[derive(Deserialize, Default)]
#[serde(from = "ProjectTable")]
pub struct ProjectMetadata {
    pub dependencies: BTreeSet<String>,
    pub entry_points: Vec<String>,
}

impl From<ProjectTable> for ProjectMetadata {
    fn from(project: ProjectTable) -> Self {
        Self {
            // A requirement with an environment marker is conditional, so a guarded import of it is legitimate.
            dependencies: project
                .dependencies
                .iter()
                .filter(|spec| !spec.contains(';'))
                .flat_map(|spec| import_names(spec))
                .collect(),
            entry_points: project
                .scripts
                .into_values()
                .chain(project.gui_scripts.into_values())
                .chain(project.entry_points.into_values().flat_map(BTreeMap::into_values))
                .collect(),
        }
    }
}

fn import_names(requirement: &str) -> Vec<String> {
    // Distributions whose import name differs from the project name; the rest import under their normalized name.
    const RENAMED: &[(&str, &str)] = &[
        ("attrs", "attr"),
        ("beautifulsoup4", "bs4"),
        ("opencv_python", "cv2"),
        ("opencv_python_headless", "cv2"),
        ("pillow", "pil"),
        ("protobuf", "google"),
        ("pyjwt", "jwt"),
        ("python_dateutil", "dateutil"),
        ("python_dotenv", "dotenv"),
        ("pyyaml", "yaml"),
        ("pyzmq", "zmq"),
        ("scikit_learn", "sklearn"),
        ("setuptools", "pkg_resources"),
    ];
    let name: String = requirement
        .chars()
        .take_while(|char| char.is_ascii_alphanumeric() || matches!(char, '-' | '_' | '.'))
        .map(|char| if matches!(char, '-' | '.') { '_' } else { char.to_ascii_lowercase() })
        .collect();
    let renamed = RENAMED.iter().filter(|(project, _)| *project == name).map(|(_, import)| (*import).to_owned());
    renamed.chain([name.clone()]).collect()
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "kebab-case")]
struct ProjectTable {
    dependencies: Vec<String>,
    scripts: BTreeMap<String, String>,
    gui_scripts: BTreeMap<String, String>,
    entry_points: BTreeMap<String, BTreeMap<String, String>>,
}
