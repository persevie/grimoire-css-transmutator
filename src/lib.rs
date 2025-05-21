use std::{
    collections::{HashMap, HashSet},
    fs::{self},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use cssparser::{Parser, ParserInput, SourcePosition, Token};
use glob::glob;
use grimoire_css_lib::{GrimoireCssError, Spell};
use regex::Regex;
use serde::Serialize;
use serde_json::to_string_pretty;

#[derive(Debug, Serialize)]
struct Transmuted {
    pub scrolls: Vec<TransmutedClass>,
}

#[derive(Debug, Serialize)]
struct TransmutedClass {
    pub name: String,
    pub spells: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oneliner: Option<String>,
}

type TransmutedMap = HashMap<String, HashSet<String>>;

/// Represents the state during CSS parsing.
#[derive(Debug, Default)]
struct ParserState {
    pub raw_classes_spells_map: HashMap<String, Vec<String>>,
    pub current_class: String,
    pub started_media_pos: Option<SourcePosition>,
    pub focus: Vec<String>,
    pub component_and_component_target_map: HashSet<String>,
    pub effects: Vec<String>,
    pub class_started: bool,
    pub focus_delim: String,
    pub effect_started: bool,
    pub colons: Vec<String>,
    pub area: Option<String>,
}

/// Reads and cleans multiple CSS files (paths mode).
fn read_and_clean_files(paths: &[PathBuf]) -> Result<String, GrimoireCssError> {
    let comment_regex = Regex::new(r"(?s)/\*.*?\*/").unwrap();

    let total_size: usize = paths
        .iter()
        .filter_map(|path| fs::metadata(path).ok())
        .map(|metadata| metadata.len() as usize)
        .sum();

    // Allocate with the estimated capacity
    let mut all_contents = String::with_capacity(total_size);

    for path in paths {
        let content = fs::read_to_string(path).map_err(|e| {
            GrimoireCssError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to read '{}': {}", path.display(), e),
            ))
        })?;

        // Process and append in one go to minimize intermediate allocations
        all_contents.push_str(&comment_regex.replace_all(&content, "").replace('"', "'"));
    }

    // Release excess capacity if significant
    if all_contents.capacity() > all_contents.len() * 2 {
        all_contents.shrink_to_fit();
    }

    Ok(all_contents)
}

/// Removes the last character of a string.
fn remove_last_char(s: &str) -> &str {
    s.char_indices()
        .next_back()
        .map(|(i, _)| &s[..i])
        .unwrap_or(s)
}

/// Generates a map of spells based on parser state.
fn generate_spells_map(state: &ParserState) -> TransmutedMap {
    let mut spells_map = HashMap::new();

    for (class, prefixes) in &state.raw_classes_spells_map {
        let mut spells = HashSet::new();

        for prefix in prefixes {
            for component in &state.component_and_component_target_map {
                let spell = if prefix.is_empty() {
                    component.clone()
                } else {
                    format!("{}{}", prefix, component)
                };
                spells.insert(spell);
            }
        }
        spells_map.insert(class.clone(), spells);
    }

    spells_map
}

/// Merges two HashMaps, concatenating values for duplicate keys.
fn merge_maps(map1: &mut TransmutedMap, map2: TransmutedMap) {
    for (key, value) in map2 {
        if let Some(existing_value) = map1.get_mut(&key) {
            existing_value.extend(value);
        } else {
            map1.insert(key, value);
        }
    }
}

