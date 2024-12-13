/*
    This code is a derivative work based on .NET Standard Library source code

    All original attributions and licenses apply to this work.

    Adapter: Caiyi Shyu <cai1hsu@outlook.com>

    Source: https://source.dot.net/#System.Private.CoreLib/src/libraries/System.Private.CoreLib/src/System/IO/Path.cs
*/

extern crate alloc;

use core::cmp::max;

use alloc::format;
use alloc::string::{String, ToString};

// Root directory
pub const ROOT_STR: &str = "/";
// Directory separator character in &str
pub const SEPARATOR_STR: &str = "/";
// dot character in &str
pub const DOT_STR: &str = ".";

// Directory separator character
pub const SEPARATOR: char = '/';
// dot character
pub const DOT: char = '.';

/// Combine two paths
pub fn combine(path1: &str, path2: &str) -> Option<String> {
    combine_internal(path1, path2)
}

/// Determine if a character is a separator
pub fn is_separator(c: char) -> bool {
    c == SEPARATOR
}

/// Determine if a path is fully qualified
pub fn is_path_rooted(path: &str) -> bool {
    starts_with_separator(path)
}

/// Determine if path ends with separator
pub fn ends_in_separator(path: &str) -> bool {
    !path.is_empty() && path.chars().last().unwrap() == SEPARATOR
}

/// Determine if path starts with separator
pub fn starts_with_separator(path: &str) -> bool {
    !path.is_empty() && path.chars().next().unwrap() == SEPARATOR
}

/// Remove the trailing separator of a path
/// Returns a slice of previous &str
pub fn trim_end_separator(path: &str) -> &str {
    match ends_in_separator(path) && !is_root_internal(path) {
        true => &path[..path.len() - 1],
        false => path,
    }
}

/// Determine if a path is root
/// This method doesn't handle non-simplified paths
pub fn is_root(path: &str) -> bool {
    !path.is_empty() && is_root_internal(path)
}

/// Determine if a path is fully qualified
pub fn is_path_fully_qualified(path: &str) -> bool {
    !is_partially_qualified(path)
}

/// Determine if a path has extension
/// Directory or empty paths are considered false
pub fn has_extension(path: &str) -> bool {
    path.chars()
        .rev()
        .take_while(|c| !is_separator(*c))
        .any(|c| c == DOT)
}

/// Get the extensions of a path
/// Returns None if the path does not have a extension
/// or if the path is empty or a directory
pub fn get_extension(path: &str) -> Option<&str> {
    let mut iter = path.chars().rev().take_while(|c| !is_separator(*c));
    let dot = iter.position(|c| c == DOT);

    match dot {
        Some(dot) => Some(&path[path.len() - dot..]),
        None => None,
    }
}

/// Get the last component of the path, may be file name or the last directory's name
/// Returns the full path if not found
pub fn get_filename(path: &str) -> &str {
    match index_of_filename(path) {
        ..=0 => path,
        pos => &path[pos..],
    }
}

/// Get the file name without extension
pub fn get_filename_without_extension(path: &str) -> &str {
    let filename = get_filename(path);

    match filename.rfind(DOT) {
        Some(dot) => &filename[..dot],
        None => filename,
    }
}

/// Change the file name of the path, keeps the directory part
/// Will return None if the path is empty.
pub fn change_extension(path: &str, extension: &str) -> Option<String> {
    if path.is_empty() {
        return None;
    }

    let filename = get_filename_without_extension(path);
    let directory = get_directory_name(path);

    let changed = match extension.is_empty() {
        true => filename.to_string(),
        false => format!("{}{}{}", filename, DOT, extension),
    };

    match directory {
        Some(directory) => combine(directory, &changed),
        None => Some(changed),
    }
}

/// Returns "/" if the path is not empty
pub fn get_path_root(path: &str) -> Option<&str> {
    match path.is_empty() || !is_path_rooted(path) {
        true => None,
        false => Some(ROOT_STR),
    }
}

/// Get the parent directory of a path
/// Returns None for "/" or empty path
pub fn get_directory_name(path: &str) -> Option<&str> {
    match path.is_empty() {
        true => None,
        false => {
            let end = get_directory_name_offset(path);

            if end < 0 {
                return None;
            }

            Some(&path[..end as usize])
        }
    }
}

