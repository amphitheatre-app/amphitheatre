// Copyright (c) The Amphitheatre Authors. All rights reserved.
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

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Buildpack {
    id: String,
    order: Option<Vec<Order>>,
}

#[derive(Deserialize, Serialize, Default, Debug)]
pub struct Order {
    pub group: Vec<Group>,
}

#[derive(Deserialize, Serialize, Default, Debug)]
pub struct Group {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

pub fn find_top_level_buildpacks(buildpacks: &Vec<Buildpack>) -> Vec<Order> {
    let mut dependent_ids: HashSet<String> = HashSet::new();
    for buildpack in buildpacks {
        if let Some(order) = &buildpack.order {
            for group in order {
                for dependency in &group.group {
                    dependent_ids.insert(dependency.id.as_ref().unwrap().clone());
                }
            }
        }
    }

    let mut top_level_buildpack_ids: Vec<Order> = Vec::new();
    for buildpack in buildpacks {
        if !dependent_ids.contains(&buildpack.id) {
            top_level_buildpack_ids
                .push(Order { group: vec![Group { id: Some(buildpack.id.clone()), ..Default::default() }] })
        }
    }

    top_level_buildpack_ids
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_top_level_buildpacks() {
        let buildpacks = vec![
            Buildpack {
                id: "buildpack1".to_string(),
                order: Some(vec![Order {
                    group: vec![Group { id: Some("buildpack2".to_string()), ..Default::default() }],
                }]),
            },
            Buildpack {
                id: "buildpack2".to_string(),
                order: Some(vec![Order {
                    group: vec![Group { id: Some("buildpack3".to_string()), ..Default::default() }],
                }]),
            },
            Buildpack {
                id: "buildpack3".to_string(),
                order: Some(vec![Order {
                    group: vec![Group { id: Some("buildpack4".to_string()), ..Default::default() }],
                }]),
            },
            Buildpack { id: "buildpack4".to_string(), order: None },
        ];

        let top_level_buildpacks = find_top_level_buildpacks(&buildpacks);
        assert_eq!(top_level_buildpacks.len(), 1);
        assert_eq!(top_level_buildpacks[0].group[0].id, Some("buildpack1".to_string()));
    }
}