/// Processes CSS input and generates raw spells.
fn process_css_into_raw_spells(
    css_input: &str,
    parser_state: &mut ParserState,
) -> Result<TransmutedMap, GrimoireCssError> {
    let mut result: TransmutedMap = HashMap::new();
    let mut parser_input = ParserInput::new(css_input);
    let mut parser = Parser::new(&mut parser_input);

    while let Ok(token) = parser.next() {
        match token {
            Token::Ident(cow_rc_str) => {
                if parser_state.class_started && parser_state.current_class.is_empty() {
                    parser_state.current_class.push_str(cow_rc_str);
                    parser_state.class_started = false;
                } else if !parser_state.focus_delim.is_empty() {
                    let prefix = if parser_state.focus.is_empty() {
                        ""
                    } else {
                        "_"
                    };
                    parser_state.focus.push(format!(
                        "{}{}_{}",
                        prefix, &parser_state.focus_delim, &cow_rc_str
                    ));
                    parser_state.focus_delim.clear();
                } else if parser_state.effect_started {
                    if parser_state.colons.len() > 2 {
                        parser_state.colons = vec![":".to_string(), ":".to_string()]
                    }
                    let focus_item = format!("{}{}", parser_state.colons.join(""), cow_rc_str);
                    parser_state.focus.push(focus_item.clone());
                    parser_state.effects.push(cow_rc_str.to_string());
                    parser_state.effect_started = false;
                    parser_state.colons.clear();

                    if parser_state.current_class.is_empty() {
                        parser_state.current_class.push_str(&focus_item);
                    }
                } else if !parser_state.current_class.is_empty() {
                    parser_state.focus.push(format!("_{}", cow_rc_str));
                } else {
                    // This is a tag selector
                    parser_state.current_class.push_str(cow_rc_str);
                }
            }
            Token::AtKeyword(cow_rc_str) => {
                if cow_rc_str.as_ref() == "media" {
                    parser_state.started_media_pos = Some(parser.position());
                }
            }
            Token::Delim(d) => match d.to_string().as_str() {
                "." => {
                    parser_state.class_started = true;
                    if !parser_state.current_class.is_empty() && parser_state.focus_delim.is_empty()
                    {
                        let focus_str = parser_state.focus.join("").trim().replace(" ", "_");

                        let base_raw_spell = if focus_str.is_empty() {
                            String::new()
                        } else {
                            format!("{{{}}}", focus_str)
                        };

                        parser_state
                            .raw_classes_spells_map
                            .entry(parser_state.current_class.to_owned())
                            .or_default()
                            .push(base_raw_spell.clone());

                        parser_state.focus.clear();
                        parser_state.effects.clear();
                        parser_state.current_class.clear();
                        parser_state.focus_delim.clear();
                    }
                }
                ":" | "::" | ">" | "+" | "~" => parser_state.focus_delim = d.to_string(),
                "*" => {
                    if parser_state.focus.is_empty() {
                        parser_state.focus.push(d.to_string());

                        if parser_state.current_class.is_empty() {
                            parser_state.current_class.push('*');
                        }
                    } else {
                        parser_state.focus_delim = d.to_string();
                    }
                }
                _ => {}
            },
            Token::Colon => {
                parser_state.effect_started = true;
                parser_state.colons.push(":".to_string());
            }
            Token::Comma => {
                if !parser_state.focus.is_empty() {
                    if !parser_state.focus_delim.is_empty() {
                        parser_state.focus.push(parser_state.focus_delim.clone());

                        parser_state.focus_delim.clear();
                    }

                    parser_state.focus.push(",".to_string());
                } else {
                    let focus_str = parser_state.focus.join("").trim().replace(" ", "_");

                    let base_raw_spell = if focus_str.is_empty() {
                        String::new()
                    } else {
                        format!("{{{}}}", focus_str)
                    };

                    parser_state
                        .raw_classes_spells_map
                        .entry(parser_state.current_class.to_owned())
                        .or_default()
                        .push(base_raw_spell.clone());

                    parser_state.focus.clear();
                    parser_state.effects.clear();
                    parser_state.current_class.clear();
                    parser_state.class_started = false;
                    parser_state.focus_delim.clear();
                }
            }
            Token::SquareBracketBlock => {
                let mut squared_focus = "[".to_string();
                let start_pos = parser.position();

                parser
                    .parse_nested_block(|input| {
                        while input.next().is_ok() {}
                        Ok::<(), cssparser::ParseError<'_, ()>>(())
                    })
                    .unwrap();

                let slice = parser.slice_from(start_pos);
                squared_focus.push_str(slice);

                parser_state.focus.push(squared_focus);
            }
            Token::CurlyBracketBlock => {
                if let Some(start_media_pos) = parser_state.started_media_pos {
                    let slice = parser.slice_from(start_media_pos);
                    let trimmed_slice = slice
                        .char_indices()
                        .next_back()
                        .map_or(slice, |(i, _)| &slice[..i])
                        .trim()
                        .replace(" ", "_");

                    parser_state.area = Some(trimmed_slice.to_owned());
                    parser_state.started_media_pos = None;

                    let start_nested_pos = parser.position();
                    parser
                        .parse_nested_block(|input| {
                            while input.next().is_ok() {}
                            Ok::<(), cssparser::ParseError<'_, ()>>(())
                        })
                        .unwrap();

                    let mut state = ParserState {
                        area: parser_state.area.clone(),
                        ..Default::default()
                    };

                    let res = process_css_into_raw_spells(
                        parser.slice_from(start_nested_pos),
                        &mut state,
                    )?;
                    merge_maps(&mut result, res);
                    parser_state.area = None;
                } else {
                    let spell = Spell::new(&parser_state.current_class, &HashSet::new(), &None)?;

                    if spell.is_some() {
                        println!(
                            "This class is already Spell: {:#?}",
                            &parser_state.current_class
                        );
                    } else {
                        let focus_str = parser_state.focus.join("").trim().replace(" ", "_");

                        let mut base_raw_spell = if focus_str.is_empty() {
                            String::new()
                        } else {
                            format!("{{{}}}", focus_str)
                        };

                        if let Some(a) = &parser_state.area {
                            base_raw_spell = format!("{}__{}", a, base_raw_spell);
                        }

                        parser_state
                            .raw_classes_spells_map
                            .entry(parser_state.current_class.to_owned())
                            .or_default()
                            .push(base_raw_spell.clone());

                        parser
                            .parse_nested_block(|input| {
                                let mut start_decl_pos: SourcePosition = input.position();
                                let mut colon_pos: SourcePosition = input.position();

                                while let Ok(inner_token) = input.next() {
                                    match inner_token {
                                        Token::Colon => {
                                            colon_pos = input.position();
                                        }
                                        Token::Semicolon => {
                                            let component = remove_last_char(
                                                input.slice(start_decl_pos..colon_pos),
                                            )
                                            .trim();
                                            let target =
                                                remove_last_char(input.slice_from(colon_pos))
                                                    .trim();

                                            parser_state.component_and_component_target_map.insert(
                                                format!(
                                                    "{}={}",
                                                    component.to_owned(),
                                                    target.to_owned()
                                                )
                                                .replace(" ", "_"),
                                            );

                                            start_decl_pos = input.position();
                                        }
                                        _ => {}
                                    }
                                }
                                Ok::<(), cssparser::ParseError<'_, ()>>(())
                            })
                            .unwrap();

                        merge_maps(&mut result, generate_spells_map(parser_state));
                    }

                    parser_state.raw_classes_spells_map.clear();
                    parser_state.current_class.clear();
                    parser_state.component_and_component_target_map.clear();
                    parser_state.effects.clear();
                    parser_state.focus.clear();
                    parser_state.class_started = false;
                    parser_state.focus_delim.clear();
                }
            }
            Token::Function(t) => {
                if parser_state.effect_started {
                    if parser_state.colons.len() > 2 {
                        parser_state.colons = vec![":".to_string(), ":".to_string()]
                    }

                    let fn_name = t.to_string();

                    let start_pos = parser.position();

                    parser
                        .parse_nested_block(|input| {
                            while input.next().is_ok() {}
                            Ok::<(), cssparser::ParseError<'_, ()>>(())
                        })
                        .unwrap();

                    let slice = parser.slice_from(start_pos);

                    parser_state.focus.push(format!(
                        "{}{}({}",
                        parser_state.colons.join(""),
                        &fn_name,
                        slice
                    ));
                    parser_state.effects.push(fn_name);
                    parser_state.effect_started = false;
                    parser_state.colons.clear();
                }
            }
            _ => {}
        }
    }

    Ok(result)
}

