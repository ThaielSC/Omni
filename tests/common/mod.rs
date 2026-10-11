//! Common testing infrastructure for Omni opaque-box E2E CLI tests.
//! Self-contained, zero external runtime/dev dependency requirement.
#![allow(dead_code)]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Locate the compiled `omni` binary.
pub fn omni_bin_path() -> PathBuf {
    // CARGO_BIN_EXE_omni is set by Cargo at compile time for integration tests.
    PathBuf::from(env!("CARGO_BIN_EXE_omni"))
}

/// Path to the `tests/fixtures` directory.
pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

/// Path to a specific fixture file.
pub fn fixture_path(relative_path: &str) -> PathBuf {
    fixtures_dir().join(relative_path)
}

/// Read fixture content as String.
pub fn read_fixture(relative_path: &str) -> String {
    let p = fixture_path(relative_path);
    fs::read_to_string(&p).unwrap_or_else(|e| panic!("Failed to read fixture {:?}: {}", p, e))
}

/// Result of executing the Omni CLI.
#[derive(Debug, Clone)]
pub struct OmniOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

impl OmniOutput {
    pub fn from_output(output: Output) -> Self {
        let exit_code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Self {
            exit_code,
            stdout,
            stderr,
            success: output.status.success(),
        }
    }

    /// Parse stdout as JSON Value.
    pub fn parse_json(&self) -> Result<JsonValue, String> {
        let trimmed = self.stdout.trim();
        if trimmed.is_empty() {
            return Err("Stdout is empty, expected JSON".to_string());
        }
        JsonValue::parse(trimmed)
    }

    /// Assert that exit code is 0.
    pub fn assert_success(&self) {
        assert!(
            self.success,
            "Expected success (exit 0), got exit code {}. Stderr: {}",
            self.exit_code, self.stderr
        );
    }

    /// Assert that exit code is non-zero.
    pub fn assert_failure(&self) {
        assert!(
            !self.success,
            "Expected failure (non-zero exit), got exit code 0. Stdout: {}",
            self.stdout
        );
    }
}

/// Command builder for invoking the Omni CLI in tests.
pub struct OmniCmd {
    args: Vec<String>,
    envs: Vec<(String, String)>,
    remove_envs: Vec<String>,
    cwd: Option<PathBuf>,
}

impl OmniCmd {
    pub fn new() -> Self {
        Self {
            args: Vec::new(),
            envs: Vec::new(),
            remove_envs: Vec::new(),
            cwd: None,
        }
    }

    pub fn arg<S: Into<String>>(mut self, a: S) -> Self {
        self.args.push(a.into());
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for a in args {
            self.args.push(a.into());
        }
        self
    }

    pub fn env<K: Into<String>, V: Into<String>>(mut self, key: K, val: V) -> Self {
        self.envs.push((key.into(), val.into()));
        self
    }

    pub fn remove_env<K: Into<String>>(mut self, key: K) -> Self {
        self.remove_envs.push(key.into());
        self
    }

    /// Set an empty PATH environment variable to test absence detection.
    pub fn with_empty_path(self) -> Self {
        self.env("PATH", "")
    }

    /// Set PATH to a dummy directory without package managers.
    pub fn with_dummy_path(self, dummy_dir: &Path) -> Self {
        self.env("PATH", dummy_dir.to_string_lossy().to_string())
    }

    /// Provide a custom config path argument `--config <path>`.
    pub fn with_config_file(self, config_path: &Path) -> Self {
        self.arg("--config")
            .arg(config_path.to_string_lossy().to_string())
    }

    pub fn cwd<P: Into<PathBuf>>(mut self, dir: P) -> Self {
        self.cwd = Some(dir.into());
        self
    }

    /// Run the CLI and capture output.
    pub fn run(self) -> OmniOutput {
        let bin = omni_bin_path();
        let mut cmd = Command::new(&bin);

        for a in &self.args {
            cmd.arg(a);
        }

        for (k, v) in &self.envs {
            cmd.env(k, v);
        }

        for k in &self.remove_envs {
            cmd.env_remove(k);
        }

        if let Some(ref dir) = self.cwd {
            cmd.current_dir(dir);
        }

        match cmd.output() {
            Ok(out) => OmniOutput::from_output(out),
            Err(e) => panic!("Failed to execute omni binary at {:?}: {}", bin, e),
        }
    }
}

/// Helper to create isolated temporary directory sandboxes.
pub struct TempSandbox {
    pub path: PathBuf,
}

impl TempSandbox {
    pub fn create(prefix: &str) -> Self {
        let id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(12345);
        let pid = std::process::id();
        let dir_name = format!("omni_test_{}_{}_{}", prefix, pid, id);
        let path = std::env::temp_dir().join(dir_name);
        fs::create_dir_all(&path).expect("Failed to create temporary sandbox directory");
        Self { path }
    }

    pub fn write_file(&self, filename: &str, content: &str) -> PathBuf {
        let file_path = self.path.join(filename);
        if let Some(parent) = file_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(&file_path, content).expect("Failed to write sandbox file");
        file_path
    }
}

