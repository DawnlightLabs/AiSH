use aish_ai::{FilesystemOperation, SemanticPlan, SemanticPlanKind};
use serde_json::Value;
use std::path::{Path, PathBuf};

const MAX_VISITED_ENTRIES: usize = 4096;
const MAX_RECURSION_DEPTH: usize = 5;

pub(crate) fn ground_filesystem_mutation(plan: &mut SemanticPlan, request: &str, context: &Value) {
    if ground_multiple_named_creations(plan, request, context) {
        return;
    }
    if plan.kind == SemanticPlanKind::FilesystemAction {
        if filesystem_operation_matches_request(plan.operation.as_ref(), request) {
            ground_typed_filesystem_action(plan, request, context);
        }
    } else if is_named_creation(request) {
        ground_named_creation(plan, request, context);
    } else if is_singular_deletion(request) {
        ground_singular_deletion(plan, request, context);
    }
}

fn ground_multiple_named_creations(
    plan: &mut SemanticPlan,
    request: &str,
    context: &Value,
) -> bool {
    let clauses = ordered_request_clauses(request);
    if clauses.len() < 2
        || clauses.len() > 4
        || clauses.iter().any(|clause| !is_named_creation(clause))
    {
        return false;
    }
    let Some(base_scope) = request_scope("", context) else {
        return false;
    };
    let mut previous_target: Option<PathBuf> = None;
    let mut commands = Vec::new();
    for clause in clauses {
        let Some(name) = requested_new_name(clause) else {
            clarification(
                plan,
                "Please provide one valid name for each new item.".to_string(),
            );
            return true;
        };
        if !valid_single_name(&name) {
            clarification(
                plan,
                "Please provide one valid name for each new item.".to_string(),
            );
            return true;
        }
        let lower = clause.to_ascii_lowercase();
        let relative_to_previous = lower.contains(" inside it")
            || lower.contains(" in it")
            || lower.contains(" under it")
            || lower.contains(" within it");
        let scope = if relative_to_previous {
            let Some(previous) = previous_target.as_ref() else {
                clarification(
                    plan,
                    "Which previously created folder should contain the next item?".to_string(),
                );
                return true;
            };
            previous.clone()
        } else {
            known_folder_scope(clause, context).unwrap_or_else(|| base_scope.clone())
        };
        let target = scope.join(name);
        if target.exists() {
            clarification(
                plan,
                format!("An item named '{}' already exists.", target.display()),
            );
            return true;
        }
        let kind = expected_entry_kind(clause);
        commands.push(create_command(&target, kind));
        commands.push(verify_exists_command(&target));
        previous_target = Some(target);
    }
    complete_with_command(plan, commands.join("; "));
    true
}

fn ordered_request_clauses(request: &str) -> Vec<&str> {
    let lower = request.to_ascii_lowercase();
    let mut clauses = Vec::new();
    let mut cursor = 0;
    while cursor < request.len() {
        let remainder = &lower[cursor..];
        let next = [" and then ", " then "]
            .into_iter()
            .filter_map(|separator| {
                remainder
                    .find(separator)
                    .map(|offset| (cursor + offset, separator.len()))
            })
            .min_by_key(|(index, _)| *index);
        let Some((index, separator_length)) = next else {
            break;
        };
        let clause = request[cursor..index].trim();
        if !clause.is_empty() {
            clauses.push(clause);
        }
        cursor = index + separator_length;
    }
    let final_clause = request[cursor..].trim();
    if !final_clause.is_empty() {
        clauses.push(final_clause);
    }
    clauses
}

pub(crate) fn filesystem_operation_matches_request(
    operation: Option<&FilesystemOperation>,
    request: &str,
) -> bool {
    let words = request_words(request);
    let contains = |candidates: &[&str]| {
        words
            .iter()
            .any(|word| candidates.iter().any(|candidate| word == candidate))
    };
    match operation {
        Some(FilesystemOperation::CreateFile | FilesystemOperation::CreateDirectory) => {
            contains(&["create", "make", "new"])
        }
        Some(FilesystemOperation::Delete) => contains(&["delete", "remove", "erase"]),
        Some(FilesystemOperation::Rename) => contains(&["rename"]),
        Some(FilesystemOperation::Move) => {
            contains(&["move"])
                && !(contains(&["up", "parent"])
                    && contains(&["directory", "folder"])
                    && !contains(&["file"]))
        }
        Some(FilesystemOperation::Copy) => contains(&["copy", "duplicate"]),
        Some(FilesystemOperation::WriteFile) => contains(&["write", "overwrite", "replace"]),
        Some(FilesystemOperation::AppendFile) => contains(&["append", "add"]),
        None => false,
    }
}

