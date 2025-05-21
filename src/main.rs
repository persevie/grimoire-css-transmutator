use gcsst_lib::{run_transmutation, transmute_from_content};
use grimoire_css_lib::GrimoireCssError;
use std::env;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::PathBuf;
use std::process;

const HELP_MESSAGE: &str = "
Grimoire CSS Transmute (gcsst) - Convert CSS to Grimoire CSS format

USAGE:
    gcsst [OPTIONS] [INPUT]

OPTIONS:
    -p, --paths           Process comma-separated list of CSS file paths or patterns
    -c, --content         Process CSS content provided as string
    -o, --output          Specify output file (default: ./grimoire/transmuted.json)
    -l, --with-oneliner   Include oneliner property in output (default: disabled)
    -h, --help            Display this help message

EXAMPLES:
    gcsst -p styles.css,components.css
    gcsst -c '.button { color: red; }' -l
    gcsst -p '*.css' -o custom_output.json --with-oneliner
";

type AppResult<T> = Result<T, GrimoireCssError>;

struct Config {
    mode: Mode,
    input: String,
    output_path: Option<String>,
    include_oneliner: bool,
}

enum Mode {
    Paths,
    Content,
    Help,
}

fn main() {
    process::exit(match run_app() {
        Ok(_) => 0,
        Err(err) => {
            eprintln!("Error: {}", err);
            1
        }
    });
}

fn run_app() -> AppResult<()> {
    let config = parse_args()?;

    match config.mode {
        Mode::Help => {
            print!("{}", HELP_MESSAGE);
            Ok(())
        }
        Mode::Paths => process_paths_mode(&config),
        Mode::Content => process_content_mode(&config),
    }
}

fn parse_args() -> AppResult<Config> {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() || args.contains(&"-h".to_string()) || args.contains(&"--help".to_string()) {
        return Ok(Config {
            mode: Mode::Help,
            input: String::new(),
            output_path: None,
            include_oneliner: false,
        });
    }

    let mut mode = None;
    let mut input = None;
    let mut output_path = None;
    let mut include_oneliner = false;
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "-p" | "--paths" => {
                mode = Some(Mode::Paths);
                if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                    input = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "-c" | "--content" => {
                mode = Some(Mode::Content);
                if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                    input = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "-o" | "--output" => {
                if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                    output_path = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "-l" | "--with-oneliner" => {
                include_oneliner = true;
            }
            arg if arg.starts_with('-') => {
                return Err(GrimoireCssError::InvalidInput(format!(
                    "Unknown option: {}",
                    arg
                )));
            }
            _ => {
                if input.is_none() && mode.is_some() {
                    input = Some(args[i].clone());
                }
            }
        }
        i += 1;
    }

    let mode = mode.ok_or_else(|| {
        GrimoireCssError::InvalidInput(
            "Mode not specified. Use -p for paths or -c for content.".into(),
        )
    })?;

    let input =
        input.ok_or_else(|| GrimoireCssError::InvalidInput("Input not provided.".into()))?;

    Ok(Config {
        mode,
        input,
        output_path,
        include_oneliner,
    })
}

/// Process CSS files in paths mode
fn process_paths_mode(config: &Config) -> AppResult<()> {
    // Split paths by comma and trim whitespace
    let paths: Vec<String> = config
        .input
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();

    let (duration, json_output) = run_transmutation(paths, config.include_oneliner)?;

    // Handle output
    match &config.output_path {
        Some(path) => write_to_file(path, &json_output)?,
        None => {
            let cwd = env::current_dir().map_err(GrimoireCssError::Io)?;
            let output_dir = cwd.join("grimoire");
            fs::create_dir_all(&output_dir).map_err(GrimoireCssError::Io)?;
            let output_file = output_dir.join("transmuted.json");
            write_to_file(&output_file.to_string_lossy(), &json_output)?;

            eprintln!(
                "Transmutation complete in {:.2?}. Output written to {:?}",
                duration, output_file
            );
        }
    }

    Ok(())
}

/// Process CSS content directly
fn process_content_mode(config: &Config) -> AppResult<()> {
    // Pass the include_oneliner flag to the library function
    let (duration, json_output) = transmute_from_content(&config.input, config.include_oneliner)?;

    // Handle output
    match &config.output_path {
        Some(path) => write_to_file(path, &json_output)?,
        None => {
            // Print JSON to stdout for redirection
            io::stdout()
                .write_all(json_output.as_bytes())
                .map_err(GrimoireCssError::Io)?;
            // Print status to stderr
            eprintln!("Transmutation complete in {:.2} seconds", duration);
        }
    }

    Ok(())
}

/// Write content to a file with error handling
fn write_to_file(path: &str, content: &str) -> AppResult<()> {
    if let Some(parent) = PathBuf::from(path).parent() {
        fs::create_dir_all(parent).map_err(GrimoireCssError::Io)?;
    }

    let mut file = File::create(path).map_err(GrimoireCssError::Io)?;
    file.write_all(content.as_bytes())
        .map_err(GrimoireCssError::Io)?;

    eprintln!("Output written to {}", path);
    Ok(())
}
