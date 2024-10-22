use gcsst_lib::{run_transmutation, transmute_from_content};
use grimoire_css_lib::core::GrimoireCSSError;
use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

fn main() -> Result<(), GrimoireCSSError> {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.len() < 2 {
        return Err(GrimoireCSSError::InvalidInput(
            "Not enough arguments provided.".into(),
        ));
    }

    let mode = &args[0];
    let input = &args[1];

    match mode.as_str() {
        "-p" => {
            // Paths mode
            let paths: Vec<String> = input.split(',').map(String::from).collect();
            let (duration, json_output) = run_transmutation(paths)?;

            let cwd: PathBuf = env::current_dir().map_err(GrimoireCSSError::Io)?;
            let output_dir = cwd.join("grimoire");
            fs::create_dir_all(&output_dir).map_err(GrimoireCSSError::Io)?;
            let output_file = output_dir.join("transmuted.json");

            let mut file = File::create(&output_file).map_err(GrimoireCSSError::Io)?;
            file.write_all(json_output.as_bytes())
                .map_err(GrimoireCSSError::Io)?;

            println!(
                "Transmutation complete in {:.2?}. Output written to {:?}",
                duration, output_file
            );
        }
        "-c" => {
            // Content mode
            let (duration, json_output) = transmute_from_content(input)?;
            println!("{}", json_output);
            println!(
                "Transmutation complete in {:.2?}. Output {:?}",
                duration, json_output
            );
        }
        _ => {
            return Err(GrimoireCSSError::InvalidInput(
                "Invalid mode provided. Use -p for paths or -c for content.".into(),
            ));
        }
    }

    Ok(())
}
