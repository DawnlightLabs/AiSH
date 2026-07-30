use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SemanticPlanKind {
    ShellCommand,
    ChangeDirectory,
    FilesystemAction,
    Answer,
    Clarification,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FilesystemOperation {
    CreateFile,
    CreateDirectory,
    Delete,
    Rename,
    Move,
    Copy,
    WriteFile,
    AppendFile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SemanticPlan {
    pub kind: SemanticPlanKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation: Option<FilesystemOperation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destination: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanParseSuccess {
    pub plan: SemanticPlan,
    pub strategy: &'static str,
    pub recovered: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanParseFailure {
    pub errors: Vec<String>,
    pub likely_incomplete: bool,
}

pub fn parse_semantic_plan(raw: &str) -> Result<PlanParseSuccess, PlanParseFailure> {
    let normalized = strip_ansi(raw).replace('\0', "");
    let trimmed = normalized.trim().trim_start_matches('\u{feff}').trim();
    let mut errors = Vec::new();

    if let Some(plan) = parse_candidate(trimmed, &mut errors) {
        return Ok(PlanParseSuccess {
            plan,
            strategy: "exact_json",
            recovered: false,
        });
    }

    let cleaned = strip_known_wrappers(trimmed);
    if cleaned != trimmed {
        if let Some(plan) = parse_candidate(&cleaned, &mut errors) {
            return Ok(PlanParseSuccess {
                plan,
                strategy: "cleaned_wrapper",
                recovered: true,
            });
        }
    }

    let objects = json_objects(trimmed);
    for candidate in objects.iter().rev() {
        if let Some(plan) = parse_candidate(candidate, &mut errors) {
            return Ok(PlanParseSuccess {
                plan,
                strategy: "final_valid_json_object",
                recovered: true,
            });
        }
    }

    errors.sort();
    errors.dedup();
    errors.truncate(8);
    if errors.is_empty() {
        errors.push("no valid semantic-plan JSON object was found".to_string());
    }
    Err(PlanParseFailure {
        likely_incomplete: likely_incomplete_json(trimmed),
        errors,
    })
}

fn parse_candidate(candidate: &str, errors: &mut Vec<String>) -> Option<SemanticPlan> {
    let value = match serde_json::from_str::<Value>(candidate.trim()) {
        Ok(value) => value,
        Err(error) => {
            errors.push(format!("JSON parse: {error}"));
            return None;
        }
    };
    match normalize_plan(value) {
        Ok(plan) => Some(plan),
        Err(error) => {
            errors.push(error);
            None
        }
    }
}

fn normalize_plan(value: Value) -> Result<SemanticPlan, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "plan must be a JSON object".to_string())?;
    let raw_kind = string_field(object, &["kind", "action_type", "action", "type"])
        .ok_or_else(|| "plan is missing kind".to_string())?;
    let kind = normalize_kind(raw_kind)?;
    let payload = cleaned_field(
        object,
        &["payload", "command", "shell_command", "content", "text"],
    );
    let destination = cleaned_field(object, &["destination", "new_name", "new_path", "to"]);
    let mut target = cleaned_field(object, &["target", "path", "directory", "source"]);
    if kind == SemanticPlanKind::ChangeDirectory && target.is_none() {
        target = destination.clone();
    }
    let scope = cleaned_field(object, &["scope", "search_scope", "root"]);
    let message = cleaned_field(
        object,
        &[
            "message",
            "fallback_message",
            "answer",
            "question",
            "reason",
        ],
    );

    let plan = SemanticPlan {
        kind,
        payload,
        target,
        scope,
        message,
        operation: string_field(object, &["operation", "filesystem_operation", "op"])
            .map(normalize_filesystem_operation)
            .transpose()?,
        destination,
    };
    validate_plan(plan)
}

fn validate_plan(plan: SemanticPlan) -> Result<SemanticPlan, String> {
    let present = |value: &Option<String>| value.as_deref().is_some_and(|text| !text.is_empty());
    match plan.kind {
        SemanticPlanKind::ShellCommand if !present(&plan.payload) => {
            Err("shell_command plan is missing payload".to_string())
        }
        SemanticPlanKind::ChangeDirectory if !present(&plan.target) => {
            Err("change_directory plan is missing target".to_string())
        }
        SemanticPlanKind::FilesystemAction
            if plan.operation.is_none() || !present(&plan.target) =>
        {
            Err("filesystem_action plan requires operation and target".to_string())
        }
        SemanticPlanKind::FilesystemAction
            if matches!(
                plan.operation,
                Some(
                    FilesystemOperation::Rename
                        | FilesystemOperation::Move
                        | FilesystemOperation::Copy
                )
            ) && !present(&plan.destination) =>
        {
            Err("filesystem_action operation requires destination".to_string())
        }
        SemanticPlanKind::FilesystemAction
            if matches!(
                plan.operation,
                Some(FilesystemOperation::WriteFile | FilesystemOperation::AppendFile)
            ) && !present(&plan.payload) =>
        {
            Err("filesystem_action write operation requires content".to_string())
        }
        SemanticPlanKind::Answer | SemanticPlanKind::Clarification if !present(&plan.message) => {
            Err("response plan is missing message".to_string())
        }
        _ => Ok(plan),
    }
}

fn normalize_kind(value: &str) -> Result<SemanticPlanKind, String> {
    let normalized = value.trim().to_ascii_lowercase().replace(['-', ' '], "_");
    match normalized.as_str() {
        "shell_command" | "command" | "run_command" | "execute" => {
            Ok(SemanticPlanKind::ShellCommand)
        }
        "change_directory" | "change_dir" | "directory" | "navigate" | "cd" => {
            Ok(SemanticPlanKind::ChangeDirectory)
        }
        "filesystem_action" | "filesystem" | "file_action" | "file_operation" => {
            Ok(SemanticPlanKind::FilesystemAction)
        }
        "answer" | "fallback" | "fallback_message" | "explanation" => Ok(SemanticPlanKind::Answer),
        "clarification" | "clarify" | "question" => Ok(SemanticPlanKind::Clarification),
        _ => Err(format!("unsupported plan kind: {value}")),
    }
}

fn normalize_filesystem_operation(value: &str) -> Result<FilesystemOperation, String> {
    let normalized = value.trim().to_ascii_lowercase().replace(['-', ' '], "_");
    match normalized.as_str() {
        "create_file" | "new_file" | "touch" => Ok(FilesystemOperation::CreateFile),
        "create_directory" | "create_folder" | "new_directory" | "new_folder" | "mkdir" => {
            Ok(FilesystemOperation::CreateDirectory)
        }
        "delete" | "remove" | "erase" => Ok(FilesystemOperation::Delete),
        "rename" => Ok(FilesystemOperation::Rename),
        "move" => Ok(FilesystemOperation::Move),
        "copy" => Ok(FilesystemOperation::Copy),
        "write_file" | "write" | "set_content" | "overwrite_file" => {
            Ok(FilesystemOperation::WriteFile)
        }
        "append_file" | "append" | "add_content" => Ok(FilesystemOperation::AppendFile),
        _ => Err(format!("unsupported filesystem operation: {value}")),
    }
}

fn string_field<'a>(object: &'a Map<String, Value>, names: &[&str]) -> Option<&'a str> {
    names
        .iter()
        .find_map(|name| object.get(*name).and_then(Value::as_str))
}

