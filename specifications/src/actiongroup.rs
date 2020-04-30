use crate::common::{Argument, FunctionNotation, Type};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use serde_yaml;
use std::fs;
use std::path::PathBuf;

type Map<T> = std::collections::HashMap<String, T>;
type FResult<T> = Result<T, failure::Error>;

#[skip_serializing_none]
#[serde(rename_all = "camelCase")]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ActionGroup {
    pub actions: Map<Action>,
    pub base: Option<String>,
    pub contributors: Option<Vec<String>>,
    pub description: Option<String>,
    pub entrypoint: Entrypoint,
    pub environment: Option<Map<String>>,
    pub dependencies: Option<Vec<String>>,
    pub files: Option<Vec<String>>,
    pub initialize: Option<Vec<String>>,
    pub install: Option<Vec<String>>,
    pub kind: String,
    pub name: String,
    pub types: Option<Map<Type>>,
    pub version: String,
}

#[allow(unused)]
impl ActionGroup {
    pub fn from_path(path: PathBuf) -> FResult<ActionGroup> {
        let contents = fs::read_to_string(path)?;

        ActionGroup::from_string(contents)
    }

    pub fn from_string(contents: String) -> FResult<ActionGroup> {
        let result = serde_yaml::from_str(&contents)?;

        Ok(result)
    }
}

#[skip_serializing_none]
#[serde(rename_all = "camelCase")]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Action {
    pub command: Option<ActionCommand>,
    pub description: Option<String>,
    pub endpoint: Option<ActionEndpoint>,
    pub notation: Option<FunctionNotation>,
    pub input: Vec<Argument>,
    pub output: Vec<Argument>,
}

#[skip_serializing_none]
#[serde(rename_all = "camelCase")]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ActionCommand {
    pub args: Vec<String>,
    pub capture: Option<String>,
}

#[skip_serializing_none]
#[serde(rename_all = "camelCase")]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ActionEndpoint {
    pub method: Option<String>,
    pub path: String,
}

#[skip_serializing_none]
#[serde(rename_all = "camelCase")]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Entrypoint {
    pub kind: String,
    pub exec: String,
    pub content: Option<String>,
    pub delay: Option<u64>,
}