fn ground_typed_filesystem_action(plan: &mut SemanticPlan, request: &str, context: &Value) {
    let Some(operation) = plan.operation.clone() else {
        clarification(
            plan,
            "Which filesystem operation should I perform?".to_string(),
        );
        return;
    };
    let Some(target_reference) = plan.target.clone() else {
        clarification(plan, "Which file or folder should I use?".to_string());
        return;
    };
    let Some(scope) = typed_action_scope(&operation, plan.scope.as_deref(), request, context)
    else {
        clarification(
            plan,
            "I could not verify the requested filesystem location.".to_string(),
        );
        return;
    };

    match operation {
        FilesystemOperation::CreateFile | FilesystemOperation::CreateDirectory => {
            let name = requested_new_name(request).unwrap_or_else(|| {
                reference_name(&target_reference).unwrap_or_else(|| target_reference.clone())
            });
            if !valid_single_name(&name) {
                clarification(plan, "Please provide one valid new item name.".to_string());
                return;
            }
            let target = scope.join(&name);
            if target.exists() {
                clarification(
                    plan,
                    format!(
                        "An item named '{name}' already exists in {}.",
                        scope.display()
                    ),
                );
                return;
            }
            let kind = if operation == FilesystemOperation::CreateFile {
                EntryKind::File
            } else {
                EntryKind::Directory
            };
            complete_with_verified_command(
                plan,
                create_command(&target, kind),
                verify_exists_command(&target),
            );
        }
        FilesystemOperation::Delete => {
            let matches = matching_entries(
                &scope,
                request,
                EntryKind::Either,
                requests_recursive_search(request),
            );
            match matches.as_slice() {
                [target] => complete_with_verified_command(
                    plan,
                    delete_command(target),
                    verify_absent_command(target),
                ),
                [] => clarification(
                    plan,
                    format!(
                        "I could not find an existing item matching '{}' in {}.",
                        target_reference,
                        scope.display()
                    ),
                ),
                _ => clarification(
                    plan,
                    format!(
                        "I found multiple items matching '{}' in {}. Please provide the exact name.",
                        target_reference,
                        scope.display()
                    ),
                ),
            }
        }
        FilesystemOperation::Rename => {
            let source_reference = requested_source_reference(request).unwrap_or(&target_reference);
            let Some(source) = resolve_unique_source(plan, &scope, source_reference, request)
            else {
                return;
            };
            let Some(destination) = requested_destination_name(request, context)
                .or_else(|| plan.destination.as_deref().and_then(reference_name))
            else {
                clarification(plan, "What should the item be renamed to?".to_string());
                return;
            };
            let new_name = destination.trim().trim_matches(['\'', '"']);
            if !valid_single_name(new_name) {
                clarification(plan, "Please provide one valid new name.".to_string());
                return;
            }
            let destination_path = source.parent().unwrap_or(&scope).join(new_name);
            if destination_path.exists() {
                clarification(plan, format!("An item named '{new_name}' already exists."));
                return;
            }
            complete_with_verified_command(
                plan,
                rename_command(&source, new_name),
                verify_relocated_command(&source, &destination_path),
            );
        }
        FilesystemOperation::Move | FilesystemOperation::Copy => {
            let source_reference = requested_source_reference(request).unwrap_or(&target_reference);
            let Some(source) = resolve_unique_source(plan, &scope, source_reference, request)
            else {
                return;
            };
            let Some(destination_reference) = plan.destination.clone() else {
                clarification(plan, "Where should the item go?".to_string());
                return;
            };
            let Some(destination) =
                resolve_destination(&destination_reference, request, context, &scope)
            else {
                clarification(
                    plan,
                    format!(
                        "I could not verify a safe destination matching '{}'.",
                        destination_reference
                    ),
                );
                return;
            };
            let command = if operation == FilesystemOperation::Move {
                move_command(&source, &destination)
            } else {
                copy_command(&source, &destination)
            };
            let final_destination = final_destination_path(&source, &destination);
            let verification = if operation == FilesystemOperation::Move {
                verify_relocated_command(&source, &final_destination)
            } else {
                verify_exists_command(&final_destination)
            };
            complete_with_verified_command(plan, command, verification);
        }
        FilesystemOperation::WriteFile | FilesystemOperation::AppendFile => {
            let target_name = requested_write_target(request)
                .or_else(|| reference_name(&target_reference))
                .unwrap_or(target_reference);
            if !valid_single_name(&target_name) {
                clarification(plan, "Please provide one valid file name.".to_string());
                return;
            }
            let Some(content) = requested_write_content(request).or_else(|| plan.payload.clone())
            else {
                clarification(plan, "What content should I write?".to_string());
                return;
            };
            let target = scope.join(target_name);
            if target.is_dir() {
                clarification(
                    plan,
                    "The requested target is a directory, not a file.".to_string(),
                );
                return;
            }
            complete_with_verified_command(
                plan,
                write_file_command(
                    &target,
                    &content,
                    operation == FilesystemOperation::AppendFile,
                ),
                verify_exists_command(&target),
            );
        }
    }
}

fn typed_action_scope(
    operation: &FilesystemOperation,
    scope: Option<&str>,
    request: &str,
    context: &Value,
) -> Option<PathBuf> {
    if matches!(
        operation,
        FilesystemOperation::Move | FilesystemOperation::Copy
    ) {
        let source_request = request
            .to_ascii_lowercase()
            .rfind(" to ")
            .map(|index| &request[..index])
            .unwrap_or(request);
        if let Some(path) = known_folder_scope(source_request, context) {
            return Some(path);
        }
    }
    if let Some(path) = known_folder_scope(request, context) {
        return Some(path);
    }
    if let Some(scope) = scope {
        if let Some(path) = known_folder_scope(scope, context) {
            return Some(path);
        }
        let path = PathBuf::from(scope.trim().trim_matches(['\'', '"']));
        let rooted = if path.is_absolute() {
            path
        } else {
            context
                .get("cwd")
                .and_then(Value::as_str)
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."))
                .join(path)
        };
        if rooted.is_dir() {
            return Some(rooted);
        }
    }
    request_scope("", context)
}

fn reference_name(reference: &str) -> Option<String> {
    Path::new(reference.trim().trim_matches(['\'', '"']))
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
}

fn requested_destination_name(request: &str, context: &Value) -> Option<String> {
    let lower = request.to_ascii_lowercase();
    let start = lower.rfind(" to ")? + 4;
    let remainder = request[start..].trim();
    let lower_remainder = remainder.to_ascii_lowercase();
    let mut end = remainder.len();
    if let Some(known) = context.get("known_folders").and_then(Value::as_object) {
        for name in known.keys() {
            for prefix in [" in ", " on ", " under ", " inside "] {
                for candidate in [name.as_str(), name.trim_end_matches('s')] {
                    let suffix = format!("{prefix}{candidate}");
                    if let Some(index) = lower_remainder.rfind(&suffix) {
                        if index + suffix.len() == lower_remainder.len() {
                            end = end.min(index);
                        }
                    }
                }
            }
        }
    }
    let name = remainder[..end]
        .trim()
        .trim_matches(['\'', '"', '.', ',', ';']);
    (!name.is_empty()).then(|| name.to_string())
}

