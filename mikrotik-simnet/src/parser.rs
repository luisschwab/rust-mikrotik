//! Deterministic topology manifest parser.

use core::mem;
use std::collections::BTreeMap;

use crate::error::Error;
use crate::error::Result;
use crate::runner::link_interface_index;
use crate::topology::Check;
use crate::topology::CommandAttribute;
use crate::topology::Endpoint;
use crate::topology::Link;
use crate::topology::Router;
use crate::topology::RouterCommand;
use crate::topology::Topology;

/// Parse the supported deterministic TOML subset into a topology.
pub(crate) fn parse_topology(contents: &str) -> Result<Topology> {
    let mut root = BTreeMap::new();
    let mut routers = Vec::new();
    let mut links = Vec::new();
    let mut checks = Vec::new();
    let mut section = Section::Root;

    for (line_index, line) in logical_lines(contents)? {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        match line {
            "[[routers]]" => {
                routers.push(BTreeMap::new());
                section = Section::Router;
                continue;
            }
            "[[links]]" => {
                links.push(BTreeMap::new());
                section = Section::Link;
                continue;
            }
            "[[checks]]" => {
                checks.push(BTreeMap::new());
                section = Section::Check;
                continue;
            }
            _ if line.starts_with('[') => {
                return Err(Error::Manifest(format!(
                    "line {line_index}: unsupported section `{line}`"
                )));
            }
            _ => {}
        }

        let (key, value) =
            parse_key_value(line).map_err(|message| Error::Manifest(format!("line {line_index}: {message}")))?;
        match section {
            Section::Root => {
                root.insert(key, value);
            }
            Section::Router => {
                routers
                    .last_mut()
                    .expect("router section inserted before assignment")
                    .insert(key, value);
            }
            Section::Link => {
                links
                    .last_mut()
                    .expect("link section inserted before assignment")
                    .insert(key, value);
            }
            Section::Check => {
                checks
                    .last_mut()
                    .expect("check section inserted before assignment")
                    .insert(key, value);
            }
        }
    }

    reject_unknown_keys(&root, &["name", "allow_software_emulation"], "topology")?;
    let topology = Topology {
        name: required_string(&root, "name")?,
        allow_software_emulation: optional_bool(&root, "allow_software_emulation")?.unwrap_or(false),
        routers: routers.iter().map(parse_router).collect::<Result<Vec<_>>>()?,
        links: links.iter().map(parse_link).collect::<Result<Vec<_>>>()?,
        checks: checks.iter().map(parse_check).collect::<Result<Vec<_>>>()?,
    };
    validate_topology(&topology)?;
    Ok(topology)
}

/// Collapse physical manifest lines into logical assignment or section lines.
fn logical_lines(contents: &str) -> Result<Vec<(usize, String)>> {
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut start_line = 0;

    for (line_index, raw_line) in contents.lines().enumerate() {
        let line_number = line_index + 1;
        let line = trim_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }

        if current.is_empty() {
            start_line = line_number;
            current.push_str(line);
        } else {
            current.push(' ');
            current.push_str(line);
        }

        let depth = array_depth(&current)?;
        if depth == 0 {
            lines.push((start_line, mem::take(&mut current)));
        }
    }

    if current.is_empty() {
        Ok(lines)
    } else {
        Err(Error::Manifest(format!("line {start_line}: unterminated array value")))
    }
}

/// Return bracket nesting depth outside quoted strings.
fn array_depth(line: &str) -> Result<usize> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for character in line.chars() {
        if escaped {
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '"' {
            in_string = !in_string;
        } else if character == '[' && !in_string {
            depth = depth
                .checked_add(1)
                .ok_or_else(|| Error::Manifest("array nesting is too deep".to_owned()))?;
        } else if character == ']' && !in_string {
            depth = depth
                .checked_sub(1)
                .ok_or_else(|| Error::Manifest("unexpected `]` outside array".to_owned()))?;
        }
    }

    Ok(depth)
}

/// Current manifest section while parsing logical lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    /// Top-level key-value assignments.
    Root,
    /// `[[routers]]` table-array item.
    Router,
    /// `[[links]]` table-array item.
    Link,
    /// `[[checks]]` table-array item.
    Check,
}

/// Parse one router table.
fn parse_router(values: &BTreeMap<String, Value>) -> Result<Router> {
    reject_unknown_keys(
        values,
        &["name", "version", "memory_mib", "cpus", "bootstrap"],
        "router",
    )?;
    Ok(Router {
        name: required_string(values, "name")?,
        version: required_string(values, "version")?,
        memory_mib: optional_u16(values, "memory_mib")?.unwrap_or(256),
        cpus: optional_u8(values, "cpus")?.unwrap_or(1),
        bootstrap: optional_string_array(values, "bootstrap")?
            .unwrap_or_default()
            .iter()
            .map(|command| parse_router_command(command))
            .collect::<Result<Vec<_>>>()?,
    })
}

