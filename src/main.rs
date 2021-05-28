use crate::export::Export;
use anyhow::{Context, Result};
use clap::{crate_version, AppSettings, Clap};
use feature::Feature;
use glob::glob;
use std::{fs, io::Write, path::PathBuf};

use crate::export::NUnit;

mod export;
mod feature;
mod gherkin_tags;
mod step;
mod tags;

#[cfg(test)]
mod tests;

#[derive(Debug, Clap)]
enum ExportFormat {
    #[clap(name = "nunit")]
    NUnit,
    JSON,
}

#[derive(Debug, Clap)]
enum ErrorBehavior {
    /// Creates a .log file for each failed parse, and sends it to
    /// the directory indicated by [output_path]
    Log,

    /// Ignores failed parses
    Silent,

    /// Outputs error messages to stdout
    Stdout,

    /// Outputs error messages to stderr
    Stderr,
}

#[derive(Debug, Clap)]
#[clap(
    about="A tool to convert gherkin feature files",
    version=crate_version!(),
    setting(AppSettings::ArgRequiredElseHelp)
)]
struct Arguments {
    /// Input path (use wildcards for directory contents)
    #[clap(parse(from_str))]
    input_pattern: String,

    /// Destination for output source files and logs.
    #[clap(parse(from_os_str), default_value(".\\gherkin_output"))]
    output_path: PathBuf,

    /// Output format for converted feature files
    #[clap(short = 'f', long = "format")]
    #[clap(arg_enum)]
    #[clap(default_value("nunit"))]
    export_format: ExportFormat,

    /// What the do with error messages. If set to `log`, log
    /// files are created in <output_path>
    #[clap(short, long)]
    #[clap(arg_enum)]
    #[clap(default_value("log"))]
    error_behavior: ErrorBehavior,
}

fn main() {
    let args = Arguments::parse();
    let outcome = main_inner(args).context("Fatal error");
    if let Err(e) = outcome {
        eprintln!("{:#}", e);
    }
}

fn main_inner(args: Arguments) -> Result<()> {
    let mut success_count = 0;
    let mut failure_count = 0;
    let input_path = args.input_pattern;
    let export_format = args.export_format;
    let output_dir = args.output_path;
    fs::create_dir_all(&output_dir).context(format!(
        "Could not create output directory: {:?}",
        &output_dir
    ))?;
    let paths = glob(&input_path).context(format!(
        "Error evaluating paths for input pattern {}",
        input_path
    ))?;
    for path in paths {
        if let Err(path_err) = path {
            eprintln!("{:?}", path_err);
            failure_count += 1;
        } else if let Ok(path) = path {
            if path.is_dir() {
                continue;
            }
            let name = &path
                .file_name()
                .context("Input file not found")?
                .to_str()
                .context("File path contains invalid utf-8")?;
            let content = fs::read_to_string(&path)
                .context(format!("Could not read the following input file: {}", name))?;

            // Trim utf-8 BOM, if present
            let content = content.trim_start_matches("\u{FEFF}");

            let feature = Feature::from_str(content);
            if let Ok(feature) = feature {
                let extension = match export_format {
                    ExportFormat::NUnit => ".cs",
                    ExportFormat::JSON => ".json",
                };
                let mut w = fs::OpenOptions::new()
                    .create(true)
                    .write(true)
                    .open(output_dir.join((*name).to_owned() + extension))
                    .context(format!("Failed to create output file for {}", name))?;

                let content = match export_format {
                    ExportFormat::NUnit => feature.export(NUnit),
                    ExportFormat::JSON => serde_json::to_string_pretty(&feature)?,
                };
                //w.write(content.as_bytes())?;
                write!(w, "{}", content)?;
                success_count += 1;
            } else if let Err(error) = feature {
                let display_path = path.to_str().unwrap_or("[[Non UTF-8 path]]");
                let display_error = format!("{:#}", error).replace(':', ":\n");
                let error_text = format!("Error parsing {}: {}", display_path, display_error);
                match args.error_behavior {
                    ErrorBehavior::Log => {
                        fs::write(output_dir.join((*name).to_owned() + ".log"), error_text)
                            .context(format!(
                                "Error attempting to write error log for file `{}`",
                                name
                            ))?;
                    }
                    ErrorBehavior::Silent => {
                        // deaddove.jpg
                    }
                    ErrorBehavior::Stdout => {
                        println!("{}", error_text)
                    }
                    ErrorBehavior::Stderr => {
                        eprintln!("{}", error_text)
                    }
                }

                failure_count += 1;
            }
        }
    }
    println!("Successful parses: {}", success_count);
    println!("Failed parses: {}", failure_count);
    Ok(())
}