/// Calculate relative form of the second path relative to the first path
/// Replace the relative segaments with "." or ".."
/// Returns None if the two paths are both empty or partially qualified.
pub fn get_relative_path(relative_to: &str, path: &str) -> Option<String> {
    match relative_to.is_empty()
        || path.is_empty()
        || is_partially_qualified(relative_to)
        || is_partially_qualified(path)
    {
        true => None,
        false => get_relative_path_internal(relative_to, path),
    }
}

/// Get the path of the first directory relative to the second directory
/// Returns None if the two paths are both empty or partially qualified.
/// Like combine or remove_relative_segments depending on if the path is rooted or not
pub fn get_full_path(path: &str, cwd: Option<&str>) -> Option<String> {
    match path.is_empty() {
        true => cwd.map(|c| c.to_string()),
        false => get_full_path_internal(path, cwd),
    }
}

/// Remove as many relative segaments as possible of a path
pub fn remove_relative_segments(path: &str) -> String {
    match remove_relative_segments_internal(path) {
        Some(p) => p,
        // TODO: Optimize this, this is not needed
        None => path.to_string(),
    }
}

/// Get relative to root form of the path
pub fn relative_to_root(path: &str) -> &str {
    match is_path_fully_qualified(path) {
        false => path,
        true => &path[get_root_length(path)..],
    }
}

fn is_root_internal(path: &str) -> bool {
    path.len() == get_root_length(path)
}

fn get_full_path_internal(path: &str, cwd: Option<&str>) -> Option<String> {
    match is_path_rooted(path) {
        true => {
            let collapsed = remove_relative_segments(path);

            match collapsed.len() {
                0 => Some(ROOT_STR.to_string()),
                _ => Some(collapsed),
            }
        }
        false => match cwd {
            None => Some(path.to_string()),
            Some(cwd) => combine(cwd, path),
        },
    }
}

fn remove_relative_segments_internal(path: &str) -> Option<String> {
    let root_length = get_root_length(path);
    let mut sb = String::with_capacity(path.len());

    let mut skip = root_length;
    let path_len = path.len();

    // We treat "\.." , "\." and "\\" as a relative segment. We want to collapse the first separator past the root presuming
    // the root actually ends in a separator. Otherwise the first segment for RemoveRelativeSegments
    // in cases like "\\?\C:\.\" and "\\?\C:\..\", the first segment after the root will be ".\" and "..\" which is not considered as a relative segment and hence not be removed.
    // Since the root on unix is only one character, we only have to check the first character.
    if root_length == 1 && is_separator(path.chars().next().unwrap()) {
        skip -= 1;
    }

    // Remove "//", "/./", and "/../" from the path by copying each character to the output,
    // except the ones we're removing, such that the builder contains the normalized path
    // at the end.
    if skip > 0 {
        sb.push_str(&path[0..skip]);
    }

    let mut enu = path.chars().skip(skip).enumerate();

    while let Some((i, c)) = enu.next() {
        if is_separator(c) && i + 1 < path_len {
            let mut cloned_it = enu.clone();
            let next = cloned_it.next().unwrap().1;

            // Skip this character if it's a directory separator and if the next character is, too,
            // e.g. "parent//child" => "parent/child"
            // path[i + 1] == '/'
            if is_separator(next) {
                continue;
            }

            let next_next = cloned_it.next().unwrap().1;

            // Skip this character and the next if it's referring to the current directory,
            // e.g. "parent/./child" => "parent/child"
            if (i + 2 == path_len || is_separator(next_next)) && next == DOT {
                // skip the next dot, and `continue` skips the slash
                enu.next();
                continue;
            }

            let next3 = cloned_it.next().unwrap().1;

            // Skip this character and the next two if it's referring to the parent directory,
            // e.g. "parent/child/../grandchild" => "parent/grandchild"
            if i + 2 < path_len
                && (i + 3 == path_len || is_separator(next3))
                && next == DOT
                && next_next == DOT
            {
                let mut si = sb.len();
                for c in sb.chars().rev() {
                    si -= 1;

                    if si < skip {
                        break;
                    }

                    if is_separator(c) {
                        let new_len = match i + 3 >= path_len && si == skip {
                            true => si + 1,
                            false => si,
                        };

                        sb.truncate(new_len);
                        break;
                    }
                }

                if si < skip {
                    sb.truncate(skip);
                }

                // skip the next 2 dots, and `continue` skips the slash
                enu.next();
                enu.next();
                continue;
            }
        }

        sb.push(c);
    }

    let sb_len = sb.len();

    // If we haven't changed the source path, return the original
    if sb_len == path_len {
        return None;
    }

    // We may have eaten the trailing separator from the root when we started and not replaced it
    if skip != root_length && sb_len < root_length && root_length == 1 {
        sb.push(SEPARATOR);
    }

    Some(sb)
}