/// Parse one link table.
fn parse_link(values: &BTreeMap<String, Value>) -> Result<Link> {
    reject_unknown_keys(values, &["a", "b"], "link")?;
    Ok(Link {
        a: parse_endpoint(&required_string(values, "a")?)?,
        b: parse_endpoint(&required_string(values, "b")?)?,
    })
}

/// Parse one check table.
fn parse_check(values: &BTreeMap<String, Value>) -> Result<Check> {
    reject_unknown_keys(
        values,
        &["type", "router", "allow_unsupported", "command", "min_rows"],
        "check",
    )?;
    match required_string(values, "type")?.as_str() {
        "all-print-methods" => Ok(Check::AllPrintMethods {
            router: required_string(values, "router")?,
            allow_unsupported: optional_bool(values, "allow_unsupported")?.unwrap_or(false),
        }),
        "command-rows" => Ok(Check::CommandRows {
            router: required_string(values, "router")?,
            command: required_string(values, "command")?,
            min_rows: optional_usize(values, "min_rows")?.unwrap_or(1),
        }),
        check_type => Err(Error::Manifest(format!("unsupported check type `{check_type}`"))),
    }
}

/// Parse a `router:interface` endpoint string.
fn parse_endpoint(value: &str) -> Result<Endpoint> {
    let (router, interface) = value
        .split_once(':')
        .ok_or_else(|| Error::Manifest(format!("endpoint `{value}` must use router:interface")))?;
    Ok(Endpoint {
        router: router.to_owned(),
        interface: interface.to_owned(),
    })
}

/// Parse a bootstrap command string into command path and attributes.
fn parse_router_command(value: &str) -> Result<RouterCommand> {
    let mut parts = value.split_whitespace();
    let command = parts
        .next()
        .ok_or_else(|| Error::Manifest("bootstrap command must not be empty".to_owned()))?;
    if !command.starts_with('/') {
        return Err(Error::Manifest(format!(
            "bootstrap command `{command}` must start with `/`"
        )));
    }

    let mut attributes = Vec::new();
    for part in parts {
        let (key, value) = part
            .split_once('=')
            .map_or_else(|| (part, None), |(key, value)| (key, Some(value)));
        if key.is_empty() {
            return Err(Error::Manifest(format!(
                "bootstrap command `{command}` has an empty attribute key"
            )));
        }
        attributes.push(CommandAttribute {
            key: key.to_owned(),
            value: value.map(ToOwned::to_owned),
        });
    }

    Ok(RouterCommand {
        command: command.to_owned(),
        attributes,
    })
}

/// Validate cross-references and deterministic naming assumptions.
fn validate_topology(topology: &Topology) -> Result<()> {
    if topology.routers.is_empty() {
        return Err(Error::Manifest("topology must declare at least one router".to_owned()));
    }
    for router in &topology.routers {
        if router.name.is_empty() {
            return Err(Error::Manifest("router name must not be empty".to_owned()));
        }
        if router.version.is_empty() {
            return Err(Error::Manifest(format!(
                "router {} version must not be empty",
                router.name
            )));
        }
    }
    for link in &topology.links {
        topology.router(&link.a.router)?;
        topology.router(&link.b.router)?;
        link_interface_index(&link.a.router, link)?;
        link_interface_index(&link.b.router, link)?;
    }
    for check in &topology.checks {
        match check {
            Check::AllPrintMethods { router, .. } | Check::CommandRows { router, .. } => {
                topology.router(router)?;
            }
        }
    }
    Ok(())
}

/// Reject unsupported manifest keys before interpreting a table.
fn reject_unknown_keys(values: &BTreeMap<String, Value>, allowed: &[&str], section: &str) -> Result<()> {
    for key in values.keys() {
        if !allowed.iter().any(|allowed_key| allowed_key == key) {
            return Err(Error::Manifest(format!("unsupported {section} key `{key}`")));
        }
    }
    Ok(())
}

/// Parse one `key = value` logical line.
fn parse_key_value(line: &str) -> core::result::Result<(String, Value), String> {
    let (key, value) = line.split_once('=').ok_or_else(|| "expected key = value".to_owned())?;
    Ok((key.trim().to_owned(), parse_value(value.trim())?))
}

/// Parse a supported TOML scalar or string-array value.
fn parse_value(value: &str) -> core::result::Result<Value, String> {
    if let Some(value) = parse_string(value) {
        return Ok(Value::String(value));
    }
    if value == "true" {
        return Ok(Value::Bool(true));
    }
    if value == "false" {
        return Ok(Value::Bool(false));
    }
    if value.starts_with('[') && value.ends_with(']') {
        return parse_array(&value[1..value.len() - 1]).map(Value::StringArray);
    }
    value
        .parse::<u64>()
        .map(Value::Integer)
        .map_err(|_| format!("unsupported TOML value `{value}`"))
}