/// Run the transmutation process on multiple CSS files.
/// This is the main entry point for the paths mode.
pub fn run_transmutation(
    args: Vec<String>,
    include_oneliner: bool,
) -> Result<(Duration, String), GrimoireCssError> {
    // Get current directory
    let cwd: PathBuf = std::env::current_dir().map_err(GrimoireCssError::Io)?;

    // Validate input
    if args.is_empty() {
        return Err(GrimoireCssError::InvalidInput(
            "No CSS file patterns provided.".into(),
        ));
    }

    // Expand file paths based on glob patterns
    let expanded_paths = expand_file_paths(&cwd, &args)?;
    if expanded_paths.is_empty() {
        return Err(GrimoireCssError::InvalidPath(
            "No files found matching the provided patterns.".into(),
        ));
    }

    let start_time = Instant::now();

    let mut parser_state = ParserState::default();

    // Read and process CSS files
    let all_css_string = read_and_clean_files(&expanded_paths)?;
    let processed_css = process_css_into_raw_spells(&all_css_string, &mut parser_state)?;

    if processed_css.is_empty() {
        return Err(GrimoireCssError::InvalidInput(
            "There is nothing to transmute.".into(),
        ));
    }

    // Build the transmuted output structure
    let mut transmuted = Transmuted {
        scrolls: Vec::with_capacity(processed_css.len()),
    };

    for (name, spells) in processed_css {
        if !name.is_empty() {
            // Convert HashSet to Vec to preserve JSON ordering
            let spells_vec: Vec<String> = spells.into_iter().collect();

            let oneliner = if include_oneliner {
                Some(spells_vec.join(" "))
            } else {
                None
            };

            transmuted.scrolls.push(TransmutedClass {
                name,
                spells: spells_vec,
                oneliner,
            });
        }
    }

    let duration = start_time.elapsed();

    let json_data = to_string_pretty(&transmuted).map_err(GrimoireCssError::Serde)?;

    Ok((duration, json_data))
}