/// Remove alternate directory separator('//' or '\\')
pub fn normalize_path(path: &str) -> Option<String> {
    if path.is_empty() {
        return None;
    }

    let mut normalized = false;

    let mut it = path.chars();
    while let Some(c) = it.next() {
        // path[i] == '/' && path[i + 1] == '/'
        if is_separator(c) {
            if let Some(next) = it.next() {
                if is_separator(next) {
                    normalized = false;
                    break;
                }
            }
        }
    }

    if normalized {
        return Some(path.to_string());
    }

    let mut result = String::with_capacity(path.len());

    let mut it = path.chars();
    while let Some(c) = it.next() {
        if is_separator(c) {
            if let Some(next) = it.next() {
                if is_separator(next) {
                    continue;
                }
            }
        }

        result.push(c);
    }

    Some(result)
}

fn get_directory_name_offset(path: &str) -> isize {
    let len = path.len();
    let root_len = get_root_length(path);

    match len <= root_len {
        true => -1,
        false => {
            let mut it = path.chars().rev();
            let mut end = len - 1;

            for c in it.by_ref() {
                match end > root_len && !is_separator(c) {
                    true => end -= 1,
                    false => break,
                }
            }

            // Handle alternate directory separator('//' or '\\')
            it.next();
            for c in it.by_ref() {
                match end > root_len && is_separator(c) {
                    true => end -= 1,
                    false => break,
                }
            }

            end as isize
        }
    }
}

fn index_of_filename(path: &str) -> usize {
    match path.is_empty() {
        true => 0,
        false => match path.chars().rev().position(is_separator) {
            Some(pos) => path.len() - pos,
            None => 0,
        },
    }
}

fn is_partially_qualified(path: &str) -> bool {
    !is_path_rooted(path)
}

fn get_root_length(path: &str) -> usize {
    match starts_with_separator(path) {
        true => 1,
        false => 0,
    }
}

fn get_relative_path_internal(relative_to: &str, path: &str) -> Option<String> {
    let mut common_len = get_common_length(relative_to, path);

    if common_len == 0 {
        return Some(path.to_string());
    }

    // Trailing separators aren't significant for comparison
    let relative_to_len = effective_length(relative_to);
    let path_len = effective_length(path);

    // If we have effectively the same path, return "."
    if relative_to_len == path_len && common_len >= relative_to_len {
        return Some(DOT_STR.to_string());
    }

    // We have the same root, we need to calculate the difference now using the
    // common Length and Segment count past the length.
    //
    // Some examples:
    //
    //  C:\Foo C:\Bar L3, S1 -> ..\Bar
    //  C:\Foo C:\Foo\Bar L6, S0 -> Bar
    //  C:\Foo\Bar C:\Bar\Bar L3, S2 -> ..\..\Bar\Bar
    //  C:\Foo\Foo C:\Foo\Bar L7, S1 -> ..\Bar

    let mut sb = String::with_capacity(max(relative_to.len(), path.len()));

    // Add parent segments for segments past the common on the "from" path
    if common_len < relative_to_len {
        sb.push_str("..");

        for c in relative_to.chars().skip(common_len + 1) {
            if is_separator(c) {
                sb.push(SEPARATOR);
                sb.push_str("..");
            }
        }
    } else if is_separator(path.chars().nth(common_len).unwrap()) {
        // No parent segments and we need to eat the initial separator
        //  (C:\Foo C:\Foo\Bar case)
        common_len += 1;
    }

    // Now add the rest of the "to" path, adding back the trailing separator
    let mut diff_len = path_len - common_len;
    if ends_in_separator(path) {
        diff_len += 1;
    }

    if diff_len > 0 {
        if !sb.is_empty() {
            sb.push(SEPARATOR);
        }

        sb.push_str(&path[common_len..common_len + diff_len]);
    }

    Some(sb)
}