/// Parse a comma-separated string array body.
fn parse_array(value: &str) -> core::result::Result<Vec<String>, String> {
    let mut strings = Vec::new();
    let mut rest = value.trim();
    while !rest.is_empty() {
        let Some(item) = parse_string_prefix(rest) else {
            return Err(format!("array item must be a quoted string: `{rest}`"));
        };
        strings.push(item.0);
        rest = item.1.trim_start();
        if let Some(next) = rest.strip_prefix(',') {
            rest = next.trim_start();
        } else if !rest.is_empty() {
            return Err(format!("expected comma in array near `{rest}`"));
        }
    }
    Ok(strings)
}

/// Parse an entire quoted string.
fn parse_string(value: &str) -> Option<String> {
    parse_string_prefix(value).and_then(|(string, rest)| rest.trim().is_empty().then_some(string))
}

/// Parse a quoted string prefix and return the unconsumed suffix.
fn parse_string_prefix(value: &str) -> Option<(String, &str)> {
    let value = value.strip_prefix('"')?;
    let mut output = String::new();
    let mut escaped = false;
    for (index, character) in value.char_indices() {
        if escaped {
            output.push(match character {
                '"' => '"',
                '\\' => '\\',
                'n' => '\n',
                't' => '\t',
                other => other,
            });
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '"' {
            return Some((output, &value[index + 1..]));
        } else {
            output.push(character);
        }
    }
    None
}

/// Trim a `#` comment that appears outside a quoted string.
fn trim_comment(line: &str) -> &str {
    let mut in_string = false;
    let mut escaped = false;
    for (index, character) in line.char_indices() {
        if escaped {
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '"' {
            in_string = !in_string;
        } else if character == '#' && !in_string {
            return &line[..index];
        }
    }
    line
}

/// Read a required string value from a parsed table.
fn required_string(values: &BTreeMap<String, Value>, key: &str) -> Result<String> {
    match values.get(key) {
        Some(Value::String(value)) => Ok(value.clone()),
        Some(_) => Err(Error::Manifest(format!("`{key}` must be a string"))),
        None => Err(Error::Manifest(format!("missing required `{key}`"))),
    }
}

/// Read an optional string-array value from a parsed table.
fn optional_string_array(values: &BTreeMap<String, Value>, key: &str) -> Result<Option<Vec<String>>> {
    match values.get(key) {
        Some(Value::StringArray(value)) => Ok(Some(value.clone())),
        Some(_) => Err(Error::Manifest(format!("`{key}` must be an array of strings"))),
        None => Ok(None),
    }
}

/// Read an optional boolean value from a parsed table.
fn optional_bool(values: &BTreeMap<String, Value>, key: &str) -> Result<Option<bool>> {
    match values.get(key) {
        Some(Value::Bool(value)) => Ok(Some(*value)),
        Some(_) => Err(Error::Manifest(format!("`{key}` must be a boolean"))),
        None => Ok(None),
    }
}

/// Read an optional `u16` value from a parsed table.
fn optional_u16(values: &BTreeMap<String, Value>, key: &str) -> Result<Option<u16>> {
    optional_integer(values, key)?
        .map(|value| u16::try_from(value).map_err(|_| Error::Manifest(format!("`{key}` is outside u16 range"))))
        .transpose()
}

/// Read an optional `u8` value from a parsed table.
fn optional_u8(values: &BTreeMap<String, Value>, key: &str) -> Result<Option<u8>> {
    optional_integer(values, key)?
        .map(|value| u8::try_from(value).map_err(|_| Error::Manifest(format!("`{key}` is outside u8 range"))))
        .transpose()
}

/// Read an optional `usize` value from a parsed table.
fn optional_usize(values: &BTreeMap<String, Value>, key: &str) -> Result<Option<usize>> {
    optional_integer(values, key)?
        .map(|value| usize::try_from(value).map_err(|_| Error::Manifest(format!("`{key}` is outside usize range"))))
        .transpose()
}

/// Read an optional unsigned integer value from a parsed table.
fn optional_integer(values: &BTreeMap<String, Value>, key: &str) -> Result<Option<u64>> {
    match values.get(key) {
        Some(Value::Integer(value)) => Ok(Some(*value)),
        Some(_) => Err(Error::Manifest(format!("`{key}` must be an integer"))),
        None => Ok(None),
    }
}

/// TOML values supported by the simnet manifest parser.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Value {
    /// Quoted string.
    String(String),
    /// Non-negative integer.
    Integer(u64),
    /// Boolean literal.
    Bool(bool),
    /// Array containing only quoted strings.
    StringArray(Vec<String>),
}
