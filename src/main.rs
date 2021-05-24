use std::{env, fs::{self}, io::Write, path::{Path, PathBuf}};
use quote::quote;

use syn::parse::{Parse, ParseStream, Result as SynResult};
use syn;
use anyhow::{anyhow, Error, Result, Context};
use feature::{Feature, FeatureItem, ParseStr};
use glob::glob;
use step::Step;

use crate::step::StepKeyword;

mod feature;
mod step;

type Str<'a> = &'a str;


macro_rules! template {
    ($tmp:tt, $($arg:tt)*) => {{
        let res = format!(include_str!(#$tmp),$($arg)*);
        res
    }}
}

pub trait Language {
    type ArgTypes;
}

pub trait TestFramework {
    type Lang : Language;
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
impl NUnit {
    fn interpret_arg(arg: Str) -> (String, CSType) {
        let path = "errors/invalid/scenario.txt";
        if let Ok(_integer) = arg.parse::<i64>() {
            (arg.to_owned(), CSType::Int64)
        } else if arg.starts_with("/") {
            (String::new() + "@\"" + &arg[1..] + "\"", CSType::String)
        } else if arg.starts_with('"')
            && arg.ends_with('"')
            && arg.chars().filter(|&x| x == '"').count() == 2
        {
            (String::new() + "@" + arg, CSType::String)
        } else {
            let mut output = String::from("@\"");
            let output = arg.split('"').fold(output, |a,b| a + "\"\"" + b);
            (output, CSType::String)
        }
    }
}

pub trait Export<T> {
    fn export(&self, export_format: T) -> String;
}

#[test]
fn test_given() -> Result<()> {
    StepKeyword::from_str("Given").map(|_| ())
}

//#[test]

#[test]
fn test_load_step() -> Result<()> {
    let cases: &[(Str, Option<usize>)] = &[
        ("Given I do something", Some(0)),
        ("    When I load an image from <path>", Some(1)),
        (
            " Then images <output> and <groundtruth> are visually identical",
            Some(2),
        ),
        ("   When <verb> happens", Some(1)),
        ("Then I", Some(0)),
        ("Then", None),
        ("<BadParam> Given I don't care", None),
        ("Hopefully it notices this is an invalid keyword", None),
    ];
    let mut results = vec![];
    for (input, expectation) in cases {
        let result = match (Step::from_str(input), expectation) {
            (Err(_), None) => {
                // pass
                Ok(())
            }
            (Err(err), Some(arity)) => Err(err),
            (Ok(step), None) => Err({
                anyhow!(
                    "Parsed a nonsensical input {} into this step: {:?}",
                    input,
                    step
                )
            }),
            (Ok(step), &Some(arity)) => {
                if step.arity() == arity {
                    Ok(())
                    // pass
                } else {
                    panic!(
                        "Expected the following string:\n{:?}\n to decode to step of arity {:?}, but got this step:\n{:?}\n of arity {:?} instead.",
                        input, arity, step, step.arity()
                    )
                }
            }
        };
        results.push(result);
    }
    Ok(())
}

#[test]
fn test_load_scenario() -> Result<()> {
    let input = r###"Scenario: Shave a yak
    Given I have a yak
    And My yak has hair
    And I have a razor
    When I shave the yak
    Then My yak does not have <hair>
    And I have yak hair"###;

    let item = FeatureItem::from_str(input);
    item.map(|_| ())
}

#[test]
fn test_load_scenario_outline() -> Result<()> {
    let input = r###"Scenario Outline: Shave an animal
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
    let item = FeatureItem::from_str(input);
    item.map(|_| ())
}

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


fn main() -> Result<()> {
    let mut success_count = 0;
    let mut failure_count = 0;
    let mut args = env::args_os().skip(1);
    let input_path = Path::new(&args.next().expect("No input path given."));
    let output_dir = match args.next() {
        Some(os_str) => {
            Path::new(&os_str).to_owned()
        }
        None => {
            let s = include_str!("errors/invalid/target_dir.txt");
            env::current_dir().context(s)?
        }
    };
    let output_dir = args.next();
    for entry in glob("*") {
        for path in entry {
            match path {
                Ok(path) => {
                    let name = &path.file_name().unwrap().to_str().unwrap();
                    let content = fs::read_to_string(&path)?;
                    // Trim utf-8 BOM, if present
                    let content = content.trim_start_matches("\u{FEFF}");
                    let feature = Feature::from_str(content);
                    match feature {
                        Ok(feature) => {
                            let mut w = fs::OpenOptions::new().create(true).write(true).open(
                                r"C:\Users\rdavidson\Desktop\Features\".to_owned() + name + ".cs",
                            )?;
                            write!(w, "{}", feature.export(NUnit)).unwrap();
                            //println!("Successful parse :)");
                            success_count += 1;
                        }
                        Err(error) => {
                            fs::write(
                                r"C:\Users\rdavidson\Desktop\Features\logs\".to_owned()
                                    + name
                                    + ".log",
                                error.to_string(),
                            )?;
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
