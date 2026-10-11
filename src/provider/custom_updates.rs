//! Opt-in update checks for user-defined package managers.
//!
//! The configured executable is always spawned directly. Neither update checks nor
//! update actions are interpreted by a shell, so package names remain a single argv
//! value even when they contain shell metacharacters.

use crate::config::{CustomManagerConfig, OutputFormat};
use crate::model::ProviderError;
use serde_json::Value;
use std::process::Command;

/// An available update reported by a custom manager's configured check command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomPackageUpdate {
    pub manager_id: String,
    pub package_name: String,
    pub latest_version: String,
}

/// A direct process invocation for one configured package update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomUpdateCommand {
    pub program: String,
    pub args: Vec<String>,
}

/// Run a custom manager's optional update-check command and parse its output.
///
/// `update_check_cmd` contains argv text, not shell syntax. It may optionally begin
/// with the configured `binary`; that first token is ignored and the configured
/// binary remains the only executable that can be launched.
pub fn check_custom_manager_updates<T: CustomUpdateConfig>(
    manager: &T,
) -> Result<Vec<CustomPackageUpdate>, ProviderError> {
    let (id, binary, command, format) = manager.update_parts();
    let Some(command) = command else {
        return Ok(Vec::new());
    };
    let format = format.ok_or_else(|| {
        ProviderError::ParseError(format!(
            "custom manager '{id}' has update_check_cmd but no update_format",
        ))
    })?;
    let args = command_arguments(command, binary)?;
    let output = Command::new(binary).args(&args).output().map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            ProviderError::NotAvailable(binary.to_owned())
        } else {
            ProviderError::Io(error)
        }
    })?;
    if !output.status.success() {
        return Err(ProviderError::CommandFailed(format!(
            "{} {}",
            binary,
            args.join(" ")
        )));
    }
    let text = String::from_utf8(output.stdout)
        .map_err(|error| ProviderError::ParseError(error.to_string()))?;
    parse_custom_update_output_fields(
        id,
        manager.update_parse_regex_value(),
        manager.update_json_array_path_value(),
        manager.update_json_name_value(),
        manager.update_json_latest_value(),
        format,
        &text,
    )
}

pub trait CustomUpdateConfig {
    fn update_parts(&self) -> (&str, &str, Option<&str>, Option<OutputFormat>);
    fn update_parse_regex_value(&self) -> Option<&str>;
    fn update_json_array_path_value(&self) -> Option<&str>;
    fn update_json_name_value(&self) -> Option<&str>;
    fn update_json_latest_value(&self) -> Option<&str>;
    fn update_command_template(&self) -> Option<&str>;
}

impl CustomUpdateConfig for CustomManagerConfig {
    fn update_parts(&self) -> (&str, &str, Option<&str>, Option<OutputFormat>) {
        (
            &self.id,
            &self.binary,
            self.update_check_cmd.as_deref(),
            self.update_format,
        )
    }
    fn update_parse_regex_value(&self) -> Option<&str> {
        self.update_parse_regex.as_deref()
    }
    fn update_json_array_path_value(&self) -> Option<&str> {
        self.update_json_array_path.as_deref()
    }
    fn update_json_name_value(&self) -> Option<&str> {
        self.update_json_name.as_deref()
    }
    fn update_json_latest_value(&self) -> Option<&str> {
        self.update_json_latest_version.as_deref()
    }
    fn update_command_template(&self) -> Option<&str> {
        self.update_cmd.as_deref()
    }
}

impl CustomUpdateConfig for crate::provider::custom::CustomManagerConfig {
    fn update_parts(&self) -> (&str, &str, Option<&str>, Option<OutputFormat>) {
        let format = self.update_format.as_ref().map(|format| match format {
            crate::provider::custom::OutputFormat::LineRegex => OutputFormat::LineRegex,
            crate::provider::custom::OutputFormat::Json => OutputFormat::Json,
            crate::provider::custom::OutputFormat::Tsv => OutputFormat::Tsv,
            crate::provider::custom::OutputFormat::Csv => OutputFormat::Csv,
        });
        (
            &self.id,
            &self.binary,
            self.update_check_cmd.as_deref(),
            format,
        )
    }
    fn update_parse_regex_value(&self) -> Option<&str> {
        self.update_parse_regex.as_deref()
    }
    fn update_json_array_path_value(&self) -> Option<&str> {
        self.update_json_array_path.as_deref()
    }
    fn update_json_name_value(&self) -> Option<&str> {
        self.update_json_name.as_deref()
    }
    fn update_json_latest_value(&self) -> Option<&str> {
        self.update_json_latest_version.as_deref()
    }
    fn update_command_template(&self) -> Option<&str> {
        self.update_cmd.as_deref()
    }
}