impl Drop for TempSandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

// -----------------------------------------------------------------------------
// Lightweight, pure Standard-Library JSON Parser & Schema Validator
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
}

impl JsonValue {
    pub fn is_null(&self) -> bool {
        matches!(self, JsonValue::Null)
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            JsonValue::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            JsonValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            JsonValue::Number(n) if *n >= 0.0 => Some(*n as u64),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            JsonValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<JsonValue>> {
        match self {
            JsonValue::Array(arr) => Some(arr),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&HashMap<String, JsonValue>> {
        match self {
            JsonValue::Object(map) => Some(map),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        self.as_object().and_then(|map| map.get(key))
    }

    /// Parse a JSON string.
    pub fn parse(input: &str) -> Result<JsonValue, String> {
        let mut chars = input.chars().peekable();
        skip_ws(&mut chars);
        let val = parse_value(&mut chars)?;
        skip_ws(&mut chars);
        if chars.peek().is_some() {
            return Err("Trailing non-whitespace characters after JSON".to_string());
        }
        Ok(val)
    }
}

fn skip_ws<I: Iterator<Item = char>>(chars: &mut std::iter::Peekable<I>) {
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
        } else {
            break;
        }
    }
}

fn parse_value<I: Iterator<Item = char>>(
    chars: &mut std::iter::Peekable<I>,
) -> Result<JsonValue, String> {
    skip_ws(chars);
    match chars.peek() {
        Some(&'n') => parse_null(chars),
        Some(&'t') | Some(&'f') => parse_bool(chars),
        Some(&'"') => parse_string(chars).map(JsonValue::String),
        Some(&'[') => parse_array(chars),
        Some(&'{') => parse_object(chars),
        Some(&c) if c == '-' || c.is_ascii_digit() => parse_number(chars),
        Some(&c) => Err(format!("Unexpected character '{}' at start of value", c)),
        None => Err("Unexpected end of input while parsing value".to_string()),
    }
}

fn parse_null<I: Iterator<Item = char>>(
    chars: &mut std::iter::Peekable<I>,
) -> Result<JsonValue, String> {
    let s: String = chars.take(4).collect();
    if s == "null" {
        Ok(JsonValue::Null)
    } else {
        Err(format!("Expected 'null', got '{}'", s))
    }
}

fn parse_bool<I: Iterator<Item = char>>(
    chars: &mut std::iter::Peekable<I>,
) -> Result<JsonValue, String> {
    if chars.peek() == Some(&'t') {
        let s: String = chars.take(4).collect();
        if s == "true" {
            Ok(JsonValue::Bool(true))
        } else {
            Err(format!("Expected 'true', got '{}'", s))
        }
    } else {
        let s: String = chars.take(5).collect();
        if s == "false" {
            Ok(JsonValue::Bool(false))
        } else {
            Err(format!("Expected 'false', got '{}'", s))
        }
    }
}

fn parse_string<I: Iterator<Item = char>>(
    chars: &mut std::iter::Peekable<I>,
) -> Result<String, String> {
    if chars.next() != Some('"') {
        return Err("Expected opening quote for string".to_string());
    }
    let mut out = String::new();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Ok(out),
            '\\' => {
                let esc = chars.next().ok_or("Unterminated escape in string")?;
                match esc {
                    '"' => out.push('"'),
                    '\\' => out.push('\\'),
                    '/' => out.push('/'),
                    'b' => out.push('\u{0008}'),
                    'f' => out.push('\u{000C}'),
                    'n' => out.push('\n'),
                    'r' => out.push('\r'),
                    't' => out.push('\t'),
                    'u' => {
                        let hex: String = chars.take(4).collect();
                        if hex.len() != 4 {
                            return Err("Incomplete \\u unicode escape".to_string());
                        }
                        let code = u32::from_str_radix(&hex, 16)
                            .map_err(|e| format!("Invalid hex in \\u escape: {}", e))?;
                        if let Some(ch) = char::from_u32(code) {
                            out.push(ch);
                        } else {
                            return Err(format!("Invalid codepoint {}", code));
                        }
                    }
                    other => return Err(format!("Unknown escape sequence \\{}", other)),
                }
            }
            ch => out.push(ch),
        }
    }
    Err("Unclosed string literal".to_string())
}

fn parse_number<I: Iterator<Item = char>>(
    chars: &mut std::iter::Peekable<I>,
) -> Result<JsonValue, String> {
    let mut num_str = String::new();
    while let Some(&c) = chars.peek() {
        if c == '-' || c == '+' || c == '.' || c == 'e' || c == 'E' || c.is_ascii_digit() {
            num_str.push(chars.next().unwrap());
        } else {
            break;
        }
    }
    num_str
        .parse::<f64>()
        .map(JsonValue::Number)
        .map_err(|e| format!("Failed to parse number '{}': {}", num_str, e))
}