fn cleaned_field(object: &Map<String, Value>, names: &[&str]) -> Option<String> {
    string_field(object, names)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn strip_known_wrappers(raw: &str) -> String {
    let mut text = raw.trim();
    for prefix in ["assistant:", "assistant", "response:", "json:"] {
        if text
            .get(..prefix.len())
            .is_some_and(|value| value.eq_ignore_ascii_case(prefix))
        {
            text = text[prefix.len()..].trim_start();
            break;
        }
    }
    if text.starts_with("```json") || text.starts_with("```JSON") {
        text = text[7..].trim_start();
    } else if text.starts_with("```") {
        text = text[3..].trim_start();
    }
    if let Some(stripped) = text.strip_suffix("```") {
        text = stripped.trim_end();
    }
    text.to_string()
}

fn json_objects(raw: &str) -> Vec<&str> {
    let mut objects = Vec::new();
    for (start, ch) in raw.char_indices() {
        if ch != '{' {
            continue;
        }
        if let Some(length) = matching_json_object_length(&raw[start..]) {
            objects.push(&raw[start..start + length]);
        }
    }
    objects
}

fn matching_json_object_length(value: &str) -> Option<usize> {
    let mut depth = 0_usize;
    let mut in_string = false;
    let mut escaped = false;
    for (offset, ch) in value.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(offset + ch.len_utf8());
                }
            }
            _ => {}
        }
    }
    None
}