/// Parse update records from JSON pointers or a regex with named captures.
pub fn parse_custom_update_output(
    manager: &CustomManagerConfig,
    format: OutputFormat,
    input: &str,
) -> Result<Vec<CustomPackageUpdate>, ProviderError> {
    parse_custom_update_output_fields(
        &manager.id,
        manager.update_parse_regex.as_deref(),
        manager.update_json_array_path.as_deref(),
        manager.update_json_name.as_deref(),
        manager.update_json_latest_version.as_deref(),
        format,
        input,
    )
}

fn parse_custom_update_output_fields(
    id: &str,
    regex: Option<&str>,
    array_path: Option<&str>,
    name_pointer: Option<&str>,
    latest_pointer: Option<&str>,
    format: OutputFormat,
    input: &str,
) -> Result<Vec<CustomPackageUpdate>, ProviderError> {
    match format {
        OutputFormat::Json => {
            parse_json_updates(id, array_path, name_pointer, latest_pointer, input)
        }
        OutputFormat::LineRegex => parse_regex_updates(id, regex, input),
        OutputFormat::Tsv | OutputFormat::Csv => Err(ProviderError::ParseError(
            "custom manager updates support only json or line_regex formats".into(),
        )),
    }
}

/// Build a direct argv invocation for updating one package, if configured.
/// `{name}`, `{version}`, and `{latest_version}` are substituted inside tokens;
/// no shell expansion, pipes, redirects, or command substitution is performed.
pub fn build_custom_update_command<T: CustomUpdateConfig>(
    manager: &T,
    package_name: &str,
    installed_version: &str,
    latest_version: &str,
) -> Result<Option<CustomUpdateCommand>, ProviderError> {
    let Some(template) = manager.update_command_template() else {
        return Ok(None);
    };
    let (_, binary, _, _) = manager.update_parts();
    let args = command_arguments(template, binary)?;
    let args = args
        .into_iter()
        .map(|arg| {
            arg.replace("{name}", package_name)
                .replace("{version}", installed_version)
                .replace("{latest_version}", latest_version)
        })
        .collect();
    Ok(Some(CustomUpdateCommand {
        program: binary.to_owned(),
        args,
    }))
}

fn parse_json_updates(
    id: &str,
    array_path: Option<&str>,
    name_pointer: Option<&str>,
    latest_pointer: Option<&str>,
    input: &str,
) -> Result<Vec<CustomPackageUpdate>, ProviderError> {
    let value: Value = serde_json::from_str(input)?;
    let name_pointer = name_pointer.ok_or_else(|| {
        ProviderError::ParseError("update_json_name is required for JSON update output".into())
    })?;
    let latest_pointer = latest_pointer.ok_or_else(|| {
        ProviderError::ParseError(
            "update_json_latest_version is required for JSON update output".into(),
        )
    })?;
    let records = if let Some(pointer) = array_path {
        value.pointer(pointer).ok_or_else(|| {
            ProviderError::ParseError(format!("JSON update array pointer '{pointer}' not found"))
        })?
    } else {
        &value
    };
    let records = records.as_array().ok_or_else(|| {
        ProviderError::ParseError("JSON update output must resolve to an array".into())
    })?;

    Ok(records
        .iter()
        .filter_map(|record| {
            let package_name = pointer_string(record, name_pointer)?;
            let latest_version = pointer_string(record, latest_pointer)?;
            Some(CustomPackageUpdate {
                manager_id: id.to_owned(),
                package_name,
                latest_version,
            })
        })
        .collect())
}

fn parse_regex_updates(
    id: &str,
    pattern: Option<&str>,
    input: &str,
) -> Result<Vec<CustomPackageUpdate>, ProviderError> {
    let pattern = pattern.ok_or_else(|| {
        ProviderError::ParseError(
            "update_parse_regex is required for line_regex update output".into(),
        )
    })?;
    let regex =
        regex::Regex::new(pattern).map_err(|error| ProviderError::ParseError(error.to_string()))?;
    if !regex.capture_names().any(|name| name == Some("name"))
        || !regex
            .capture_names()
            .any(|name| name == Some("latest_version"))
    {
        return Err(ProviderError::ParseError(
            "update_parse_regex must define named captures 'name' and 'latest_version'".into(),
        ));
    }
    Ok(input
        .lines()
        .filter_map(|line| {
            let captures = regex.captures(line)?;
            Some(CustomPackageUpdate {
                manager_id: id.to_owned(),
                package_name: captures.name("name")?.as_str().to_owned(),
                latest_version: captures.name("latest_version")?.as_str().to_owned(),
            })
        })
        .collect())
}

