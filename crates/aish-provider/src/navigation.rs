use std::collections::VecDeque;
use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};

const MAX_VISITED: usize = 4096;
const MAX_DEPTH: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavigationResolution {
    Resolved(PathBuf),
    Ambiguous(Vec<PathBuf>),
    Missing(String),
}

pub fn infer_existing_target_from_request(request: &str, cwd: &Path) -> Option<String> {
    let mut queue = VecDeque::from([(cwd.to_path_buf(), 0_usize)]);
    let mut visited = 0_usize;
    let mut names = Vec::new();
    while let Some((directory, depth)) = queue.pop_front() {
        if visited >= MAX_VISITED || depth > MAX_DEPTH {
            break;
        }
        visited += 1;
        let Ok(entries) = fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.chars().count() >= 2 && request_contains_name(request, &name) {
                push_unique_name(&mut names, name);
            }
            if depth < MAX_DEPTH {
                queue.push_back((path, depth + 1));
            }
        }
    }
    names.sort_by_key(|name| std::cmp::Reverse(name.chars().count()));
    let longest = names.first()?.chars().count();
    let longest_names = names
        .into_iter()
        .take_while(|name| name.chars().count() == longest)
        .collect::<Vec<_>>();
    (longest_names.len() == 1).then(|| longest_names[0].clone())
}

pub fn resolve_navigation_target(
    target: &str,
    scope: Option<&str>,
    request: &str,
    cwd: &Path,
    home: &Path,
) -> NavigationResolution {
    let target = unquote(target.trim());
    if target.is_empty() {
        return NavigationResolution::Missing("No directory target was supplied.".to_string());
    }

    let expanded = expand_environment(&target, home);
    let candidate = PathBuf::from(&expanded);
    let nearest_requested = scope.is_some_and(|value| value.trim().eq_ignore_ascii_case("nearest"))
        || request_requests_nearest(request);
    if candidate.is_absolute()
        || has_path_separator(&expanded)
        || expanded == "."
        || expanded == ".."
    {
        let rooted = if candidate.is_absolute() {
            candidate.clone()
        } else {
            cwd.join(&candidate)
        };
        if let Some(existing) = existing_directory(&rooted) {
            return NavigationResolution::Resolved(existing);
        }
        if nearest_requested {
            let name = candidate
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or(&target);
            return resolve_nearest(name, cwd);
        }
        return NavigationResolution::Missing(format!("Directory not found: {target}"));
    }

    let normalized_scope = scope.map(str::trim).filter(|value| !value.is_empty());
    if nearest_requested {
        return resolve_nearest(&target, cwd);
    }

    let mut roots = Vec::new();
    let mut effective_scope = normalized_scope.is_some();
    if let Some(scope) = normalized_scope {
        if scope.eq_ignore_ascii_case("home") {
            roots.push(home.to_path_buf());
        } else if ["current", "cwd", "relative", "here"]
            .iter()
            .any(|value| scope.eq_ignore_ascii_case(value))
        {
            roots.push(cwd.to_path_buf());
        } else if let Some(root) = drive_root(scope) {
            roots.push(root);
        } else {
            let expanded_scope = expand_environment(scope, home);
            let path = PathBuf::from(expanded_scope);
            let rooted = if path.is_absolute() {
                path
            } else {
                cwd.join(path)
            };
            if rooted.is_dir() || scope_is_grounded_in_request(scope, request) {
                roots.push(rooted);
            } else {
                effective_scope = false;
                roots.push(cwd.to_path_buf());
                if home != cwd {
                    roots.push(home.to_path_buf());
                }
            }
        }
    } else {
        roots.push(cwd.to_path_buf());
        if home != cwd {
            roots.push(home.to_path_buf());
        }
    }

    let mut direct = unique_existing_children(&roots, &target);
    if direct.len() == 1 {
        return NavigationResolution::Resolved(direct.remove(0));
    }
    if direct.len() > 1 {
        return NavigationResolution::Ambiguous(direct);
    }

    let allow_recursive = effective_scope;
    if allow_recursive {
        let matches = bounded_directory_search(&roots, &target, false);
        if matches.len() == 1 {
            return NavigationResolution::Resolved(matches[0].clone());
        }
        if matches.len() > 1 {
            return NavigationResolution::Ambiguous(matches);
        }
    }

    let fuzzy = fuzzy_children(&roots, &target);
    if fuzzy.len() == 1 {
        NavigationResolution::Resolved(fuzzy[0].clone())
    } else if fuzzy.len() > 1 {
        NavigationResolution::Ambiguous(fuzzy)
    } else {
        NavigationResolution::Missing(format!(
            "I could not find an existing directory matching '{target}'."
        ))
    }
}

