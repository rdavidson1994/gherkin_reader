use std::{
    env,
    error::Error,
    fs::{self, File},
    io::Write,
};

use feature::{Feature, FeatureItem, ParseStr};
use glob::glob;
use step::Step;

use crate::step::StepKeyword;

mod feature;
mod step;

type Str<'a> = &'a str;

pub struct NUnit;
impl NUnit {
    fn interpret_arg(arg: Str) -> String {
        if arg.starts_with("/") {
            String::new() + "@\"" + &arg[1..] + "\""
        }
        else if arg.starts_with('"')
            && arg.ends_with('"')
            && arg.chars().filter(|&x| x == '"').count() == 2
        {
            String::new() + "@" + arg
        }
        else {
            String::new() + "x"
        }
    }
}

pub trait Export<T> {
    fn export(&self, export_format: T) -> String;
}

fn test_given() {
    match StepKeyword::from_str("Given") {
        Some(StepKeyword::Given) => {
            // pass
        }
        _ => {
            panic!("Oh no!");
        }
    }
}

fn test_load_step() {
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

    for (input, expectation) in cases {
        match (Step::from_str(input), expectation) {
            (None, None) => {
                // pass
            }
            (None, Some(arity)) => {
                panic!(
                    "Expected to parse the following as a step with arity {}:\n{}",
                    arity, input
                );
            }
            (Some(step), None) => {
                dbg!(input);
                dbg!(step);
                panic!("^^^ Nonsense string parsed into the above step!")
            }
            (Some(step), &Some(arity)) => {
                if step.arity() == arity {
                    // pass
                } else {
                    panic!(
                        "Expected the following string:\n{:?}\n to decode to step of arity {:?}, but got this step:\n{:?}\n of arity {:?} instead.",
                        input, arity, step, step.arity()
                    )
                }
            }
        }
    }
}

fn test_load_scenario() {
    let input = r###"Scenario: Shave a yak
    Given I have a yak
    And My yak has hair
    And I have a razor
    When I shave the yak
    Then My yak does not have <hair>
    And I have yak hair"###;

    let item = FeatureItem::from_str(input);
    if let Some(item) = item {
        dbg!(item);
        // pass
    } else {
        panic!("Could not parse scenario!");
    }
}

fn test_load_scenario_outline() {
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
    if let Some(item) = item {
        dbg!(item);
        // pass
    } else {
        panic!("Could not parse scenario!");
    }
}

fn test_load_feature() {
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
    let outcome = Feature::from_str(input);

    if let Some(outcome) = outcome {
        dbg!(outcome);
    } else {
        panic!();
    }
}

fn main_test() {
    test_given();
    test_load_step();
    test_load_scenario();
    test_load_scenario_outline();
    test_load_feature();
    println!("Yay!");
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut success_count = 0;
    let mut failure_count = 0;
    let mut args = env::args().skip(1);
    let input_path = args.next().expect("No input path given.");
    // let output_dir = args.next();
    for entry in glob(&input_path) {
        for path in entry {
            match path {
                Ok(path) => {
                    let name = &path.file_name().unwrap().to_str().unwrap();
                    let file_contents = fs::read_to_string(&path)?;
                    let feature = Feature::from_str(&file_contents);
                    if let Some(feature) = feature {
                        let mut w = fs::OpenOptions::new().create(true).write(true).open(
                            r"C:\Users\rdavidson\Desktop\Features\".to_owned() + name + ".cs",
                        )?;
                        write!(w, "{}", feature.export(NUnit)).unwrap();
                        //println!("Successful parse :)");
                        success_count += 1;
                    } else {
                        //eprintln!("Failed parse :(");
                        failure_count += 1;
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
