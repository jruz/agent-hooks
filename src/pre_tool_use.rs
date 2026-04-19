mod claude;
mod codex;
mod filters;

use std::collections::HashMap;
use std::io::{self, Read as _};

use serde_json::Value;
use thiserror::Error;

use crate::shared::agent::{self, Agent};

#[derive(Error, Debug)]
#[error("{reason}")]
pub struct PreToolUseError {
    log_attributes: HashMap<String, String>,
    reason: String,
}

pub fn run(agent: Agent) -> Result<Option<String>, PreToolUseError> {
    let stdin = match read_stdin() {
        Ok(stdin) => stdin,
        Err(err) => {
            let error = io_error(agent, &err);
            return Err(error);
        }
    };

    let log_attributes = payload_log_attributes(agent, &stdin);

    match run_agent(agent, &stdin) {
        Ok(output) => Ok(output),
        Err(reason) => {
            let error = pre_tool_use_error(log_attributes, reason);
            Err(error)
        }
    }
}

pub fn log_attributes(error: &PreToolUseError) -> HashMap<String, String> {
    error.log_attributes.clone()
}

fn read_stdin() -> Result<String, io::Error> {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;
    Ok(buf)
}

fn run_agent(agent: Agent, stdin: &str) -> Result<Option<String>, String> {
    match agent {
        Agent::Claude => map_claude_error(stdin),
        Agent::Codex => map_codex_error(stdin),
    }
}

fn map_claude_error(stdin: &str) -> Result<Option<String>, String> {
    let output = claude::run(stdin).map_err(|err| err.to_string())?;
    Ok(output)
}

fn map_codex_error(stdin: &str) -> Result<Option<String>, String> {
    let output = codex::run(stdin).map_err(|err| err.to_string())?;
    Ok(output)
}

fn io_error(agent: Agent, err: &io::Error) -> PreToolUseError {
    let attributes = base_log_attributes(agent);
    let reason = io_reason(err);
    pre_tool_use_error(attributes, reason)
}

fn io_reason(err: &io::Error) -> String {
    format!("failed to read PreToolUse stdin: {err}")
}

fn pre_tool_use_error(log_attributes: HashMap<String, String>, reason: String) -> PreToolUseError {
    PreToolUseError {
        log_attributes,
        reason,
    }
}

fn base_log_attributes(agent: Agent) -> HashMap<String, String> {
    let agent_name = agent::as_str(agent);
    let key = String::from("agent");
    let value = String::from(agent_name);
    HashMap::from([(key, value)])
}

fn payload_log_attributes(agent: Agent, stdin: &str) -> HashMap<String, String> {
    let mut attributes = base_log_attributes(agent);
    let stdin_bytes = stdin.len().to_string();
    insert_string(&mut attributes, "stdin_bytes", &stdin_bytes);

    let parsed = serde_json::from_str::<Value>(stdin);
    match parsed {
        Ok(value) => {
            insert_string(&mut attributes, "json_valid", "true");
            extend_string_field(&mut attributes, "hook_event_name", &value);
            extend_string_field(&mut attributes, "tool_name", &value);
            let object = value.as_object();
            extend_keys_field(&mut attributes, "top_level_keys", object);
            extend_tool_input_attributes(&mut attributes, &value);
        }
        Err(err) => {
            let error_text = err.to_string();
            insert_string(&mut attributes, "json_valid", "false");
            insert_string(&mut attributes, "json_error", &error_text);
        }
    }

    attributes
}

fn extend_string_field(attributes: &mut HashMap<String, String>, key: &str, value: &Value) {
    let field = value.get(key);
    let Some(field) = field else {
        return;
    };

    let field = field.as_str();
    let Some(field) = field else {
        return;
    };

    insert_string(attributes, key, field);
}

fn extend_keys_field(
    attributes: &mut HashMap<String, String>,
    key: &str,
    object: Option<&serde_json::Map<String, Value>>,
) {
    let Some(object) = object else {
        return;
    };

    let mut keys = object.keys().cloned().collect::<Vec<_>>();
    keys.sort();
    let keys = keys.join(",");
    insert_string(attributes, key, &keys);
}

fn extend_tool_input_attributes(attributes: &mut HashMap<String, String>, value: &Value) {
    let tool_input = value.get("tool_input");
    let Some(tool_input) = tool_input else {
        return;
    };

    let tool_input_kind = value_kind(tool_input);
    insert_string(attributes, "tool_input_kind", tool_input_kind);

    let object = tool_input.as_object();
    extend_keys_field(attributes, "tool_input_keys", object);
}

fn insert_string(attributes: &mut HashMap<String, String>, key: &str, value: &str) {
    let key = key.to_owned();
    let value = value.to_owned();
    attributes.insert(key, value);
}

fn value_kind(value: &Value) -> &'static str {
    match value {
        Value::Array(_) => "array",
        Value::Bool(_) => "bool",
        Value::Null => "null",
        Value::Number(_) => "number",
        Value::Object(_) => "object",
        Value::String(_) => "string",
    }
}