fn requested_source_reference(request: &str) -> Option<&str> {
    let trimmed = request.trim();
    let lower = trimmed.to_ascii_lowercase();
    let start = ["rename ", "move ", "copy ", "duplicate "]
        .into_iter()
        .find_map(|prefix| lower.starts_with(prefix).then_some(prefix.len()))?;
    let remainder = &trimmed[start..];
    let lower_remainder = remainder.to_ascii_lowercase();
    let end = [
        " into ", " to ", " from ", " in ", " on ", " under ", " inside ",
    ]
    .into_iter()
    .filter_map(|separator| lower_remainder.find(separator))
    .min()
    .unwrap_or(remainder.len());
    let reference = remainder[..end].trim().trim_matches(['\'', '"', ',', ';']);
    (!reference.is_empty()).then_some(reference)
}

fn requested_write_parts(request: &str) -> Option<(&str, &str)> {
    let trimmed = request.trim();
    let lower = trimmed.to_ascii_lowercase();
    let start = ["write ", "append ", "overwrite "]
        .into_iter()
        .find_map(|prefix| lower.starts_with(prefix).then_some(prefix.len()))?;
    let remainder = &trimmed[start..];
    let lower_remainder = remainder.to_ascii_lowercase();
    let separator = [" into ", " to "]
        .into_iter()
        .filter_map(|separator| {
            lower_remainder
                .rfind(separator)
                .map(|index| (index, separator.len()))
        })
        .max_by_key(|(index, _)| *index)?;
    let content = remainder[..separator.0].trim().trim_matches(['\'', '"']);
    let target = remainder[separator.0 + separator.1..]
        .trim()
        .trim_matches(['\'', '"', '.', ',', ';']);
    (!content.is_empty() && !target.is_empty()).then_some((content, target))
}

fn requested_write_content(request: &str) -> Option<String> {
    requested_write_parts(request).map(|(content, _)| content.to_string())
}

fn requested_write_target(request: &str) -> Option<String> {
    requested_write_parts(request).map(|(_, target)| target.to_string())
}

fn resolve_unique_source(
    plan: &mut SemanticPlan,
    scope: &Path,
    reference: &str,
    request: &str,
) -> Option<PathBuf> {
    let mut matches = matching_entries(
        scope,
        reference,
        EntryKind::Either,
        requests_recursive_search(request),
    );
    if matches.is_empty() && normalized_match_key(reference) != normalized_match_key(request) {
        matches = matching_entries(
            scope,
            request,
            EntryKind::Either,
            requests_recursive_search(request),
        );
    }
    match matches.as_slice() {
        [source] => Some(source.clone()),
        [] => {
            clarification(
                plan,
                format!(
                    "I could not find an existing item matching '{}' in {}.",
                    reference,
                    scope.display()
                ),
            );
            None
        }
        _ => {
            clarification(
                plan,
                format!(
                    "I found multiple items matching '{}' in {}. Please provide the exact name.",
                    reference,
                    scope.display()
                ),
            );
            None
        }
    }
}

fn resolve_destination(
    reference: &str,
    request: &str,
    context: &Value,
    source_scope: &Path,
) -> Option<PathBuf> {
    if let Some(path) = known_folder_scope(reference, context) {
        return Some(path);
    }
    let path = PathBuf::from(reference.trim().trim_matches(['\'', '"']));
    let rooted = if path.is_absolute() {
        path
    } else {
        source_scope.join(path)
    };
    if rooted.is_dir() {
        return Some(rooted);
    }
    if rooted.exists() {
        return None;
    }
    if !rooted.exists()
        && rooted
            .extension()
            .is_some_and(|extension| !extension.is_empty())
        && rooted
            .parent()
            .is_some_and(|parent| verified_destination_parent(parent, source_scope, context))
        && rooted
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(valid_single_name)
    {
        return Some(rooted);
    }
    let destination_request = request
        .to_ascii_lowercase()
        .rfind(" to ")
        .map(|index| &request[index + 4..])
        .unwrap_or(request);
    if let Some(path) = known_folder_scope(destination_request, context) {
        return Some(path);
    }
    if let Some(path) = known_folder_scope(request, context) {
        if path != source_scope {
            return Some(path);
        }
    }
    let matches = matching_entries(source_scope, reference, EntryKind::Directory, false);
    (matches.len() == 1).then(|| matches[0].clone())
}

fn verified_destination_parent(parent: &Path, source_scope: &Path, context: &Value) -> bool {
    if parent == source_scope {
        return true;
    }
    context
        .get("known_folders")
        .and_then(Value::as_object)
        .is_some_and(|known| {
            known.values().any(|value| {
                value
                    .as_str()
                    .map(Path::new)
                    .is_some_and(|known_path| known_path == parent)
            })
        })
}

fn complete_with_command(plan: &mut SemanticPlan, command: String) {
    plan.kind = SemanticPlanKind::ShellCommand;
    plan.payload = Some(command);
    plan.target = None;
    plan.scope = None;
    plan.message = None;
    plan.operation = None;
    plan.destination = None;
}

fn complete_with_verified_command(plan: &mut SemanticPlan, command: String, verification: String) {
    complete_with_command(plan, format!("{command}; {verification}"));
}

fn final_destination_path(source: &Path, destination: &Path) -> PathBuf {
    if destination.is_dir() {
        source
            .file_name()
            .map(|name| destination.join(name))
            .unwrap_or_else(|| destination.to_path_buf())
    } else {
        destination.to_path_buf()
    }
}