fn effective_length(path: &str) -> usize {
    let len = path.len();

    match ends_in_separator(path) {
        true => len - 1,
        false => len,
    }
}

pub fn get_common_length(first: &str, second: &str) -> usize {
    let mut common_chars = equal_starting_character_count(first, second);

    if common_chars == 0 {
        return 0;
    }

    let first_len = first.len();
    let second_len = second.len();

    if common_chars == first_len && common_chars == second_len
        || is_separator(second.chars().nth(common_chars).unwrap())
    {
        return common_chars;
    }

    if common_chars == second_len && is_separator(first.chars().nth(common_chars).unwrap()) {
        return common_chars;
    }

    let mut it = first.chars().rev().skip(first_len - common_chars + 1);

    while common_chars > 0 {
        match it.next() {
            None => break,
            Some(c) => {
                if is_separator(c) {
                    common_chars -= 1;
                }
            }
        }
    }

    common_chars
}

fn equal_starting_character_count(first: &str, second: &str) -> usize {
    if first.is_empty() || second.is_empty() {
        return 0;
    }

    let mut common_len = 0;
    let mut first_it = first.chars();
    let mut second_it = second.chars();

    while let (Some(a_char), Some(b_char)) = (first_it.next(), second_it.next()) {
        if a_char == b_char {
            common_len += 1;
        } else {
            break;
        }
    }

    common_len
}

fn combine_internal(first: &str, second: &str) -> Option<String> {
    if first.is_empty() {
        return Some(second.to_string());
    }

    if second.is_empty() {
        return Some(first.to_string());
    }

    match is_path_rooted(second) {
        true => Some(second.to_string()),
        false => join_internal(first, second),
    }
}