/// Transmutes CSS content to Grimoire CSS format.
/// This is the main entry point for the content mode.
pub fn transmute_from_content(
    css_content: &str,
    include_oneliner: bool,
) -> Result<(f64, String), GrimoireCssError> {
    let start_time = Instant::now();

    let mut parser_state = ParserState::default();

    let processed_css = process_css_into_raw_spells(css_content, &mut parser_state)?;

    if processed_css.is_empty() {
        return Err(GrimoireCssError::InvalidInput(
            "There is nothing to transmute.".into(),
        ));
    }

    let mut transmuted = Transmuted {
        scrolls: Vec::with_capacity(processed_css.len()),
    };

    for (name, spells) in processed_css {
        if !name.is_empty() {
            // Convert HashSet to Vec to preserve JSON ordering
            let spells_vec: Vec<String> = spells.into_iter().collect();

            let oneliner = if include_oneliner {
                Some(spells_vec.join(" "))
            } else {
                None
            };

            transmuted.scrolls.push(TransmutedClass {
                name,
                spells: spells_vec,
                oneliner,
            });
        }
    }

    let duration = start_time.elapsed().as_secs_f64();

    let json_data = to_string_pretty(&transmuted).map_err(GrimoireCssError::Serde)?;

    Ok((duration, json_data))
}