fn likely_incomplete_json(raw: &str) -> bool {
    let opens = raw.chars().filter(|ch| *ch == '{').count();
    let closes = raw.chars().filter(|ch| *ch == '}').count();
    opens > closes || (raw.contains('{') && !raw.trim_end().ends_with('}'))
}

fn strip_ansi(raw: &str) -> String {
    let mut output = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for code in chars.by_ref() {
                if ('@'..='~').contains(&code) {
                    break;
                }
            }
        } else {
            output.push(ch);
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_clean_and_legacy_plans() {
        let clean =
            parse_semantic_plan(r#"{"kind":"change_directory","target":"C:\\Work Area"}"#).unwrap();
        assert_eq!(clean.plan.kind, SemanticPlanKind::ChangeDirectory);
        assert_eq!(clean.plan.target.as_deref(), Some(r"C:\Work Area"));

        let legacy = parse_semantic_plan(
            r#"{"action_type":"shell_command","command":"git status","risk":"high","fallback_message":""}"#,
        )
        .unwrap();
        assert_eq!(legacy.plan.payload.as_deref(), Some("git status"));
    }

    #[test]
    fn recovers_wrappers_echoes_timings_and_multiple_objects() {
        let fenced = parse_semantic_plan(
            "assistant:\n```json\n{\"kind\":\"answer\",\"message\":\"Done.\"}\n```",
        )
        .unwrap();
        assert_eq!(fenced.strategy, "cleaned_wrapper");

        let noisy = parse_semantic_plan(
            "Context: {\"cwd\":\"X\"}\n{\"kind\":\"clarification\",\"message\":\"Which one?\"}\nllama_perf: 2ms",
        )
        .unwrap();
        assert_eq!(noisy.strategy, "final_valid_json_object");
        assert_eq!(noisy.plan.kind, SemanticPlanKind::Clarification);

        let multiple = parse_semantic_plan(
            "{\"kind\":\"answer\",\"message\":\"draft\"}\n{\"kind\":\"answer\",\"message\":\"final\"}",
        )
        .unwrap();
        assert_eq!(multiple.plan.message.as_deref(), Some("final"));
    }

    #[test]
    fn reports_truncated_malformed_and_prose_without_executing_it() {
        let truncated =
            parse_semantic_plan(r#"{"kind":"shell_command","payload":"git"#).unwrap_err();
        assert!(truncated.likely_incomplete);
        assert!(parse_semantic_plan("Run git status").is_err());
        assert!(parse_semantic_plan(r#"{"kind":"shell_command","payload":12}"#).is_err());
    }

    #[test]
    fn parses_typed_filesystem_actions_and_requires_destinations() {
        let create = parse_semantic_plan(
            r#"{"kind":"filesystem_action","operation":"create_directory","target":"Alpha Beta","scope":"desktop"}"#,
        )
        .unwrap();
        assert_eq!(create.plan.kind, SemanticPlanKind::FilesystemAction);
        assert_eq!(
            create.plan.operation,
            Some(FilesystemOperation::CreateDirectory)
        );
        assert_eq!(create.plan.target.as_deref(), Some("Alpha Beta"));

        let rename = parse_semantic_plan(
            r#"{"kind":"file_operation","operation":"rename","source":"old-name.txt","new_name":"New Name.txt"}"#,
        )
        .unwrap();
        assert_eq!(rename.plan.operation, Some(FilesystemOperation::Rename));
        assert_eq!(rename.plan.destination.as_deref(), Some("New Name.txt"));

        let write = parse_semantic_plan(
            r#"{"kind":"filesystem_action","operation":"write_file","target":"Result File.txt","content":"hello world"}"#,
        )
        .unwrap();
        assert_eq!(write.plan.operation, Some(FilesystemOperation::WriteFile));
        assert_eq!(write.plan.target.as_deref(), Some("Result File.txt"));
        assert_eq!(write.plan.payload.as_deref(), Some("hello world"));

        assert!(parse_semantic_plan(
            r#"{"kind":"filesystem_action","operation":"move","target":"artifact.zip"}"#
        )
        .is_err());
        assert!(parse_semantic_plan(
            r#"{"kind":"filesystem_action","operation":"append_file","target":"notes.txt"}"#
        )
        .is_err());
    }
}
