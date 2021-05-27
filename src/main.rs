use crate::export::Export;
use anyhow::{bail, Context, Result};
use argh::{Flag, FromArgValue, FromArgs};
use feature::Feature;
use glob::glob;
use std::{env, fs, io::Write, path::PathBuf, str::FromStr};

use crate::export::NUnit;

mod export;
mod feature;
mod gherkin_tags;
mod step;
mod tags;

#[cfg(test)]
mod tests;

#[derive(PartialEq, Debug)]
enum ExportFormat {
    NUnit,
    JSON,
}

impl FromStr for ExportFormat {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "nunit" => Ok(ExportFormat::NUnit),
            "json" => Ok(ExportFormat::JSON),
            _ => bail!("Inavlid export format {}", s),
        }
    }
}

/// Convert gherkin files to .cs source files.
#[derive(FromArgs)]
struct Arguments {
    /// pattern to match feature files.
    /// For example, src/tests/*/features/*.feature
    #[argh(positional)]
    input_pattern: String,
    /// destination for output source files and logs.
    /// Defaults to ${cwd}/gherkin_output
    #[argh(positional)]
    output_path: Option<PathBuf>,

    /// which export format to use (default nunit)
    #[argh(option, default = "ExportFormat::NUnit")]
    export_format: ExportFormat,
}

fn main() {
    let args = argh::from_env();
    let outcome = main_inner(args).context("Fatal error");
    if let Err(e) = outcome {
        eprintln!("{:#}", e);
    }
}

fn main_inner(args: Arguments) -> Result<()> {
    let mut success_count = 0;
    let mut failure_count = 0;
    let input_path = args.input_pattern;

    let output_dir = match args.output_path {
        Some(path) => path,
        None => env::current_dir()
            .context(
                "No output file provided, and the current working \
                directory could not be determined due to an IO error.",
            )?
            .join("gherkin_output"),
    };
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
                let extension = match args.export_format {
                    ExportFormat::NUnit => ".cs",
                    ExportFormat::JSON => ".json",
                };
                let mut w = fs::OpenOptions::new()
                    .create(true)
                    .write(true)
                    .open(output_dir.join((*name).to_owned() + extension))
                    .context(format!("Failed to create output file for {}", name))?;

                let content = match args.export_format {
                    ExportFormat::NUnit => feature.export(NUnit),
                    ExportFormat::JSON => serde_json::to_string_pretty(&feature)?,
                };
                //w.write(content.as_bytes())?;
                write!(w, "{}", content)?;
                success_count += 1;
            } else if let Err(error) = feature {
                let display_path = path.to_str().unwrap_or("[[Non UTF-8 path]]");
                let display_error = format!("{:#}", error).replace(':', ":\n");
                fs::write(
                    output_dir.join((*name).to_owned() + ".log"),
                    format!("{}\n\n{}", display_path, display_error),
                )
                .context(format!(
                    "Error attempting to write error log for file `{}`",
                    name
                ))?;
                failure_count += 1;
            }
        }
    }
    println!("Successful parses: {}", success_count);
    println!("Failed parses: {}", failure_count);
    Ok(())
}