fn request_contains_name(request: &str, name: &str) -> bool {
    let request = request.to_lowercase();
    let name = name.to_lowercase();
    request.match_indices(&name).any(|(start, matched)| {
        let before = request[..start].chars().next_back();
        let after = request[start + matched.len()..].chars().next();
        before.map_or(true, |character| !character.is_alphanumeric())
            && after.map_or(true, |character| !character.is_alphanumeric())
    })
}

fn push_unique_name(names: &mut Vec<String>, candidate: String) {
    if !names.iter().any(|existing| {
        if case_insensitive_platform() {
            existing.eq_ignore_ascii_case(&candidate)
        } else {
            existing == &candidate
        }
    }) {
        names.push(candidate);
    }
}

fn request_requests_nearest(request: &str) -> bool {
    request.split_whitespace().any(|word| {
        matches!(
            word.trim_matches(|character: char| !character.is_alphanumeric())
                .to_ascii_lowercase()
                .as_str(),
            "nearest" | "closest" | "nearby"
        )
    })
}

fn scope_is_grounded_in_request(scope: &str, request: &str) -> bool {
    let request = request.to_lowercase();
    let scope = unquote(scope.trim()).to_lowercase();
    if request.contains(&scope) {
        return true;
    }

    Path::new(&scope)
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| !name.is_empty() && request.contains(name))
}

fn resolve_nearest(target: &str, cwd: &Path) -> NavigationResolution {
    let ancestors = cwd.ancestors().map(Path::to_path_buf).collect::<Vec<_>>();
    let direct = unique_existing_children(&ancestors, target);
    if direct.len() == 1 {
        return NavigationResolution::Resolved(direct[0].clone());
    }
    if direct.len() > 1 {
        let shortest = direct
            .iter()
            .map(|path| path.components().count())
            .max()
            .unwrap_or_default();
        let closest = direct
            .into_iter()
            .filter(|path| path.components().count() == shortest)
            .collect::<Vec<_>>();
        return if closest.len() == 1 {
            NavigationResolution::Resolved(closest[0].clone())
        } else {
            NavigationResolution::Ambiguous(closest)
        };
    }

    let matches = bounded_directory_search(&[cwd.to_path_buf()], target, true);
    if matches.len() == 1 {
        NavigationResolution::Resolved(matches[0].clone())
    } else if matches.len() > 1 {
        NavigationResolution::Ambiguous(matches)
    } else {
        NavigationResolution::Missing(format!(
            "I could not find a nearby directory matching '{target}'."
        ))
    }
}

fn unique_existing_children(roots: &[PathBuf], name: &str) -> Vec<PathBuf> {
    let mut matches = Vec::new();
    for root in roots {
        if let Some(path) = matching_child(root, name) {
            push_unique(&mut matches, path);
        }
    }
    matches
}

fn matching_child(root: &Path, name: &str) -> Option<PathBuf> {
    let exact = root.join(name);
    if exact.is_dir() {
        return fs::canonicalize(&exact).ok().or(Some(exact));
    }
    if !case_insensitive_platform() {
        return None;
    }
    fs::read_dir(root).ok()?.flatten().find_map(|entry| {
        let path = entry.path();
        (path.is_dir()
            && entry
                .file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case(name))
        .then(|| fs::canonicalize(&path).unwrap_or(path))
    })
}

fn bounded_directory_search(
    roots: &[PathBuf],
    name: &str,
    stop_at_first_depth: bool,
) -> Vec<PathBuf> {
    let mut queue = VecDeque::new();
    for root in roots {
        queue.push_back((root.clone(), 0_usize));
    }
    let mut visited = 0_usize;
    let mut found_depth = None;
    let mut matches = Vec::new();
    while let Some((directory, depth)) = queue.pop_front() {
        if visited >= MAX_VISITED
            || depth > MAX_DEPTH
            || found_depth.is_some_and(|value| depth > value)
        {
            break;
        }
        visited += 1;
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let is_match = if case_insensitive_platform() {
                entry
                    .file_name()
                    .to_string_lossy()
                    .eq_ignore_ascii_case(name)
            } else {
                entry.file_name().to_string_lossy() == name
            };
            if is_match {
                found_depth.get_or_insert(depth);
                push_unique(
                    &mut matches,
                    fs::canonicalize(&path).unwrap_or(path.clone()),
                );
            }
            if depth < MAX_DEPTH && (!stop_at_first_depth || found_depth.is_none()) {
                queue.push_back((path, depth + 1));
            }
        }
    }
    matches
}

