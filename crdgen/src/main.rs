// Copyright 2023 The Amphitheatre Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::collections::HashMap;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use amp_common::resource::{Actor, Character, Playbook};
use clap::Parser;
use kube::CustomResourceExt;
use serde::Serialize;

/// Generate custom resource definitions for Amphitheatre.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Print the names of the custom resource definition.
    #[arg(short, long)]
    list: bool,
    /// Names of the custom resource definition, separated by comma.
    #[arg(short, long)]
    names: Option<String>,
    /// Which output path to write to, If not specified, will print to stdout.
    #[arg(short, long)]
    output: Option<String>,
}

fn main() {
    let mappings = HashMap::from([
        ("actor", ("actor.yaml", Actor::crd())),
        ("character", ("character.yaml", Character::crd())),
        ("playbook", ("playbook.yaml", Playbook::crd())),
    ]);

    let args = Args::parse();
    let mut all_names: Vec<&str> = mappings.keys().copied().collect();
    all_names.sort();

    // Print the names of the custom resource definition sorted by name.
    if args.list {
        for name in all_names {
            println!("{}", name);
        }
        return;
    }

    // Parse the inputted names, if not specified, use all names.
    let names: Vec<&str> = args.names.as_ref().map(|s| s.split(',').collect()).unwrap_or(all_names);

    // Check the names are valid.
    for name in &names {
        if !mappings.contains_key(name) {
            eprintln!("The given name is not valid: {}", name);
            std::process::exit(1);
        }
    }

    let mut dir: Option<&Path> = None;
    if let Some(output) = &args.output {
        let path = Path::new(output);
        if !path.exists() {
            eprintln!("The given output path is not exists");
            std::process::exit(1);
        }
        dir = Some(path);
    }

    for name in names {
        let (filename, data) = mappings.get(name).unwrap();
        generate(dir, filename, data);
    }
}

/// Generate custom resource definitions with the given output path and filename.
fn generate<T>(dir: Option<&Path>, filename: &str, data: &T)
where
    T: ?Sized + Serialize,
{
    let definition = serde_yaml::to_string(data).unwrap();

    if let Some(dir) = dir {
        write(&dir.join(filename), definition);
    } else {
        println!("{}\n---\n", definition);
    }
}

/// Write the given data to the given path.
fn write(path: &Path, data: String) {
    if path.exists() {
        fs::remove_file(path).unwrap();
    }

    let mut file = OpenOptions::new().create_new(true).write(true).open(path).unwrap();
    if let Err(e) = write!(file, "{}", data) {
        eprintln!("Couldn't write to file: {}", e);
    }
}

mod test {
    #[test]
    fn test_main_with_list() {
        let output = std::process::Command::new("cargo")
            .args(["run", "-p", "amp-crdgen", "--", "--list"])
            .output()
            .expect("failed to execute process");

        let stdout = String::from_utf8(output.stdout).unwrap();
        assert_eq!(stdout, "actor\ncharacter\nplaybook\n");
    }

    #[test]
    fn test_main_with_names() {
        let output = std::process::Command::new("cargo")
            .args(["run", "-p", "amp-crdgen", "--", "--names", "actor,character"])
            .output()
            .expect("failed to execute process");

        let stdout = String::from_utf8(output.stdout).unwrap();

        // assert the output contains the actor and character.
        assert!(stdout.contains("actors.amphitheatre.app"));
        assert!(stdout.contains("characters.amphitheatre.app"));
    }

    #[test]
    fn test_main_with_invalid_names() {
        let output = std::process::Command::new("cargo")
            .args(["run", "-p", "amp-crdgen", "--", "--names", "invalid"])
            .output()
            .expect("failed to execute process");

        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains("The given name is not valid: invalid"));
    }

    /// test the output path.
    /// use tempfile to create a temporary directory.
    #[test]
    fn test_main_with_output() {
        let tempdir = tempfile::tempdir().unwrap();
        let output = std::process::Command::new("cargo")
            .args([
                "run",
                "-p",
                "amp-crdgen",
                "--",
                "--names",
                "actor,character",
                "--output",
                tempdir.path().to_str().unwrap(),
            ])
            .output()
            .expect("failed to execute process");

        let stdout = String::from_utf8(output.stdout).unwrap();
        assert_eq!(stdout, "");

        // assert the output contains the actor and character.
        let actor = tempdir.path().join("actor.yaml");
        let character = tempdir.path().join("character.yaml");
        assert!(actor.exists());
        assert!(character.exists());
    }

    #[test]
    fn test_main_with_empty_args() {
        let output = std::process::Command::new("cargo")
            .args(["run", "-p", "amp-crdgen", "--"])
            .output()
            .expect("failed to execute process");

        let stdout = String::from_utf8(output.stdout).unwrap();
        // assert the output contains the all names.
        assert!(stdout.contains("actors.amphitheatre.app"));
        assert!(stdout.contains("characters.amphitheatre.app"));
        assert!(stdout.contains("playbooks.amphitheatre.app"));
    }
}
