use super::*;
use crate::model::PackageSource;
use serde::Deserialize;
use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Deserialize)]
pub struct CustomManagerConfig {
    pub id: String,
    pub name: String,
    pub badge: String,
    pub color: Option<String>,
    pub binary: String,
    pub list_cmd: String,
    pub format: OutputFormat,
    pub parse_regex: Option<String>,
    pub json_array_path: Option<String>,
    pub json_map_path: Option<String>,
    pub json_name: Option<String>,
    pub json_version: Option<String>,
    pub json_size: Option<String>,
    pub json_desc: Option<String>,
    pub size_cmd: Option<String>,
    pub info_cmd: Option<String>,
    pub uninstall_cmd: String,
    #[serde(default)]
    pub need_sudo: bool,
    pub update_check_cmd: Option<String>,
    pub update_format: Option<OutputFormat>,
    pub update_parse_regex: Option<String>,
    pub update_json_array_path: Option<String>,
    pub update_json_name: Option<String>,
    pub update_json_latest_version: Option<String>,
    pub update_cmd: Option<String>,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    LineRegex,
    Json,
    Tsv,
    Csv,
}
#[derive(Debug, Deserialize)]
struct ManagerFile {
    #[serde(default)]
    manager: Vec<CustomManagerConfig>,
}
#[derive(Debug, Clone)]
pub struct CustomManager {
    pub config: CustomManagerConfig,
}
impl CustomManager {
    pub fn load(path: impl Into<PathBuf>) -> Result<Vec<Self>, ProviderError> {
        let path = path.into();
        let content = std::fs::read_to_string(path)?;
        let file: ManagerFile =
            toml::from_str(&content).map_err(|e| ProviderError::ParseError(e.to_string()))?;
        Ok(file
            .manager
            .into_iter()
            .map(|config| Self { config })
            .collect())
    }
    pub fn from_configs(configs: Vec<crate::config::CustomManagerConfig>) -> Vec<Self> {
        configs
            .into_iter()
            .map(|c| Self {
                config: CustomManagerConfig {
                    id: c.id,
                    name: c.name,
                    badge: c.badge,
                    color: c.color,
                    binary: c.binary,
                    list_cmd: c.list_cmd,
                    format: match c.format {
                        crate::config::OutputFormat::LineRegex => OutputFormat::LineRegex,
                        crate::config::OutputFormat::Json => OutputFormat::Json,
                        crate::config::OutputFormat::Tsv => OutputFormat::Tsv,
                        crate::config::OutputFormat::Csv => OutputFormat::Csv,
                    },
                    parse_regex: c.parse_regex,
                    json_array_path: c.json_array_path,
                    json_map_path: c.json_map_path,
                    json_name: c.json_name,
                    json_version: c.json_version,
                    json_size: c.json_size,
                    json_desc: c.json_desc,
                    size_cmd: c.size_cmd,
                    info_cmd: c.info_cmd,
                    uninstall_cmd: c.uninstall_cmd,
                    need_sudo: c.need_sudo,
                    update_check_cmd: c.update_check_cmd,
                    update_format: c.update_format.map(|format| match format {
                        crate::config::OutputFormat::LineRegex => OutputFormat::LineRegex,
                        crate::config::OutputFormat::Json => OutputFormat::Json,
                        crate::config::OutputFormat::Tsv => OutputFormat::Tsv,
                        crate::config::OutputFormat::Csv => OutputFormat::Csv,
                    }),
                    update_parse_regex: c.update_parse_regex,
                    update_json_array_path: c.update_json_array_path,
                    update_json_name: c.update_json_name,
                    update_json_latest_version: c.update_json_latest_version,
                    update_cmd: c.update_cmd,
                },
            })
            .collect()
    }
    pub fn parse(&self, input: &str) -> Result<Vec<UnifiedPackage>, ProviderError> {
        parse_output(&self.config, input)
    }
}
impl PackageManagerProvider for CustomManager {
    fn id(&self) -> &str {
        &self.config.id
    }
    fn is_available(&self) -> bool {
        available(&self.config.binary)
    }
    fn list_packages(&self) -> Result<Vec<UnifiedPackage>, ProviderError> {
        if !self.is_available() {
            return Err(ProviderError::NotAvailable(self.config.binary.clone()));
        }
        // Commands are tokenized without a shell; quoted argument support is intentionally simple.
        let args = self
            .config
            .list_cmd
            .split_whitespace()
            .skip(1)
            .collect::<Vec<_>>();
        let output = run(&self.config.binary, &args)?;
        let listing = text(output)?;
        let paths = captured_paths(&self.config, &listing);
        let mut packages = self.parse(&listing)?;
        for package in &mut packages {
            package.manager_id = Some(self.config.id.clone());
            let path = paths.get(&package.name).map(String::as_str);
            if let Some(command) = self.config.size_cmd.as_deref() {
                if let Some(size) = run_metadata_command(command, package, path)
                    .and_then(|output| parse_command_size(&output))
                {
                    package.size_bytes = Some(size);
                }
            }
            if let Some(command) = self.config.info_cmd.as_deref()
                && let Some(output) = run_metadata_command(command, package, path)
            {
                apply_info_output(package, &output);
            }
        }
        Ok(packages)
    }
}
fn pointer<'a>(v: &'a Value, p: Option<&String>) -> Option<&'a Value> {
    p.and_then(|p| v.pointer(p))
}
fn val_string(v: Option<&Value>) -> Option<String> {
    v.and_then(|x| match x {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    })
}