/// Expands glob patterns into a list of file paths.
fn expand_file_paths(cwd: &Path, patterns: &[String]) -> Result<Vec<PathBuf>, GrimoireCssError> {
    let mut paths = Vec::with_capacity(patterns.len() * 4);

    for pattern in patterns {
        let absolute_pattern = if Path::new(pattern).is_absolute() {
            pattern.to_string()
        } else {
            cwd.join(pattern).to_string_lossy().into_owned()
        };

        for entry_result in glob(&absolute_pattern)
            .map_err(|e| GrimoireCssError::GlobPatternError(e.msg.to_string()))?
        {
            match entry_result {
                Ok(path) if path.is_file() => paths.push(path),
                Ok(_) => {} // Skip directories
                Err(e) => return Err(GrimoireCssError::InvalidPath(e.to_string())),
            }
        }
    }

    // If no memory waste, return as is; otherwise, shrink to fit
    if paths.len() < paths.capacity() / 2 {
        paths.shrink_to_fit();
    }

    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_last_char() {
        assert_eq!(remove_last_char("hello"), "hell");
        assert_eq!(remove_last_char("a"), "");
        assert_eq!(remove_last_char(""), "");
    }

    #[test]
    fn test_read_and_clean_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.css");
        let content = r#"
            /* Comment */
            .test {
                color: "red";
            }"#;

        fs::write(&file_path, content).unwrap();
        let result = read_and_clean_files(&[file_path]).unwrap();
        let expected = ".test { color: 'red'; }";

        let actual = result.replace("\n", "").replace(" ", "");
        let expected_normalized = expected.replace("\n", "").replace(" ", "");

        assert_eq!(actual, expected_normalized);
    }

    #[test]
    fn test_generate_spells_map() {
        let mut state = ParserState::default();
        state
            .raw_classes_spells_map
            .insert("class1".to_string(), vec!["prefix".to_string()]);
        state
            .component_and_component_target_map
            .insert("color=red".to_string());

        let result: HashMap<String, HashSet<String>> = generate_spells_map(&state);
        let left_spells = result.get("class1").unwrap();
        let left_spells_vec: Vec<String> = left_spells.iter().map(String::from).collect();

        assert_eq!(left_spells_vec, vec!["prefixcolor=red".to_string()]);
    }

    #[test]
    fn test_merge_maps() {
        let mut map1: HashMap<String, HashSet<String>> = HashMap::new();
        map1.insert("class1".to_string(), HashSet::from(["spell1".to_string()]));

        let mut map2: HashMap<String, HashSet<String>> = HashMap::new();
        map2.insert("class1".to_string(), HashSet::from(["spell2".to_string()]));
        map2.insert("class2".to_string(), HashSet::from(["spell3".to_string()]));

        merge_maps(&mut map1, map2);

        let left_spells = map1.get("class2").unwrap();
        let left_spells_vec: Vec<String> = left_spells.iter().map(String::from).collect();

        assert_eq!(left_spells_vec, vec!["spell3".to_string()]);
    }

    #[test]
    fn test_process_css_into_raw_spells() {
        let css_input = ".button { color: red; }";
        let mut parser_state = ParserState::default();

        let result = process_css_into_raw_spells(css_input, &mut parser_state);
        assert!(result.is_ok());
        let spells_map = result.unwrap();
        let left_spells = spells_map.get("button").unwrap();
        let left_spells_vec: Vec<String> = left_spells.iter().map(String::from).collect();

        assert_eq!(left_spells_vec, vec!["color=red".to_string()]);
    }

    #[test]
    fn test_expand_file_paths() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.css");
        fs::write(&file_path, ".test { color: red; }").unwrap();

        let cwd = temp_dir.path().to_path_buf();
        let result = expand_file_paths(&cwd, &["test.css".to_string()]);

        assert!(result.is_ok());
        let paths = result.unwrap();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], file_path);
    }

    #[test]
    fn test_transmute_from_content() {
        let css_input = ".button { color: red; }";
        let result = transmute_from_content(css_input, false);
        assert!(result.is_ok());
        let (_duration, json_output) = result.unwrap();
        assert!(json_output.contains("\"name\": \"button\""));
        assert!(json_output.contains("\"color=red\""));
    }
}
