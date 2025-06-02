use crate::config::{self, History};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use log::warn;
use std::fs::File;
use std::{
    cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd},
    io::{BufRead, BufReader},
    path::PathBuf,
};
use std::{env, os::unix::fs::PermissionsExt, process::Command, fs};
use tokio::{
    io::{self, AsyncBufReadExt},
    task::{spawn, spawn_blocking},
};

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct Element {
    pub name: String,
    pub value: String,
    pub base_score: usize,
}

impl Ord for Element {
    fn cmp(&self, other: &Self) -> Ordering {
        match other.base_score.cmp(&self.base_score) {
            Ordering::Equal => self.name.cmp(&other.name),
            e => e,
        }
    }
}

impl PartialOrd for Element {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Default)]
pub struct ElementList {
    inner: Vec<Element>,
}

impl ElementList {
    pub fn merge_history(&mut self, history: &History) {
        for entry in history.as_vec() {
            if let Some(elem) = self.inner.iter_mut().find(|x| x.name == entry.name) {
                elem.base_score = entry.num_used;
            } else {
                self.inner.push(Element {
                    name: entry.name.clone(),
                    value: entry.value.clone(),
                    base_score: entry.num_used,
                });
            }
        }
    }

    pub fn sort_score(&mut self) {
        self.inner.sort_by(|a, b| b.base_score.cmp(&a.base_score));
    }

    pub fn search(&self, pattern: &str) -> Vec<&Element> {
        let matcher = SkimMatcherV2::default();
        let mut executables = self
            .inner
            .iter()
            .map(|x| {
                (
                    matcher
                        .fuzzy_match(&x.name, pattern)
                        .map(|score| score + x.base_score as i64),
                    x,
                )
            })
            .filter(|x| x.0.is_some())
            .collect::<Vec<(Option<i64>, &Element)>>();
        executables.sort_by(|a, b| b.0.unwrap_or(0).cmp(&a.0.unwrap_or(0)));
        executables.into_iter().map(|x| x.1).collect()
    }

    pub fn as_ref_vec(&self) -> Vec<&Element> {
        self.inner.iter().collect()
    }
}

#[derive(Debug, Default)]
pub struct ElementListBuilder {
    path_config: config::SearchConfig,
    from_path: bool,
    from_stdin: bool,
    from_file: Vec<PathBuf>,
    from_snap: bool,
    from_flatpak: bool,
    from_desktop: bool,
}

