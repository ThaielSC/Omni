use super::*;
use crate::model::PackageSource;
use serde::Deserialize;
use serde_json::Value;
use std::path::PathBuf;

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
        let mut packages = self.parse(&text(output)?)?;
        for package in &mut packages {
            package.manager_id = Some(self.config.id.clone());
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