fn parse_array<I: Iterator<Item = char>>(
    chars: &mut std::iter::Peekable<I>,
) -> Result<JsonValue, String> {
    chars.next(); // eat '['
    let mut items = Vec::new();
    skip_ws(chars);
    if chars.peek() == Some(&']') {
        chars.next();
        return Ok(JsonValue::Array(items));
    }
    loop {
        let val = parse_value(chars)?;
        items.push(val);
        skip_ws(chars);
        match chars.peek() {
            Some(&',') => {
                chars.next();
                skip_ws(chars);
            }
            Some(&']') => {
                chars.next();
                return Ok(JsonValue::Array(items));
            }
            Some(&c) => return Err(format!("Expected ',' or ']' in array, got '{}'", c)),
            None => return Err("Unterminated array".to_string()),
        }
    }
}

fn parse_object<I: Iterator<Item = char>>(
    chars: &mut std::iter::Peekable<I>,
) -> Result<JsonValue, String> {
    chars.next(); // eat '{'
    let mut map = HashMap::new();
    skip_ws(chars);
    if chars.peek() == Some(&'}') {
        chars.next();
        return Ok(JsonValue::Object(map));
    }
    loop {
        skip_ws(chars);
        let key = parse_string(chars)?;
        skip_ws(chars);
        if chars.next() != Some(':') {
            return Err("Expected ':' after key in object".to_string());
        }
        skip_ws(chars);
        let val = parse_value(chars)?;
        map.insert(key, val);
        skip_ws(chars);
        match chars.peek() {
            Some(&',') => {
                chars.next();
                skip_ws(chars);
            }
            Some(&'}') => {
                chars.next();
                return Ok(JsonValue::Object(map));
            }
            Some(&c) => return Err(format!("Expected ',' or '}}' in object, got '{}'", c)),
            None => return Err("Unterminated object".to_string()),
        }
    }
}

// -----------------------------------------------------------------------------
// UnifiedPackage Schema Validator
// -----------------------------------------------------------------------------

/// Validate that a JSON value matches the `UnifiedPackage` contract from PROJECT.md.
pub fn validate_unified_package_schema(pkg: &JsonValue) -> Result<(), String> {
    let obj = pkg
        .as_object()
        .ok_or("UnifiedPackage must be a JSON object")?;

    // 1. "name": non-empty String
    let name = obj
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or("UnifiedPackage 'name' must be a non-empty string")?;
    if name.trim().is_empty() {
        return Err("UnifiedPackage 'name' cannot be blank".to_string());
    }

    // 2. "version": non-empty String
    let version = obj
        .get("version")
        .and_then(|v| v.as_str())
        .ok_or("UnifiedPackage 'version' must be a non-empty string")?;
    if version.trim().is_empty() {
        return Err("UnifiedPackage 'version' cannot be blank".to_string());
    }

    // 3. "source": valid PackageSource enum name
    let source = obj
        .get("source")
        .and_then(|v| v.as_str())
        .ok_or("UnifiedPackage 'source' must be a string")?;
    let lower_src = source.to_lowercase();
    let valid_sources = [
        "pacman", "aur", "yay", "flatpak", "cargo", "uv", "pip", "python",
    ];
    if !valid_sources.contains(&lower_src.as_str()) {
        return Err(format!(
            "UnifiedPackage 'source' has unknown value: '{}'",
            source
        ));
    }

    // 4. "size_bytes": Option<u64> -> integer number or null
    if let Some(size) = obj.get("size_bytes") {
        if !size.is_null() && size.as_u64().is_none() {
            return Err(
                "UnifiedPackage 'size_bytes' must be a positive integer or null".to_string(),
            );
        }
    }

    // 5. "description": Option<String>
    if let Some(desc) = obj.get("description") {
        if !desc.is_null() && desc.as_str().is_none() {
            return Err("UnifiedPackage 'description' must be a string or null".to_string());
        }
    }

    // 6. "dependencies": Vec<String> (optional field in JSON, but if present must be array of strings)
    if let Some(deps) = obj.get("dependencies") {
        if !deps.is_null() {
            let arr = deps
                .as_array()
                .ok_or("UnifiedPackage 'dependencies' must be an array")?;
            for item in arr {
                if item.as_str().is_none() {
                    return Err("UnifiedPackage 'dependencies' items must be strings".to_string());
                }
            }
        }
    }

    // 7. "files": Vec<String>
    if let Some(files) = obj.get("files") {
        if !files.is_null() {
            let arr = files
                .as_array()
                .ok_or("UnifiedPackage 'files' must be an array")?;
            for item in arr {
                if item.as_str().is_none() {
                    return Err("UnifiedPackage 'files' items must be strings".to_string());
                }
            }
        }
    }

    // 8. "binaries": Vec<String>
    if let Some(bins) = obj.get("binaries") {
        if !bins.is_null() {
            let arr = bins
                .as_array()
                .ok_or("UnifiedPackage 'binaries' must be an array")?;
            for item in arr {
                if item.as_str().is_none() {
                    return Err("UnifiedPackage 'binaries' items must be strings".to_string());
                }
            }
        }
    }

    Ok(())
}