fn ground_singular_deletion(plan: &mut SemanticPlan, request: &str, context: &Value) {
    let Some(scope) = request_scope(request, context) else {
        return;
    };
    let expected = expected_entry_kind(request);
    let matches = matching_entries(
        &scope,
        request,
        expected,
        requests_recursive_search(request),
    );
    match matches.as_slice() {
        [target] => {
            complete_with_verified_command(
                plan,
                delete_command(target),
                verify_absent_command(target),
            );
        }
        [] => clarification(
            plan,
            format!(
                "I could not find an existing {} matching that name in {}.",
                expected.description(),
                scope.display()
            ),
        ),
        _ => clarification(
            plan,
            format!(
                "I found multiple matching {}s in {}. Please provide the exact name.",
                expected.description(),
                scope.display()
            ),
        ),
    }
}

fn ground_named_creation(plan: &mut SemanticPlan, request: &str, context: &Value) {
    let Some(scope) = known_folder_scope(request, context) else {
        return;
    };
    let Some(name) = requested_new_name(request) else {
        return;
    };
    if !valid_single_name(&name) {
        clarification(plan, "Please provide one file or folder name.".to_string());
        return;
    }
    let target = scope.join(&name);
    if target.exists() {
        clarification(
            plan,
            format!(
                "An item named '{name}' already exists in {}.",
                scope.display()
            ),
        );
        return;
    }

    let kind = expected_entry_kind(request);
    complete_with_verified_command(
        plan,
        create_command(&target, kind),
        verify_exists_command(&target),
    );
}

fn clarification(plan: &mut SemanticPlan, message: String) {
    plan.kind = SemanticPlanKind::Clarification;
    plan.payload = None;
    plan.target = None;
    plan.scope = None;
    plan.message = Some(message);
    plan.operation = None;
    plan.destination = None;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EntryKind {
    File,
    Directory,
    Either,
}

impl EntryKind {
    fn accepts(self, path: &Path) -> bool {
        match self {
            Self::File => path.is_file(),
            Self::Directory => path.is_dir(),
            Self::Either => path.is_file() || path.is_dir(),
        }
    }

    fn description(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Directory => "folder",
            Self::Either => "file or folder",
        }
    }
}

fn expected_entry_kind(request: &str) -> EntryKind {
    let words = request_words(request);
    if words.iter().any(|word| {
        matches!(
            word.as_str(),
            "file" | "files" | "zip" | "archive" | "document"
        )
    }) {
        EntryKind::File
    } else if words.iter().any(|word| {
        matches!(
            word.as_str(),
            "folder" | "folders" | "directory" | "directories"
        )
    }) {
        EntryKind::Directory
    } else {
        EntryKind::Either
    }
}

fn is_singular_deletion(request: &str) -> bool {
    let words = request_words(request);
    words
        .iter()
        .any(|word| matches!(word.as_str(), "delete" | "remove" | "erase"))
        && !words
            .iter()
            .any(|word| matches!(word.as_str(), "all" | "every" | "contents"))
        && !words
            .iter()
            .any(|word| matches!(word.as_str(), "cleanup" | "clean"))
}

fn is_named_creation(request: &str) -> bool {
    let words = request_words(request);
    words
        .iter()
        .any(|word| matches!(word.as_str(), "create" | "make"))
        && words
            .iter()
            .any(|word| matches!(word.as_str(), "file" | "folder" | "directory"))
        && words.iter().any(|word| word == "named" || word == "called")
}

fn requests_recursive_search(request: &str) -> bool {
    let lower = request.to_ascii_lowercase();
    lower.contains("recursive")
        || lower.contains("subdirector")
        || lower.contains("every director")
        || lower.contains("all director")
}

fn request_scope(request: &str, context: &Value) -> Option<PathBuf> {
    if let Some(path) = known_folder_scope(request, context) {
        return Some(path);
    }
    context
        .get("cwd")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
}

fn known_folder_scope(request: &str, context: &Value) -> Option<PathBuf> {
    let words = request_words(request);
    if let Some(known) = context.get("known_folders").and_then(Value::as_object) {
        for (name, value) in known {
            if words.iter().any(|word| scope_name_matches(word, name)) {
                let path = PathBuf::from(value.as_str()?);
                return path.is_dir().then_some(path);
            }
        }
    }
    None
}

fn scope_name_matches(word: &str, scope_name: &str) -> bool {
    word == scope_name || word.trim_end_matches('s') == scope_name.trim_end_matches('s')
}

fn matching_entries(
    scope: &Path,
    request: &str,
    expected: EntryKind,
    recursive: bool,
) -> Vec<PathBuf> {
    let request_key = normalized_match_key(request);
    let mut scored = Vec::new();
    let mut visited = 0;
    collect_matches(
        scope,
        &request_key,
        expected,
        recursive,
        0,
        &mut visited,
        &mut scored,
    );
    let Some(best_score) = scored.iter().map(|(score, _)| *score).max() else {
        return Vec::new();
    };
    scored
        .into_iter()
        .filter_map(|(score, path)| (score == best_score).then_some(path))
        .collect()
}

fn collect_matches(
    directory: &Path,
    request_key: &str,
    expected: EntryKind,
    recursive: bool,
    depth: usize,
    visited: &mut usize,
    matches: &mut Vec<(u8, PathBuf)>,
) {
    if *visited >= MAX_VISITED_ENTRIES || depth > MAX_RECURSION_DEPTH {
        return;
    }
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        if *visited >= MAX_VISITED_ENTRIES {
            return;
        }
        *visited += 1;
        let path = entry.path();
        if expected.accepts(&path) {
            let name = entry.file_name().to_string_lossy().into_owned();
            let name_key = normalized_match_key(&name);
            let stem_key = path
                .file_stem()
                .map(|stem| normalized_match_key(&stem.to_string_lossy()))
                .unwrap_or_default();
            let score = if !name_key.is_empty() && request_key == name_key {
                4
            } else if !name_key.is_empty() && request_key.contains(&name_key) {
                3
            } else if stem_key.len() >= 3 && request_key.contains(&stem_key) {
                2
            } else {
                0
            };
            if score > 0 {
                matches.push((score, path.clone()));
            }
        }
        if recursive && path.is_dir() {
            collect_matches(
                &path,
                request_key,
                expected,
                true,
                depth + 1,
                visited,
                matches,
            );
        }
    }
}