fn fuzzy_children(roots: &[PathBuf], target: &str) -> Vec<PathBuf> {
    let threshold = if target.chars().count() <= 5 { 1 } else { 2 };
    let target_lower = target.to_lowercase();
    let mut scored = Vec::new();
    for root in roots {
        let Ok(entries) = fs::read_dir(root) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_lowercase();
            let distance = levenshtein(&target_lower, &name);
            if distance <= threshold {
                scored.push((distance, fs::canonicalize(&path).unwrap_or(path)));
            }
        }
    }
    let best = scored.iter().map(|(score, _)| *score).min();
    scored
        .into_iter()
        .filter(|(score, _)| Some(*score) == best)
        .map(|(_, path)| path)
        .collect()
}

fn existing_directory(path: &Path) -> Option<PathBuf> {
    if path.is_dir() {
        return Some(fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf()));
    }
    let parent = path.parent()?;
    let name = path.file_name()?.to_string_lossy();
    matching_child(parent, &name)
}

fn expand_environment(value: &str, home: &Path) -> String {
    let mut output = value.to_string();
    if output == "~" {
        return home.display().to_string();
    }
    if output.starts_with("~/") || output.starts_with("~\\") {
        return home.join(&output[2..]).display().to_string();
    }
    for (name, env_value) in env::vars() {
        for token in [
            format!("%{name}%"),
            format!("$env:{name}"),
            format!("${{{name}}}"),
        ] {
            output = replace_ascii_case(&output, &token, &env_value);
        }
    }
    output
}

fn drive_root(value: &str) -> Option<PathBuf> {
    let trimmed = value.trim().trim_end_matches(['/', '\\']);
    let letter = if trimmed.len() == 1 {
        trimmed.chars().next()
    } else if trimmed.len() == 2 && trimmed.ends_with(':') {
        trimmed.chars().next()
    } else {
        let mut words = trimmed.split_whitespace();
        let first = words.next()?;
        let second = words.next()?;
        if words.next().is_none() && second.eq_ignore_ascii_case("drive") && first.len() == 1 {
            first.chars().next()
        } else {
            None
        }
    }?;
    letter
        .is_ascii_alphabetic()
        .then(|| PathBuf::from(format!("{}:\\", letter.to_ascii_uppercase())))
}

fn has_path_separator(value: &str) -> bool {
    value.contains('/')
        || value.contains('\\')
        || matches!(
            Path::new(value).components().next(),
            Some(Component::ParentDir)
        )
}

fn case_insensitive_platform() -> bool {
    cfg!(windows) || env::var("AISH_CASE_INSENSITIVE_PATHS").ok().as_deref() == Some("1")
}

fn unquote(value: &str) -> String {
    value.trim_matches(['"', '\'']).trim().to_string()
}

fn replace_ascii_case(haystack: &str, needle: &str, replacement: &str) -> String {
    let lower = haystack.to_ascii_lowercase();
    let needle_lower = needle.to_ascii_lowercase();
    let mut result = String::new();
    let mut offset = 0;
    while let Some(index) = lower[offset..].find(&needle_lower) {
        let absolute = offset + index;
        result.push_str(&haystack[offset..absolute]);
        result.push_str(replacement);
        offset = absolute + needle.len();
    }
    result.push_str(&haystack[offset..]);
    result
}

fn push_unique(values: &mut Vec<PathBuf>, value: PathBuf) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

