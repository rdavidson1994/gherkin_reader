use std::{env, fs, io::Write, path::PathBuf};

use anyhow::{Context, Result};
use argh::FromArgs;
use feature::Feature;
use glob::glob;
mod feature;
mod step;

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
}

type Str<'a> = &'a str;

pub trait Language {
    type ArgTypes;
}

pub trait TestFramework {
    type Lang: Language;
}

pub struct CSharp;

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum CSType {
    Unknown,
    Bool,
    Int64,
    Double,
    String,
}

impl CSType {
    fn lowest_common_type(self, other: CSType) -> CSType {
        use CSType::*;
        match (self, other) {
            // Undetermined types remain undetermined until
            // more info is available
            (Unknown, Unknown) => Unknown,
            // Any new information replaces an undetermined type
            (Unknown, x) | (x, Unknown) => x,
            // Calculated types remain in place unless contradicted
            (x, y) if x == y => x,
            // If a contradiction occurs, we default back to string
            _ => String,
        }
    }
    fn from(input: &str) -> CSType {
        if input.parse::<i64>().is_ok() {
            CSType::Int64
        } else if input.parse::<f64>().is_ok() {
            CSType::Double
        } else if input.parse::<bool>().is_ok() {
            CSType::Bool
        } else {
            CSType::String
        }
    }

    fn to_str(self) -> &'static str {
        match self {
            CSType::Unknown => "object",
            CSType::Bool => "bool",
            CSType::Int64 => "long",
            CSType::Double => "double",
            CSType::String => "string",
        }
    }
}

impl Language for CSharp {
    type ArgTypes = CSType;
}

impl TestFramework for NUnit {
    type Lang = CSharp;
}

pub struct NUnit;

pub trait Export<T> {
    fn export(&self, export_format: T) -> String;
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
                let mut w = fs::OpenOptions::new()
                    .create(true)
                    .write(true)
                    .open(output_dir.join((*name).to_owned() + ".cs"))
                    .context(format!("Failed to create output file for {}", name))?;
                write!(w, "{}", feature.export(NUnit)).unwrap();
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

#[cfg(test)]
mod tests {
    use crate::feature::Feature;
    use anyhow::Result;
    #[test]
    fn test_load_feature() -> Result<()> {
        let input = r###"
    Feature: Farm activities
    
    
    Scenario: Shave a yak
        Given I have a yak
        And My yak has hair
        And I have a razor
        When I shave the yak
        Then My yak does not have <hair>
        And I have yak hair
    
    
    
    Scenario Outline: Shave an animal
        Given I am Old McDonald
        And I have a farm
        And On that farm there is a <animal>
        When I listen
        Then I hear a <noise> here
        And I hear a <noise> there
    Examples:
        | animal | noise |
        | cow    | moo   |
        | horse  | neigh |
        | pig    | oink  |
    "###;
        Feature::from_str(input).map(|_| ())
    }

    #[test]
    fn test_load_outline_with_multiple_example_blocks() -> Result<()> {
        let input = r###"
    Feature: Farm activities
    
    
    Scenario: Shave a yak
        Given I have a yak
        And My yak has hair
        And I have a razor
        When I shave the yak
        Then My yak does not have <hair>
        And I have yak hair
    
    
    
    Scenario Outline: Shave an animal
        Given I am Old McDonald
        And I have a farm
        And On that farm there is a <animal>
        When I listen
        Then I hear a <noise> here
        And I hear a <noise> there
    
    @Mammal
    Examples:
        | animal  | noise |
        | cow     | moo   |
        | horse   | neigh |
        | pig     | oink  |
    
    @Bird
    Examples:
        | duck    | quack |
        | chicken | cluck |
    "###;
        Feature::from_str(input).map(|_| ())
    }
}