fn val_size_bytes(v: Option<&Value>) -> Option<u64> {
    let value = v?;
    match value {
        Value::Number(number) => number.as_u64(),
        Value::String(text) => parse_size_bytes(text),
        _ => None,
    }
}

/// Parse byte counts and common human-readable sizes (for example, `1.5 MiB`).
fn parse_size_bytes(value: &str) -> Option<u64> {
    let value = value.trim().replace(',', "");
    let split = value
        .find(|character: char| !character.is_ascii_digit() && character != '.')
        .unwrap_or(value.len());
    let (amount, unit) = value.split_at(split);
    let amount = amount.parse::<f64>().ok()?;
    if !amount.is_finite() || amount < 0.0 {
        return None;
    }
    let normalized_unit = unit.trim().replace(' ', "").to_ascii_lowercase();
    let multiplier = match normalized_unit.as_str() {
        "" | "b" | "byte" | "bytes" => 1_f64,
        "k" | "kb" => 1_000_f64,
        "ki" | "kib" => 1_024_f64,
        "m" | "mb" => 1_000_f64.powi(2),
        "mi" | "mib" => 1_024_f64.powi(2),
        "g" | "gb" => 1_000_f64.powi(3),
        "gi" | "gib" => 1_024_f64.powi(3),
        "t" | "tb" => 1_000_f64.powi(4),
        "ti" | "tib" => 1_024_f64.powi(4),
        _ => return None,
    };
    let bytes = amount * multiplier;
    (bytes <= u64::MAX as f64).then_some(bytes.round() as u64)
}

/// Run a configured metadata command without a shell. Placeholders are expanded
/// inside individual argv tokens, so package names cannot inject shell syntax.
fn run_metadata_command(
    command: &str,
    package: &UnifiedPackage,
    path: Option<&str>,
) -> Option<String> {
    if command.contains("{path}") && path.is_none() {
        return None;
    }
    let args = tokenize_command(command)?;
    let (binary, arguments) = args.split_first()?;
    let arguments = arguments
        .iter()
        .map(|argument| {
            argument
                .replace("{name}", &package.name)
                .replace("{version}", &package.version)
                .replace("{path}", path.unwrap_or(""))
        })
        .collect::<Vec<_>>();
    let output = Command::new(binary).args(arguments).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

fn captured_paths(
    config: &CustomManagerConfig,
    input: &str,
) -> std::collections::HashMap<String, String> {
    if !matches!(config.format, OutputFormat::LineRegex) {
        return std::collections::HashMap::new();
    }
    let Some(pattern) = config.parse_regex.as_deref() else {
        return std::collections::HashMap::new();
    };
    let Ok(regex) = regex::Regex::new(pattern) else {
        return std::collections::HashMap::new();
    };
    input
        .lines()
        .filter_map(|line| {
            let captures = regex.captures(line)?;
            Some((
                captures.name("name")?.as_str().to_owned(),
                captures.name("path")?.as_str().to_owned(),
            ))
        })
        .collect()
}

/// Small argv tokenizer supporting single/double quoted arguments and backslash
/// escapes. It deliberately does not implement shell expansion, pipes or redirects.
fn tokenize_command(command: &str) -> Option<Vec<String>> {
    let mut result = Vec::new();
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
            (Some('\''), '\'') | (Some('"'), '"') => quote = None,
            (Some(_), '\\') => escaped = true,
            (Some(_), _) => current.push(character),
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
                    result.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            (None, _) => {
                current.push(character);
                started = true;
            }
        }
    }
    if escaped || quote.is_some() {
        return None;
    }
    if started {
        result.push(current);
    }
    Some(result)
}

fn parse_command_size(output: &str) -> Option<u64> {
    // `du -sb` emits `<bytes>\t<path>`; also accept a bare human-readable size.
    let first_column = output.trim().split('\t').next()?.trim();
    parse_size_bytes(first_column)
}