fn pointer_string(value: &Value, pointer: &str) -> Option<String> {
    value.pointer(pointer).and_then(|value| match value {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    })
}

/// Tokenize argv text with basic quote and backslash support, never shell syntax.
fn command_arguments(command: &str, configured_binary: &str) -> Result<Vec<String>, ProviderError> {
    let tokens = tokenize_argv(command)?;
    let mut args = tokens;
    if args.first().is_some_and(|first| first == configured_binary) {
        args.remove(0);
    }
    Ok(args)
}

fn tokenize_argv(command: &str) -> Result<Vec<String>, ProviderError> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;
    let mut started = false;

    for character in command.chars() {
        if escaped {
            current.push(character);
            escaped = false;
            started = true;
            continue;
        }
        match (quote, character) {
            (Some(q), c) if q == c => quote = None,
            (Some(_), '\\') => escaped = true,
            (Some(_), c) => current.push(c),
            (None, '\'' | '"') => {
                quote = Some(character);
                started = true;
            }
            (None, '\\') => {
                escaped = true;
                started = true;
            }
            (None, c) if c.is_whitespace() => {
                if started {
                    tokens.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            (None, c) => {
                current.push(c);
                started = true;
            }
        }
    }
    if escaped || quote.is_some() {
        return Err(ProviderError::ParseError(
            "invalid quoting in custom manager argv template".into(),
        ));
    }
    if started {
        tokens.push(current);
    }
    if tokens.is_empty() {
        return Err(ProviderError::ParseError(
            "custom manager argv template cannot be empty".into(),
        ));
    }
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manager() -> CustomManagerConfig {
        let mut manager = crate::config::parse_managers_toml(
            r#"
            [[manager]]
            id = "example"
            name = "Example"
            badge = "[EX]"
            binary = "example-pm"
            list_cmd = "example-pm list"
            format = "json"
            uninstall_cmd = "example-pm remove {name}"
            update_check_cmd = "outdated --json"
            update_format = "json"
            update_json_array_path = "/updates"
            update_json_name = "/name"
            update_json_latest_version = "/latest"
            update_cmd = "update {name} --to {latest_version}"
            "#,
        )
        .unwrap()
        .remove(0);
        manager.id = "example".into();
        manager
    }

    #[test]
    fn parses_json_update_records_using_pointers() {
        let manager = manager();
        let updates = parse_custom_update_output(
            &manager,
            OutputFormat::Json,
            r#"{"updates":[{"name":"alpha","latest":"2.4"}]}"#,
        )
        .unwrap();
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].package_name, "alpha");
        assert_eq!(updates[0].latest_version, "2.4");
        assert_eq!(updates[0].manager_id, "example");
    }

    #[test]
    fn parses_regex_update_records_with_named_captures() {
        let mut manager = manager();
        manager.update_parse_regex = Some(r"^(?P<name>\S+)\s+(?P<latest_version>\S+)$".into());
        let updates =
            parse_custom_update_output(&manager, OutputFormat::LineRegex, "alpha 2.4\nbeta 5.0\n")
                .unwrap();
        assert_eq!(updates.len(), 2);
        assert_eq!(updates[1].package_name, "beta");
        assert_eq!(updates[1].latest_version, "5.0");
    }

    #[test]
    fn update_command_substitutes_values_without_shell_expansion() {
        let manager = manager();
        let command = build_custom_update_command(&manager, "pkg; touch /tmp/pwned", "1.0", "2.0")
            .unwrap()
            .unwrap();
        assert_eq!(command.program, "example-pm");
        assert_eq!(command.args[1], "pkg; touch /tmp/pwned");
        assert_eq!(command.args[3], "2.0");
    }

    #[test]
    fn check_is_opt_in_when_no_command_is_configured() {
        let mut manager = manager();
        manager.update_check_cmd = None;
        assert!(check_custom_manager_updates(&manager).unwrap().is_empty());
    }
}