fn join_internal(first: &str, second: &str) -> Option<String> {
    assert!(!first.is_empty());
    assert!(!second.is_empty());

    let has_separator =
        is_separator(first.chars().last().unwrap()) || is_separator(second.chars().next().unwrap());

    match has_separator {
        true => Some(format!("{}{}", first, second)),
        false => Some(format!("{}{}{}", first, SEPARATOR, second)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_combine() {
        assert_eq!(
            combine("/home/user", "docs"),
            Some("/home/user/docs".to_string())
        );
        assert_eq!(combine("", "docs"), Some("docs".to_string()));
        assert_eq!(combine("/home/user", ""), Some("/home/user".to_string()));
        assert_eq!(combine("", ""), Some("".to_string()));
        assert_eq!(combine("/", "docs"), Some("/docs".to_string()));
        assert_eq!(combine("/home/user", "/docs"), Some("/docs".to_string()));
        assert_eq!(combine("/home/user/", "/docs"), Some("/docs".to_string()));
    }

    #[test]
    fn test_ends_in_separator() {
        assert!(ends_in_separator("/home/user/"));
        assert!(!ends_in_separator("/home/user"));
        assert!(ends_in_separator("/"));
        assert!(!ends_in_separator(""));
    }

    #[test]
    fn test_starts_with_separator() {
        assert!(starts_with_separator("/home/user"));
        assert!(!starts_with_separator("home/user"));
        assert!(starts_with_separator("/"));
        assert!(!starts_with_separator(""));
    }

    #[test]
    fn test_trim_end_separator() {
        assert_eq!(trim_end_separator("/home/user/"), "/home/user");
        assert_eq!(trim_end_separator("/home/user"), "/home/user");
        assert_eq!(trim_end_separator("/"), "/");
        assert_eq!(trim_end_separator(""), "");
    }

    #[test]
    fn test_is_root() {
        assert!(is_root_internal("/"));
        assert!(!is_root_internal("/home/user"));
    }

    #[test]
    fn test_is_path_fully_qualified() {
        assert!(is_path_fully_qualified("/home/user"));
        assert!(!is_path_fully_qualified("home/user"));
        assert!(is_path_fully_qualified("/"));
        assert!(!is_path_fully_qualified(""));
    }

    #[test]
    fn test_has_extension() {
        assert!(has_extension("/home/user/file.txt"));
        assert!(!has_extension("/home/user/file"));
        assert!(!has_extension("/home/user/"));
        assert!(!has_extension(""));
    }

    #[test]
    fn test_get_extension() {
        assert_eq!(get_extension("/home/user/file.txt"), Some("txt"));
        assert_eq!(get_extension("/home/user/file"), None);
        assert_eq!(get_extension("/home/user/"), None);
        assert_eq!(get_extension(""), None);
    }

    #[test]
    fn test_get_filename() {
        assert_eq!(get_filename("/home/user/file.txt"), "file.txt");
        assert_eq!(get_filename("/home/user/"), "");
        assert_eq!(get_filename("/"), "");
        assert_eq!(get_filename(""), "");
    }

    #[test]
    fn test_get_filename_without_extension() {
        assert_eq!(
            get_filename_without_extension("/home/user/file.txt"),
            "file"
        );
        assert_eq!(get_filename_without_extension("/home/user/file"), "file");
        assert_eq!(get_filename_without_extension("/home/user/"), "");
        assert_eq!(get_filename_without_extension(""), "");
    }

    #[test]
    fn test_change_extension() {
        assert_eq!(
            change_extension("/home/user/file.txt", "md"),
            Some("/home/user/file.md".to_string())
        );
        assert_eq!(
            change_extension("/home/user/file", "md"),
            Some("/home/user/file.md".to_string())
        );
        assert_eq!(
            change_extension("/home/user/file.txt", ""),
            Some("/home/user/file".to_string())
        );
        assert_eq!(
            change_extension("/home/user/file.txt", "tar.gz"),
            Some("/home/user/file.tar.gz".to_string())
        );
    }

    #[test]
    fn test_get_path_root() {
        assert_eq!(get_path_root("/home/user"), Some("/"));
        assert_eq!(get_path_root("home/user"), None);
        assert_eq!(get_path_root("/"), Some("/"));
        assert_eq!(get_path_root(""), None);
    }

    #[test]
    fn test_get_directory_name() {
        assert_eq!(
            get_directory_name("/home/user/file.txt"),
            Some("/home/user")
        );
        assert_eq!(get_directory_name("/file.txt"), Some("/"));
        assert_eq!(get_directory_name("/"), None);
        assert_eq!(get_directory_name(""), None);
    }

    #[test]
    fn test_get_relative_path() {
        assert_eq!(
            get_relative_path("/home/user", "/home/user/docs/file.txt"),
            Some("docs/file.txt".to_string())
        );
        assert_eq!(
            get_relative_path("/", "/home/user/docs/file.txt"),
            Some("home/user/docs/file.txt".to_string())
        );
        assert_eq!(get_relative_path("/", "/"), Some(".".to_string()));
        assert_eq!(
            get_relative_path("/home/user", "/docs/file.txt"),
            Some("../../docs/file.txt".to_string())
        );
    }

    #[test]
    fn test_get_full_path() {
        assert_eq!(
            get_full_path("file.txt", Some("/home/user")),
            Some("/home/user/file.txt".to_string())
        );
        assert_eq!(
            get_full_path("/docs/file.txt", Some("/home/user")),
            Some("/docs/file.txt".to_string())
        );
        assert_eq!(
            get_full_path("file.txt", None),
            Some("file.txt".to_string())
        );
        assert_eq!(
            get_full_path("", Some("/home/user")),
            Some("/home/user".to_string())
        );
    }

    #[test]
    fn test_remove_relative_segments() {
        assert_eq!(remove_relative_segments("/home/user/../docs"), "/home/docs");
        assert_eq!(
            remove_relative_segments("/home/./user/docs"),
            "/home/user/docs"
        );
        assert_eq!(remove_relative_segments("/../home/user"), "/home/user");
        assert_eq!(remove_relative_segments("/home/user/../../docs"), "/docs");
    }
}