fn apply_info_output(package: &mut UnifiedPackage, output: &str) {
    if let Ok(value) = serde_json::from_str::<Value>(output) {
        let object = value.as_object().or_else(|| {
            value
                .as_array()
                .and_then(|array| array.first())
                .and_then(Value::as_object)
        });
        if let Some(object) = object {
            let get = |keys: &[&str]| {
                keys.iter()
                    .find_map(|key| object.get(*key).and_then(|value| val_string(Some(value))))
            };
            if package.description.is_none() {
                package.description = get(&["description", "desc", "summary"]);
            }
            package.url = package.url.take().or_else(|| get(&["url", "homepage"]));
            package.license = package.license.take().or_else(|| get(&["license"]));
            if package.size_bytes.is_none() {
                package.size_bytes =
                    ["size_bytes", "size", "installed_size"]
                        .iter()
                        .find_map(|key| {
                            object
                                .get(*key)
                                .and_then(|value| val_size_bytes(Some(value)))
                        });
            }
            if package.dependencies.is_empty() {
                package.dependencies = object
                    .get("dependencies")
                    .map(|dependencies| match dependencies {
                        Value::Array(array) => array
                            .iter()
                            .filter_map(|value| val_string(Some(value)))
                            .collect::<Vec<_>>(),
                        Value::Object(map) => map.keys().cloned().collect(),
                        _ => Vec::new(),
                    })
                    .unwrap_or_default();
            }
            return;
        }
    }

    // Common human-readable `key: value` output, including uv's tool show format.
    for line in output.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key
            .trim()
            .to_ascii_lowercase()
            .chars()
            .filter(|character| !matches!(character, ' ' | '_' | '-'))
            .collect::<String>();
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        match key.as_str() {
            "description" | "summary" if package.description.is_none() => {
                package.description = Some(value.to_owned());
            }
            "url" | "homepage" if package.url.is_none() => package.url = Some(value.to_owned()),
            "license" if package.license.is_none() => package.license = Some(value.to_owned()),
            "size" | "sizebytes" | "installedsize" if package.size_bytes.is_none() => {
                package.size_bytes = parse_size_bytes(value);
            }
            "dependencies" if package.dependencies.is_empty() => {
                package.dependencies = value
                    .split([',', ' '])
                    .map(str::trim)
                    .filter(|dependency| !dependency.is_empty())
                    .map(str::to_owned)
                    .collect();
            }
            _ => {}
        }
    }
}
pub fn parse_output(
    c: &CustomManagerConfig,
    input: &str,
) -> Result<Vec<UnifiedPackage>, ProviderError> {
    let source = c.id.parse::<PackageSource>().unwrap_or(PackageSource::Pip);
    match c.format {
        OutputFormat::Json => {
            if input.trim().is_empty() {
                return Ok(Vec::new());
            }
            let root: Value = serde_json::from_str(input)?;
            let collection = pointer(&root, c.json_array_path.as_ref())
                .or_else(|| pointer(&root, c.json_map_path.as_ref()))
                .unwrap_or(&root);
            let rows: Vec<&Value> = match collection {
                Value::Array(a) => a.iter().collect(),
                Value::Object(m) => m.values().collect(),
                _ => {
                    return Err(ProviderError::ParseError(
                        "JSON collection path is not an array or object".into(),
                    ));
                }
            };
            Ok(rows
                .into_iter()
                .filter_map(|row| {
                    let name = val_string(pointer(row, c.json_name.as_ref()))?;
                    let version = val_string(pointer(row, c.json_version.as_ref()))
                        .unwrap_or_else(|| "unknown".into());
                    let mut p = UnifiedPackage::new(name, version, source);
                    p.size_bytes = val_size_bytes(pointer(row, c.json_size.as_ref()));
                    p.description = val_string(pointer(row, c.json_desc.as_ref()));
                    Some(p)
                })
                .collect())
        }
        OutputFormat::LineRegex => {
            let pattern = c
                .parse_regex
                .as_deref()
                .ok_or_else(|| ProviderError::ParseError("parse_regex is required".into()))?;
            let re =
                regex::Regex::new(pattern).map_err(|e| ProviderError::ParseError(e.to_string()))?;
            Ok(input
                .lines()
                .filter_map(|line| {
                    let caps = re.captures(line)?;
                    let name = caps.name("name")?.as_str();
                    let version = caps
                        .name("version")
                        .map(|x| x.as_str())
                        .unwrap_or("unknown");
                    let mut p = UnifiedPackage::new(name, version, source);
                    p.description = caps.name("desc").map(|x| x.as_str().to_string());
                    Some(p)
                })
                .collect())
        }
        OutputFormat::Tsv | OutputFormat::Csv => {
            let delim = if matches!(c.format, OutputFormat::Tsv) {
                '\t'
            } else {
                ','
            };
            let mut rows = input.lines();
            let headers = rows.next().unwrap_or("").split(delim).collect::<Vec<_>>();
            let idx = |key: &str| headers.iter().position(|h| *h == key);
            let ni = idx("name").unwrap_or(0);
            let vi = idx("version");
            let di = idx("desc");
            Ok(rows
                .filter_map(|line| {
                    let f = line.split(delim).collect::<Vec<_>>();
                    let name = f.get(ni)?.trim();
                    if name.is_empty() {
                        return None;
                    }
                    let mut p = UnifiedPackage::new(
                        name,
                        vi.and_then(|i| f.get(i)).copied().unwrap_or("unknown"),
                        source,
                    );
                    p.description = di.and_then(|i| f.get(i)).map(|s| s.to_string());
                    Some(p)
                })
                .collect())
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn config(format: OutputFormat) -> CustomManagerConfig {
        CustomManagerConfig {
            id: "npm".into(),
            name: "NPM".into(),
            badge: "[NPM]".into(),
            color: None,
            binary: "npm".into(),
            list_cmd: "npm list".into(),
            format,
            parse_regex: None,
            json_array_path: None,
            json_map_path: None,
            json_name: Some("/name".into()),
            json_version: Some("/version".into()),
            json_size: None,
            json_desc: Some("/desc".into()),
            size_cmd: None,
            info_cmd: None,
            uninstall_cmd: "npm uninstall {name}".into(),
            need_sudo: false,
            update_check_cmd: None,
            update_format: None,
            update_parse_regex: None,
            update_json_array_path: None,
            update_json_name: None,
            update_json_latest_version: None,
            update_cmd: None,
        }
    }
    #[test]
    fn parses_regex() {
        let mut c = config(OutputFormat::LineRegex);
        c.parse_regex = Some(r"^(?P<name>\w+) v(?P<version>[\d.]+)$".into());
        assert_eq!(parse_output(&c, "ruff v1.2").unwrap()[0].version, "1.2");
    }
    #[test]
    fn parses_json() {
        let c = config(OutputFormat::Json);
        let p = parse_output(&c, r#"[{"name":"pkg","version":"1","desc":"tool"}]"#).unwrap();
        assert_eq!(p[0].description.as_deref(), Some("tool"));
    }
    #[test]
    fn parses_json_size_as_bytes_or_human_readable_value() {
        let mut c = config(OutputFormat::Json);
        c.json_size = Some("/size".into());
        let p = parse_output(
            &c,
            r#"[{"name":"bytes","version":"1","size":2048},{"name":"human","version":"1","size":"1.5 MiB"}]"#,
        )
        .unwrap();
        assert_eq!(p[0].size_bytes, Some(2048));
        assert_eq!(p[1].size_bytes, Some(1_572_864));
    }
    #[test]
    fn parses_size_command_output_and_rejects_unknown_units() {
        assert_eq!(parse_command_size("12345\t/tmp/pkg"), Some(12_345));
        assert_eq!(parse_command_size("1.5 GiB\n"), Some(1_610_612_736));
        assert_eq!(parse_command_size("1.5 blocks"), None);
    }
    #[test]
    fn parses_info_json_and_key_value_details() {
        let mut package = UnifiedPackage::new("pkg", "1", PackageSource::Pip);
        apply_info_output(
            &mut package,
            r#"{"description":"A package","license":"MIT","dependencies":["dep>=1"]}"#,
        );
        assert_eq!(package.description.as_deref(), Some("A package"));
        assert_eq!(package.license.as_deref(), Some("MIT"));
        assert_eq!(package.dependencies, vec!["dep>=1"]);

        let mut package = UnifiedPackage::new("pkg", "1", PackageSource::Pip);
        apply_info_output(
            &mut package,
            "Summary: Human summary\nInstalled-Size: 2 MiB\nURL: https://example.test",
        );
        assert_eq!(package.description.as_deref(), Some("Human summary"));
        assert_eq!(package.size_bytes, Some(2_097_152));
        assert_eq!(package.url.as_deref(), Some("https://example.test"));
    }
    #[test]
    fn metadata_command_tokenizer_keeps_placeholders_inside_argv() {
        assert_eq!(
            tokenize_command("du -sb '{path}' --label=\"{name}\"").unwrap(),
            vec!["du", "-sb", "{path}", "--label={name}"]
        );
        assert!(tokenize_command("du '{path}").is_none());
    }
    #[test]
    fn parses_delimited() {
        let c = config(OutputFormat::Tsv);
        let p = parse_output(&c, "name\tversion\npkg\t2").unwrap();
        assert_eq!(p[0].version, "2");
    }
    #[test]
    fn loads_manager_file() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("managers.toml");
        std::fs::write(&p,"[[manager]]\nid='x'\nname='X'\nbadge='[X]'\nbinary='x'\nlist_cmd='x list'\nformat='line_regex'\nparse_regex='(?P<name>x)'\nuninstall_cmd='x rm {name}'\n").unwrap();
        assert_eq!(CustomManager::load(p).unwrap().len(), 1);
    }
}