impl ElementListBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_path(&mut self, config: config::SearchConfig) {
        self.from_path = true;
        self.path_config = config;
    }
    pub fn add_files(&mut self, files: &[PathBuf]) {
        self.from_file = files.to_vec();
    }
    pub fn add_stdin(&mut self) {
        self.from_stdin = true;
    }

    pub fn add_snap(&mut self) {
        self.from_snap = true;
    }

    pub fn add_flatpak(&mut self) {
        self.from_flatpak = true;
    }

    pub fn add_desktop(&mut self) {
        self.from_desktop = true;
    }

    pub async fn build(&self) -> Result<ElementList, std::io::Error> {
        let mut fut = Vec::new();
        if self.from_stdin {
            fut.push(spawn(Self::build_stdin()));
        }
        if !self.from_file.is_empty() {
            let files = self.from_file.clone();
            fut.push(spawn_blocking(move || Self::build_files(&files)));
        }
        if self.from_path {
            let show_hidden = self.path_config.show_hidden_files;
            fut.push(spawn_blocking(move || Self::build_path(show_hidden)));
        }
        if self.from_snap {
            fut.push(spawn_blocking(Self::build_snap));
        }
        if self.from_flatpak {
            fut.push(spawn_blocking(Self::build_flatpak));
        }
        if self.from_desktop {
            fut.push(spawn_blocking(Self::build_desktop));
        }

        let finished = futures::future::join_all(fut).await;

        let mut res = Vec::new();
        for elements in finished {
            let mut elements = elements??;
            res.append(&mut elements);
        }

        Ok(ElementList { inner: res })
    }

    fn build_files(files: &[PathBuf]) -> Result<Vec<Element>, std::io::Error> {
        let mut res = Vec::new();
        for file in files {
            let mut reader = BufReader::new(File::open(file)?);
            let mut buf = String::new();
            let mut base_score = 0;

            while reader.read_line(&mut buf)? > 0 {
                let kv_pair = match parse_line(&buf) {
                    None => continue,
                    Some(res) => res,
                };
                match kv_pair {
                    ("%base_score", Some(value)) => {
                        if let Ok(value) = value.parse::<usize>() {
                            base_score = value;
                        }
                    }
                    (key, Some(value)) => res.push(Element {
                        name: key.to_string(),
                        value: value.to_string(),
                        base_score,
                    }),
                    ("", None) => {} // Empty Line
                    (key, None) => res.push(Element {
                        name: key.to_string(),
                        value: key.to_string(),
                        base_score,
                    }),
                }

                buf.clear();
            }
        }

        Ok(res)
    }

    fn build_path(show_hidden: bool) -> Result<Vec<Element>, std::io::Error> {
        let var = env::var("PATH").unwrap();

        let mut res: Vec<Element> = Vec::new();

        let paths_iter = env::split_paths(&var);
        let dirs_iter = paths_iter.filter_map(|path| std::fs::read_dir(path).ok());

        for dir in dirs_iter {
            dir.filter_map(Result::ok).for_each(|file| {
                if !show_hidden
                    && file
                        .file_name()
                        .to_str()
                        .is_some_and(|name| name.starts_with('.'))
                {
                    return;
                }
                if let Ok(metadata) = file.metadata() {
                    if !metadata.is_dir() && metadata.permissions().mode() & 0o111 != 0 {
                        let name = file.file_name().to_str().unwrap().to_string();
                        res.push(Element {
                            value: name.clone(),
                            name,
                            base_score: 0,
                        });
                    }
                }
            });
        }

        res.sort();
        res.dedup_by(|a, b| a.name == b.name);

        Ok(res)
    }

    async fn build_stdin() -> Result<Vec<Element>, std::io::Error> {
        let stdin = io::stdin();
        let reader = io::BufReader::new(stdin);
        let mut lines = reader.lines();
        let mut res = Vec::new();
        let mut base_score = 0;

        while let Some(line) = lines.next_line().await? {
            let kv_pair = match parse_line(&line) {
                None => continue,
                Some(res) => res,
            };
            match kv_pair {
                ("%base_score", Some(value)) => {
                    if let Ok(value) = value.parse::<usize>() {
                        base_score = value;
                    }
                }
                (key, Some(value)) => res.push(Element {
                    name: key.to_string(),
                    value: value.to_string(),
                    base_score,
                }),
                ("", None) => {} // Empty Line
                (key, None) => res.push(Element {
                    name: key.to_string(),
                    value: key.to_string(),
                    base_score,
                }),
            }
        }

        Ok(res)
    }

    fn build_snap() -> Result<Vec<Element>, std::io::Error> {
        let output = match Command::new("snap").arg("list").output() {
            Ok(output) => output,
            Err(_) => return Ok(Vec::new()), // snap not available
        };

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut res = Vec::new();

        // Skip the header line
        for line in stdout.lines().skip(1) {
            if let Some(name) = line.split_whitespace().next() {
                // Skip core snaps and system snaps
                if name.starts_with("core") || name == "snapd" {
                    continue;
                }
                res.push(Element {
                    name: name.to_string(),
                    value: name.to_string(),
                    base_score: 0,
                });
            }
        }

        Ok(res)
    }

    fn build_flatpak() -> Result<Vec<Element>, std::io::Error> {
        let output = match Command::new("flatpak")
            .args(&["list", "--app", "--columns=application,name"])
            .output()
        {
            Ok(output) => output,
            Err(_) => return Ok(Vec::new()), // flatpak not available
        };

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut res = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 2 {
                let app_id = parts[0].trim();
                let display_name = parts[1].trim();
                
                if !app_id.is_empty() {
                    let name = if display_name.is_empty() {
                        // Use the app ID without the domain part as display name
                        app_id.split('.').last().unwrap_or(app_id).to_string()
                    } else {
                        display_name.to_string()
                    };
                    
                    res.push(Element {
                        name,
                        value: format!("flatpak run {}", app_id),
                        base_score: 0,
                    });
                }
            }
        }

        Ok(res)
    }

    fn build_desktop() -> Result<Vec<Element>, std::io::Error> {
        let mut res = Vec::new();
        
        // Standard desktop file locations
        let desktop_dirs = [
            "/usr/share/applications",
            "/usr/local/share/applications",
            &format!("{}/.local/share/applications", env::var("HOME").unwrap_or_default()),
        ];

        for dir_path in &desktop_dirs {
            if let Ok(entries) = fs::read_dir(dir_path) {
                for entry in entries.flatten() {
                    if let Some(file_name) = entry.file_name().to_str() {
                        if file_name.ends_with(".desktop") {
                            if let Ok(content) = fs::read_to_string(entry.path()) {
                                if let Some(element) = Self::parse_desktop_file(&content) {
                                    res.push(element);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Remove duplicates by name, keeping the first occurrence
        res.sort_by(|a, b| a.name.cmp(&b.name));
        res.dedup_by(|a, b| a.name == b.name);

        Ok(res)
    }

    fn parse_desktop_file(content: &str) -> Option<Element> {
        let mut name = None;
        let mut exec = None;
        let mut hidden = false;
        let mut no_display = false;
        let mut app_type = None;
        let mut in_desktop_entry = false;

        for line in content.lines() {
            let line = line.trim();
            
            if line == "[Desktop Entry]" {
                in_desktop_entry = true;
                continue;
            } else if line.starts_with('[') && line.ends_with(']') {
                in_desktop_entry = false;
                continue;
            }

            if !in_desktop_entry {
                continue;
            }

            if let Some(equals_pos) = line.find('=') {
                let key = &line[..equals_pos];
                let value = &line[equals_pos + 1..];

                match key {
                    "Name" => name = Some(value.to_string()),
                    "Exec" => exec = Some(value.to_string()),
                    "Hidden" => hidden = value.eq_ignore_ascii_case("true"),
                    "NoDisplay" => no_display = value.eq_ignore_ascii_case("true"),
                    "Type" => app_type = Some(value.to_string()),
                    _ => {}
                }
            }
        }

        if hidden {
            return None;
        }
        
        // Allow settings applications even if NoDisplay=true (like Cosmic settings panels)
        let is_settings = app_type.as_ref().map_or(false, |t| t == "Settings");
        let is_cosmic_settings = exec.as_ref().map_or(false, |e| e.contains("cosmic-settings"));
        
        if no_display && !is_settings && !is_cosmic_settings {
            return None;
        }

        if let (Some(name), Some(mut exec)) = (name, exec) {
            // Clean up exec command - remove field codes like %f, %F, %u, %U
            exec = exec.replace("%f", "").replace("%F", "")
                      .replace("%u", "").replace("%U", "")
                      .replace("%i", "").replace("%c", "")
                      .replace("%k", "").trim().to_string();

            Some(Element {
                name,
                value: exec,
                base_score: 0,
            })
        } else {
            None
        }
    }
}

#[allow(clippy::type_complexity)]
fn parse_line(input: &str) -> Option<(&str, Option<&str>)> {
    let input = input.trim();
    let parts = input.splitn(2, '=').map(str::trim).collect::<Vec<&str>>();

    if parts.is_empty() {
        warn!("Failed to pares line: {input}");
        None
    } else {
        Some((parts.first().unwrap(), parts.get(1).copied()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_line_test() {
        assert_eq!(parse_line("foobar"), Some(("foobar", None)));
        assert_eq!(parse_line("foo=bar"), Some(("foo", Some("bar"))));
        assert_eq!(
            parse_line("foo=bar\"baz\""),
            Some(("foo", Some("bar\"baz\"")))
        );
        assert_eq!(
            parse_line(
                r#"Desktop: Firefox Developer Edition - New Window=/usr/lib/firefox-developer-edition/firefox --class="firefoxdeveloperedition" --new-window %u"#
            ),
            Some((
                "Desktop: Firefox Developer Edition - New Window",
                Some(
                    r#"/usr/lib/firefox-developer-edition/firefox --class="firefoxdeveloperedition" --new-window %u"#
                )
            ))
        );
    }
}
