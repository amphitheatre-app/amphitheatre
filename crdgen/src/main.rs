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

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::{env, fs};

use amp_common::schema::{Actor, Character, Playbook};
use kube::CustomResourceExt;

fn main() {
    let args: Vec<String> = env::args().collect();

    let actor_definition = serde_yaml::to_string(&Actor::crd()).unwrap();
    let character_definition = serde_yaml::to_string(&Character::crd()).unwrap();
    let playbook_definition = serde_yaml::to_string(&Playbook::crd()).unwrap();

    if args.len() == 2 {
        let dir = Path::new(&args[1]);

        write(&dir.join("actor.yaml"), actor_definition);
        write(&dir.join("character.yaml"), character_definition);
        write(&dir.join("playbook.yaml"), playbook_definition);

        return;
    }

    print!(
        "{}\n---\n{}\n---\n{}",
        actor_definition, character_definition, playbook_definition
    )
}

fn write(path: &Path, data: String) {
    if path.exists() {
        fs::remove_file(path).unwrap();
    }

    let mut file = OpenOptions::new().create_new(true).write(true).open(path).unwrap();
    if let Err(e) = write!(file, "{}", data) {
        eprintln!("Couldn't write to file: {}", e);
    }
}