fn requested_new_name(request: &str) -> Option<String> {
    let lower = request.to_ascii_lowercase();
    let marker = [" named ", " called "]
        .into_iter()
        .filter_map(|marker| lower.find(marker).map(|index| (index, marker)))
        .min_by_key(|(index, _)| *index)?;
    let start = marker.0 + marker.1.len();
    let remainder = request[start..].trim();
    let lower_remainder = remainder.to_ascii_lowercase();
    let end = [" on ", " in ", " under ", " inside "]
        .into_iter()
        .filter_map(|separator| lower_remainder.find(separator))
        .min()
        .unwrap_or(remainder.len());
    let name = remainder[..end]
        .trim()
        .trim_matches(['\'', '"', '.', ',', ';']);
    (!name.is_empty()).then(|| name.to_string())
}

fn valid_single_name(name: &str) -> bool {
    name != "." && name != ".." && !name.contains(['/', '\\']) && !name.contains('\0')
}

fn request_words(request: &str) -> Vec<String> {
    request
        .split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(|word| word.to_lowercase())
        .collect()
}

fn normalized_match_key(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn quote(value: &Path) -> String {
    format!("'{}'", value.to_string_lossy().replace('\'', "''"))
}

#[cfg(windows)]
fn delete_command(target: &Path) -> String {
    let recurse = if target.is_dir() { " -Recurse" } else { "" };
    format!("Remove-Item -LiteralPath {}{recurse} -Force", quote(target))
}

#[cfg(not(windows))]
fn delete_command(target: &Path) -> String {
    if target.is_dir() {
        format!("rm -rf -- {}", quote(target))
    } else {
        format!("rm -- {}", quote(target))
    }
}

#[cfg(windows)]
fn create_command(target: &Path, kind: EntryKind) -> String {
    let item_type = if kind == EntryKind::File {
        "File"
    } else {
        "Directory"
    };
    format!("New-Item -ItemType {item_type} -Path {}", quote(target))
}

#[cfg(windows)]
fn write_file_command(target: &Path, content: &str, append: bool) -> String {
    let command = if append { "Add-Content" } else { "Set-Content" };
    format!(
        "{command} -LiteralPath {} -Value '{}'",
        quote(target),
        content.replace('\'', "''")
    )
}

#[cfg(not(windows))]
fn write_file_command(target: &Path, content: &str, append: bool) -> String {
    let redirect = if append { ">>" } else { ">" };
    format!(
        "printf '%s\\n' '{}' {redirect} {}",
        content.replace('\'', "'\"'\"'"),
        quote(target)
    )
}

#[cfg(not(windows))]
fn create_command(target: &Path, kind: EntryKind) -> String {
    if kind == EntryKind::File {
        format!("touch -- {}", quote(target))
    } else {
        format!("mkdir -- {}", quote(target))
    }
}

#[cfg(windows)]
fn rename_command(source: &Path, new_name: &str) -> String {
    format!(
        "Rename-Item -LiteralPath {} -NewName '{}'",
        quote(source),
        new_name.replace('\'', "''")
    )
}

#[cfg(not(windows))]
fn rename_command(source: &Path, new_name: &str) -> String {
    let destination = source.parent().unwrap_or(Path::new(".")).join(new_name);
    format!("mv -- {} {}", quote(source), quote(&destination))
}

#[cfg(windows)]
fn move_command(source: &Path, destination: &Path) -> String {
    format!(
        "Move-Item -LiteralPath {} -Destination {}",
        quote(source),
        quote(destination)
    )
}

#[cfg(not(windows))]
fn move_command(source: &Path, destination: &Path) -> String {
    format!("mv -- {} {}", quote(source), quote(destination))
}

#[cfg(windows)]
fn copy_command(source: &Path, destination: &Path) -> String {
    let recurse = if source.is_dir() { " -Recurse" } else { "" };
    format!(
        "Copy-Item -LiteralPath {} -Destination {}{recurse}",
        quote(source),
        quote(destination)
    )
}

#[cfg(not(windows))]
fn copy_command(source: &Path, destination: &Path) -> String {
    let recurse = if source.is_dir() { "-R " } else { "" };
    format!("cp {recurse}-- {} {}", quote(source), quote(destination))
}

#[cfg(windows)]
fn verify_exists_command(target: &Path) -> String {
    format!(
        "if (-not (Test-Path -LiteralPath {})) {{ throw 'AiSH verification failed: expected item is missing.' }} else {{ Write-Output 'AiSH verified the requested filesystem change.' }}",
        quote(target)
    )
}

#[cfg(not(windows))]
fn verify_exists_command(target: &Path) -> String {
    format!(
        "test -e {} && printf '%s\\n' 'AiSH verified the requested filesystem change.' || {{ printf '%s\\n' 'AiSH verification failed: expected item is missing.' >&2; exit 1; }}",
        quote(target)
    )
}

#[cfg(windows)]
fn verify_absent_command(target: &Path) -> String {
    format!(
        "if (Test-Path -LiteralPath {}) {{ throw 'AiSH verification failed: item still exists.' }} else {{ Write-Output 'AiSH verified the requested filesystem change.' }}",
        quote(target)
    )
}

#[cfg(not(windows))]
fn verify_absent_command(target: &Path) -> String {
    format!(
        "test ! -e {} && printf '%s\\n' 'AiSH verified the requested filesystem change.' || {{ printf '%s\\n' 'AiSH verification failed: item still exists.' >&2; exit 1; }}",
        quote(target)
    )
}

#[cfg(windows)]
fn verify_relocated_command(source: &Path, destination: &Path) -> String {
    format!(
        "if ((Test-Path -LiteralPath {}) -or (-not (Test-Path -LiteralPath {}))) {{ throw 'AiSH verification failed: source or destination state is incorrect.' }} else {{ Write-Output 'AiSH verified the requested filesystem change.' }}",
        quote(source),
        quote(destination)
    )
}

#[cfg(not(windows))]
fn verify_relocated_command(source: &Path, destination: &Path) -> String {
    format!(
        "test ! -e {} && test -e {} && printf '%s\\n' 'AiSH verified the requested filesystem change.' || {{ printf '%s\\n' 'AiSH verification failed: source or destination state is incorrect.' >&2; exit 1; }}",
        quote(source),
        quote(destination)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture() -> (PathBuf, Value) {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("aish-target-resolution-{unique}"));
        let downloads = root.join("Redirected Downloads");
        let desktop = root.join("Cloud Desktop");
        fs::create_dir_all(&downloads).expect("downloads");
        fs::create_dir_all(&desktop).expect("desktop");
        let context = serde_json::json!({
            "cwd": root,
            "known_folders": {
                "downloads": downloads,
                "desktop": desktop,
            }
        });
        (root, context)
    }

    fn plan(payload: &str) -> SemanticPlan {
        SemanticPlan {
            kind: SemanticPlanKind::ShellCommand,
            payload: Some(payload.to_string()),
            target: None,
            scope: None,
            message: None,
            operation: None,
            destination: None,
        }
    }

    fn filesystem_plan(
        operation: FilesystemOperation,
        target: &str,
        destination: Option<&str>,
        scope: Option<&str>,
    ) -> SemanticPlan {
        SemanticPlan {
            kind: SemanticPlanKind::FilesystemAction,
            payload: None,
            target: Some(target.to_string()),
            scope: scope.map(str::to_string),
            message: None,
            operation: Some(operation),
            destination: destination.map(str::to_string),
        }
    }

    #[test]
    fn deletion_uses_the_existing_separator_variant() {
        let (root, context) = fixture();
        let actual = PathBuf::from(context["known_folders"]["downloads"].as_str().unwrap())
            .join("local-companion.zip");
        fs::write(&actual, b"fixture").expect("file");
        let mut value = plan("Remove-Item guessed_path");

        ground_filesystem_mutation(
            &mut value,
            "remove local companion zip in downloads",
            &context,
        );

        let command = value.payload.expect("grounded command");
        assert!(command.contains("local-companion.zip"));
        assert!(!command.contains("guessed_path"));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn ambiguous_deletion_asks_for_the_exact_name() {
        let (root, context) = fixture();
        let downloads = PathBuf::from(context["known_folders"]["downloads"].as_str().unwrap());
        fs::write(downloads.join("local-companion.zip"), b"a").expect("file");
        fs::write(downloads.join("local_companion.zip"), b"b").expect("file");
        let mut value = plan("Remove-Item guessed_path");

        ground_filesystem_mutation(
            &mut value,
            "remove local companion zip in downloads",
            &context,
        );

        assert_eq!(
            value.kind,
            SemanticPlanKind::Clarification,
            "unexpected grounded plan: {value:?}"
        );
        assert!(value.payload.is_none());
        assert!(value.message.unwrap().contains("multiple"));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn missing_deletion_never_keeps_the_models_guess() {
        let (root, context) = fixture();
        let mut value = plan("Remove-Item invented.zip");

        ground_filesystem_mutation(
            &mut value,
            "remove absent artifact zip in downloads",
            &context,
        );

        assert_eq!(value.kind, SemanticPlanKind::Clarification);
        assert!(value.payload.is_none());
        assert!(value.message.unwrap().contains("could not find"));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn typed_deletion_ignores_an_invented_model_path() {
        let (root, context) = fixture();
        let actual = root.join("Disposable 8427.tmp");
        fs::write(&actual, b"fixture").expect("file");
        let mut value = filesystem_plan(
            FilesystemOperation::Delete,
            r"C:\invented\OneDrive\Temp\aish-dynamic-8427.tmp",
            None,
            Some("current directory"),
        );

        ground_filesystem_mutation(&mut value, "delete Disposable 8427.tmp", &context);

        assert_eq!(value.kind, SemanticPlanKind::ShellCommand);
        let command = value.payload.expect("grounded command");
        assert!(command.contains(&actual.to_string_lossy().to_string()));
        assert!(!command.contains("invented"));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn creation_uses_the_redirected_known_folder() {
        let (root, context) = fixture();
        let desktop = PathBuf::from(context["known_folders"]["desktop"].as_str().unwrap());
        let mut value = plan("New-Item $env:USERPROFILE\\Desktop\\barcelona");

        ground_filesystem_mutation(
            &mut value,
            "make a folder named barcelona on desktop",
            &context,
        );

        let command = value.payload.expect("grounded command");
        assert!(command.contains(&desktop.to_string_lossy().to_string()));
        assert!(command.contains("barcelona"));
        assert!(!command.contains("USERPROFILE"));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn folder_resolution_preserves_actual_spaces_case_and_unicode() {
        let (root, context) = fixture();
        let downloads = PathBuf::from(context["known_folders"]["downloads"].as_str().unwrap());
        let actual = downloads.join("MiXeD Café Folder");
        fs::create_dir(&actual).expect("folder");
        let mut value = plan("Remove-Item mixed_cafe_folder -Recurse -Force");

        ground_filesystem_mutation(
            &mut value,
            "delete the mixed café folder in download",
            &context,
        );

        let command = value.payload.expect("grounded command");
        assert!(command.contains("MiXeD Café Folder"));
        assert!(!command.contains("mixed_cafe_folder"));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn creation_preserves_the_requested_name_instead_of_model_punctuation() {
        let (root, context) = fixture();
        let mut value = plan("New-Item -ItemType Directory -Path alpha_beta");

        ground_filesystem_mutation(
            &mut value,
            "create a folder named Alpha Beta on desktop",
            &context,
        );

        let command = value.payload.expect("grounded command");
        assert!(command.contains("Alpha Beta"));
        assert!(!command.contains("alpha_beta"));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn typed_rename_resolves_source_and_preserves_new_name() {
        let (root, context) = fixture();
        let downloads = PathBuf::from(context["known_folders"]["downloads"].as_str().unwrap());
        fs::write(downloads.join("old-file.txt"), b"fixture").expect("file");
        let mut value = filesystem_plan(
            FilesystemOperation::Rename,
            "old file.txt",
            Some("New File.txt"),
            Some("downloads"),
        );

        ground_filesystem_mutation(
            &mut value,
            "rename old file.txt to New File.txt in downloads",
            &context,
        );

        assert_eq!(value.kind, SemanticPlanKind::ShellCommand);
        let command = value.payload.expect("grounded command");
        assert!(command.contains("old-file.txt"));
        assert!(command.contains("New File.txt"));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn typed_rename_ignores_model_completed_paths() {
        let (root, context) = fixture();
        let downloads = PathBuf::from(context["known_folders"]["downloads"].as_str().unwrap());
        fs::write(
            downloads.join("aish-source-acceptance-8427.txt"),
            b"fixture",
        )
        .expect("file");
        let mut value = filesystem_plan(
            FilesystemOperation::Rename,
            r"D:\invented\source\acceptance\8427.txt",
            Some(r"D:\invented\downloads\Final Name 8427.txt"),
            Some("current directory only"),
        );

        ground_filesystem_mutation(
            &mut value,
            "rename aish source acceptance 8427 txt to Final Name 8427.txt in downloads",
            &context,
        );

        assert_eq!(value.kind, SemanticPlanKind::ShellCommand);
        let command = value.payload.expect("grounded command");
        assert!(command.contains("aish-source-acceptance-8427.txt"));
        assert!(command.contains("Final Name 8427.txt"));
        assert!(!command.contains("invented"));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn typed_move_requires_one_existing_destination() {
        let (root, context) = fixture();
        let downloads = PathBuf::from(context["known_folders"]["downloads"].as_str().unwrap());
        fs::write(downloads.join("artifact-one.zip"), b"fixture").expect("file");
        let mut value = filesystem_plan(
            FilesystemOperation::Move,
            "artifact one zip",
            Some("missing destination"),
            Some("downloads"),
        );

        ground_filesystem_mutation(
            &mut value,
            "move artifact one zip from downloads to missing destination",
            &context,
        );

        assert_eq!(value.kind, SemanticPlanKind::Clarification);
        assert!(value.payload.is_none());
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn typed_move_uses_directional_known_folder_scopes() {
        let (root, context) = fixture();
        let downloads = PathBuf::from(context["known_folders"]["downloads"].as_str().unwrap());
        let desktop = PathBuf::from(context["known_folders"]["desktop"].as_str().unwrap());
        fs::write(downloads.join("move-source-8427.zip"), b"fixture").expect("file");
        let mut value = filesystem_plan(
            FilesystemOperation::Move,
            r"C:\invented\move_source_8427.zip",
            Some(r"C:\invented\Desktop"),
            Some("current directory only"),
        );

        ground_filesystem_mutation(
            &mut value,
            "move move source 8427 zip from downloads to desktop",
            &context,
        );

        assert_eq!(value.kind, SemanticPlanKind::ShellCommand);
        let command = value.payload.expect("grounded command");
        assert!(command.contains("move-source-8427.zip"));
        assert!(command.contains(&desktop.to_string_lossy().to_string()));
        assert!(!command.contains("invented"));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn typed_move_distinguishes_the_source_from_a_named_destination() {
        let (root, context) = fixture();
        let source = root.join("Move Source 8427.txt");
        let destination = root.join("Orbit-8427");
        fs::write(&source, b"fixture").expect("file");
        fs::write(root.join("source-8427.txt"), b"decoy").expect("decoy");
        fs::create_dir(&destination).expect("destination");
        let mut value = filesystem_plan(
            FilesystemOperation::Move,
            "source-8427.txt",
            Some("Orbit-8427"),
            Some("current directory"),
        );

        ground_filesystem_mutation(
            &mut value,
            "move Move Source 8427.txt into Orbit-8427",
            &context,
        );

        assert_eq!(value.kind, SemanticPlanKind::ShellCommand);
        let command = value.payload.expect("grounded command");
        assert!(command.contains(&source.to_string_lossy().to_string()));
        assert!(command.contains(&destination.to_string_lossy().to_string()));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn typed_copy_preserves_the_existing_source_and_destination() {
        let (root, context) = fixture();
        let downloads = PathBuf::from(context["known_folders"]["downloads"].as_str().unwrap());
        let desktop = PathBuf::from(context["known_folders"]["desktop"].as_str().unwrap());
        fs::create_dir(downloads.join("Copy Source 8427")).expect("folder");
        let mut value = filesystem_plan(
            FilesystemOperation::Copy,
            "copy_source_8427",
            Some("desktop"),
            Some("downloads"),
        );

        ground_filesystem_mutation(
            &mut value,
            "copy Copy Source 8427 from downloads to desktop",
            &context,
        );

        assert_eq!(value.kind, SemanticPlanKind::ShellCommand);
        let command = value.payload.expect("grounded command");
        assert!(command.contains("Copy Source 8427"));
        assert!(command.contains(&desktop.to_string_lossy().to_string()));
        assert!(command.contains("AiSH verified the requested filesystem change."));
        assert!(command.contains(
            &desktop
                .join("Copy Source 8427")
                .to_string_lossy()
                .to_string()
        ));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn typed_copy_accepts_a_new_name_in_the_verified_source_parent() {
        let (root, context) = fixture();
        let downloads = PathBuf::from(context["known_folders"]["downloads"].as_str().unwrap());
        fs::write(downloads.join("Cargo.toml"), b"fixture").expect("file");
        let mut value = filesystem_plan(
            FilesystemOperation::Copy,
            "Cargo.toml",
            Some("Cargo-copy.toml"),
            Some("downloads"),
        );

        ground_filesystem_mutation(
            &mut value,
            "copy Cargo.toml to Cargo-copy.toml in downloads",
            &context,
        );

        assert_eq!(value.kind, SemanticPlanKind::ShellCommand);
        let command = value.payload.expect("grounded command");
        assert!(command.contains("Cargo.toml"));
        assert!(command.contains("Cargo-copy.toml"));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn typed_copy_refuses_an_existing_file_destination() {
        let (root, context) = fixture();
        let downloads = PathBuf::from(context["known_folders"]["downloads"].as_str().unwrap());
        fs::write(downloads.join("source.txt"), b"source").expect("file");
        fs::write(downloads.join("destination.txt"), b"destination").expect("file");
        let mut value = filesystem_plan(
            FilesystemOperation::Copy,
            "source.txt",
            Some("destination.txt"),
            Some("downloads"),
        );

        ground_filesystem_mutation(
            &mut value,
            "copy source.txt to destination.txt in downloads",
            &context,
        );

        assert_eq!(
            value.kind,
            SemanticPlanKind::Clarification,
            "unexpected grounded plan: {value:?}"
        );
        assert!(value.payload.is_none());
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn typed_mutations_append_host_owned_state_verification() {
        let (root, context) = fixture();
        let downloads = PathBuf::from(context["known_folders"]["downloads"].as_str().unwrap());
        let source = downloads.join("verify-source.txt");
        fs::write(&source, b"fixture").expect("file");
        let mut value = filesystem_plan(
            FilesystemOperation::Rename,
            "verify source txt",
            Some("Verified Destination.txt"),
            Some("downloads"),
        );

        ground_filesystem_mutation(
            &mut value,
            "rename verify source txt to Verified Destination.txt in downloads",
            &context,
        );

        let command = value.payload.expect("grounded command");
        let steps = crate::split_planned_commands(&command);
        assert_eq!(steps.len(), 2);
        assert!(steps[0].contains("verify-source.txt"));
        assert!(steps[1].contains("verify-source.txt"));
        assert!(steps[1].contains("Verified Destination.txt"));
        assert!(steps[1].contains("AiSH verified the requested filesystem change."));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn typed_write_preserves_content_and_quotes_the_requested_file_name() {
        let (root, context) = fixture();
        let mut value = SemanticPlan {
            kind: SemanticPlanKind::FilesystemAction,
            payload: Some("model changed this".to_string()),
            target: Some("Result_File.txt".to_string()),
            scope: Some("current directory".to_string()),
            message: None,
            operation: Some(FilesystemOperation::WriteFile),
            destination: None,
        };

        ground_filesystem_mutation(
            &mut value,
            "write hello dynamic sandbox into Result File.txt",
            &context,
        );

        assert_eq!(value.kind, SemanticPlanKind::ShellCommand);
        let command = value.payload.expect("grounded command");
        assert!(command.contains("Result File.txt"));
        assert!(command.contains("hello dynamic sandbox"));
        assert!(!command.contains("Result_File.txt"));
        assert!(!command.contains("model changed this"));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn multiple_named_creations_become_ordered_verified_steps() {
        let (root, context) = fixture();
        let mut value = filesystem_plan(
            FilesystemOperation::CreateDirectory,
            "Batch 8427",
            None,
            Some("current directory"),
        );

        ground_filesystem_mutation(
            &mut value,
            "create a folder named Batch 8427 and then create an empty file named Created Together 8427.txt inside it",
            &context,
        );

        assert_eq!(value.kind, SemanticPlanKind::ShellCommand);
        let command = value.payload.expect("grounded command");
        let folder = root.join("Batch 8427");
        let file = folder.join("Created Together 8427.txt");
        assert!(command.contains(&folder.to_string_lossy().to_string()));
        assert!(command.contains(&file.to_string_lossy().to_string()));
        assert_eq!(
            command
                .matches("AiSH verified the requested filesystem change.")
                .count(),
            2
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn deletion_verification_checks_that_the_grounded_item_is_absent() {
        let (root, context) = fixture();
        let downloads = PathBuf::from(context["known_folders"]["downloads"].as_str().unwrap());
        fs::write(downloads.join("remove-and-verify.txt"), b"fixture").expect("file");
        let mut value = filesystem_plan(
            FilesystemOperation::Delete,
            "remove and verify txt",
            None,
            Some("downloads"),
        );

        ground_filesystem_mutation(
            &mut value,
            "delete remove and verify txt in downloads",
            &context,
        );

        let command = value.payload.expect("grounded command");
        let steps = crate::split_planned_commands(&command);
        assert_eq!(steps.len(), 2);
        assert!(steps[0].contains("remove-and-verify.txt"));
        assert!(steps[1].contains("remove-and-verify.txt"));
        #[cfg(windows)]
        assert!(steps[1].contains("if (Test-Path"));
        #[cfg(not(windows))]
        assert!(steps[1].contains("test ! -e"));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn typed_actions_must_match_state_changing_request_intent() {
        assert!(filesystem_operation_matches_request(
            Some(&FilesystemOperation::Rename),
            "rename the report"
        ));
        assert!(filesystem_operation_matches_request(
            Some(&FilesystemOperation::Move),
            "move report.txt into archive"
        ));
        assert!(!filesystem_operation_matches_request(
            Some(&FilesystemOperation::Move),
            "move one directory up"
        ));
        assert!(!filesystem_operation_matches_request(
            Some(&FilesystemOperation::Delete),
            "find the largest folders"
        ));
        assert!(!filesystem_operation_matches_request(
            Some(&FilesystemOperation::CreateDirectory),
            "show hidden files here"
        ));
    }
}