fn levenshtein(left: &str, right: &str) -> usize {
    let mut costs = (0..=right.chars().count()).collect::<Vec<_>>();
    for (i, left_char) in left.chars().enumerate() {
        let mut previous = costs[0];
        costs[0] = i + 1;
        for (j, right_char) in right.chars().enumerate() {
            let old = costs[j + 1];
            costs[j + 1] = if left_char == right_char {
                previous
            } else {
                1 + previous.min(costs[j]).min(old)
            };
            previous = old;
        }
    }
    *costs.last().unwrap_or(&0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEMP_ROOT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    fn temp_root() -> PathBuf {
        let root = env::temp_dir().join(format!(
            "aish-navigation-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            TEMP_ROOT_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn resolves_arbitrary_names_spaces_relative_paths_and_case() {
        env::set_var("AISH_CASE_INSENSITIVE_PATHS", "1");
        let root = temp_root();
        let arbitrary = root.join("Zephyr Work Area");
        fs::create_dir_all(arbitrary.join("nested")).unwrap();
        assert!(matches!(
            resolve_navigation_target("zephyr work area", None, "go there", &root, &root),
            NavigationResolution::Resolved(path) if path == fs::canonicalize(&arbitrary).unwrap()
        ));
        assert!(matches!(
            resolve_navigation_target("Zephyr Work Area/nested", None, "go there", &root, &root,),
            NavigationResolution::Resolved(_)
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reports_ambiguity_and_missing_without_inventing_paths() {
        let root = temp_root();
        fs::create_dir_all(root.join("one").join("artifact-zone")).unwrap();
        fs::create_dir_all(root.join("two").join("artifact-zone")).unwrap();
        assert!(matches!(
            resolve_navigation_target(
                "artifact-zone",
                Some(root.to_str().unwrap()),
                &format!("find artifact-zone under {}", root.display()),
                &root,
                &root,
            ),
            NavigationResolution::Ambiguous(paths) if paths.len() == 2
        ));
        assert!(matches!(
            resolve_navigation_target("does-not-exist", None, "go there", &root, &root),
            NavigationResolution::Missing(_)
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn nearest_search_prefers_the_closest_tree_level() {
        let root = temp_root();
        let cwd = root.join("project").join("src");
        fs::create_dir_all(&cwd).unwrap();
        fs::create_dir_all(root.join("project").join("build-output")).unwrap();
        assert!(matches!(
            resolve_navigation_target(
                "build-output",
                Some("nearest"),
                "enter the nearest build-output",
                &cwd,
                &root,
            ),
            NavigationResolution::Resolved(path) if path.ends_with("build-output")
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn nearest_request_does_not_depend_on_model_scope() {
        let root = temp_root();
        let target = root.join("project").join("generated-nearest");
        fs::create_dir_all(&target).unwrap();
        let resolution = resolve_navigation_target(
            "generated-nearest",
            None,
            "enter the nearest folder called generated-nearest",
            &root,
            &root,
        );
        assert_eq!(
            resolution,
            NavigationResolution::Resolved(fs::canonicalize(&target).unwrap())
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn relative_and_here_scopes_resolve_from_the_current_directory() {
        let root = temp_root();
        let target = root.join("generated-relative");
        fs::create_dir_all(&target).unwrap();

        for scope in ["relative", "here", "current", "cwd"] {
            assert_eq!(
                resolve_navigation_target(
                    "generated-relative",
                    Some(scope),
                    "open generated-relative relative to here",
                    &root,
                    &root,
                ),
                NavigationResolution::Resolved(fs::canonicalize(&target).unwrap())
            );
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn nearest_request_recovers_from_a_nonexistent_guessed_path() {
        let root = temp_root();
        let target = root.join("project").join("generated-nearest");
        fs::create_dir_all(&target).unwrap();
        let guessed = root.join("generated-nearest");
        assert!(matches!(
            resolve_navigation_target(
                &guessed.to_string_lossy(),
                None,
                "find the closest directory named generated-nearest and enter it",
                &root,
                &root,
            ),
            NavigationResolution::Resolved(path) if path == fs::canonicalize(&target).unwrap()
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn infers_arbitrary_existing_names_from_the_request() {
        let root = temp_root();
        fs::create_dir_all(root.join("Orbit-8f31")).unwrap();
        fs::create_dir_all(root.join("Work Area 8f31")).unwrap();
        assert_eq!(
            infer_existing_target_from_request("go to Orbit-8f31", &root).as_deref(),
            Some("Orbit-8f31")
        );
        assert_eq!(
            infer_existing_target_from_request("navigate to \"Work Area 8f31\"", &root).as_deref(),
            Some("Work Area 8f31")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn inferred_names_preserve_ambiguous_filesystem_matches() {
        let root = temp_root();
        fs::create_dir_all(root.join("left").join("Echo-8f31")).unwrap();
        fs::create_dir_all(root.join("right").join("Echo-8f31")).unwrap();
        let target = infer_existing_target_from_request("enter Echo-8f31", &root).unwrap();
        assert!(matches!(
            resolve_navigation_target(
                &target,
                Some("current"),
                "enter Echo-8f31",
                &root,
                &root,
            ),
            NavigationResolution::Ambiguous(paths) if paths.len() == 2
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn ignores_an_ungrounded_nonexistent_model_scope() {
        let root = temp_root();
        let expected = root.join("arbitrary-zone");
        fs::create_dir_all(&expected).unwrap();
        let invented_scope = root.join("invented").join("the-arbitrary-zone");

        assert!(matches!(
            resolve_navigation_target(
                "arbitrary-zone",
                Some(invented_scope.to_str().unwrap()),
                "go to the arbitrary-zone folder",
                &root,
                &root,
            ),
            NavigationResolution::Resolved(path) if path == fs::canonicalize(&expected).unwrap()
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn preserves_an_explicit_nonexistent_user_scope() {
        let root = temp_root();
        fs::create_dir_all(root.join("arbitrary-zone")).unwrap();
        let missing_scope = root.join("absent-scope");
        let request = format!("go to arbitrary-zone under {}", missing_scope.display());

        assert!(matches!(
            resolve_navigation_target(
                "arbitrary-zone",
                Some(missing_scope.to_str().unwrap()),
                &request,
                &root,
                &root,
            ),
            NavigationResolution::Missing(_)
        ));
        fs::remove_dir_all(root).unwrap();
    }
}
