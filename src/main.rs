use std::{
    env,
    fs::{self},
    io::Write,
    path::Path,
};

use anyhow::{Context, Result};
use feature::Feature;
use glob::glob;

// Interprets the first argument as a filepath src/err/___.txt,
// includes it with include_str!, and formats it with the remaining
// arguments. Don't make any sudden movements, it bites.
#[macro_use]
macro_rules! fmt_err {
    ($fname:tt) => {{
        format!(include_str!(concat!(stringify!(err),stringify!(/),stringify!($fname),stringify!(.),stringify!(txt))))
    }};
    ($fname:tt, $($fmtargs:expr),+) => {{
        format!( include_str!(concat!(stringify!(err),stringify!(/),stringify!($fname),stringify!(.),stringify!(txt))), $($fmtargs),+ )
    }};
}

mod feature;
mod step;

type Str<'a> = &'a str;

pub trait Language {
    type ArgTypes;
}

pub trait TestFramework {
    type Lang: Language;
}

pub struct CSharp;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum CSType {
    Bool,
    Int32,
    Int64,
    Double,
    String,
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
    let outcome = main_inner().context("Fatal error");
    if let Err(e) = outcome {
        eprintln!("{:#}", e);
    }
}

fn main_inner() -> Result<()> {
    let mut success_count = 0;
    let mut failure_count = 0;
    let mut args = env::args_os().skip(1);
    let input_path_os = &args.next().context("No input path given.")?;
    let input_path = input_path_os
        .to_str()
        .with_context(|| fmt_err!(inpath_not_utf8, input_path_os))?;

    let output_dir = match args.next() {
        Some(os_str) => Path::new(&os_str).to_owned(),
        None => env::current_dir().context(fmt_err!(target_dir_undetermined))?,
    };
    //let output_dir = args.next();
    println!("GOT HERE");
    println!("{}", input_path);
    for entry in glob(input_path).context(format!(
        "Error evaluation paths for input pattern {}",
        input_path
    )) {
        for path in entry {
            match path {
                Ok(path) => {
                    if path.is_dir() {
                        continue;
                    }
                    let name = &path
                        .file_name()
                        .context("Input file not found")?
                        .to_str()
                        .context("File path contains invalid utf-8")?;
                    let content = fs::read_to_string(&path).context(fmt_err!(bad_infile, name))?;
                    // Trim utf-8 BOM, if present
                    let content = content.trim_start_matches("\u{FEFF}");
                    let feature = Feature::from_str(content);
                    match feature {
                        Ok(feature) => {
                            let mut w = fs::OpenOptions::new()
                                .create(true)
                                .write(true)
                                .open(output_dir.join((*name).to_owned() + ".cs"))
                                .context(format!("Failed to create output file for {}", name))?;
                            write!(w, "{}", feature.export(NUnit)).unwrap();
                            //println!("Successful parse :)");
                            success_count += 1;
                        }
                        Err(error) => {
                            fs::write(
                                output_dir.join((*name).to_owned() + ".log"),
                                format!("{:#}", error),
                            )
                            .context(format!("Error attempting to write log file for {}", name))?;
                            failure_count += 1;
                        }
                    }
                }
                Err(path_err) => {
                    eprintln!("{:?}", path_err);
                    failure_count += 1;
                }
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
