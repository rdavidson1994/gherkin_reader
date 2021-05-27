use std::{iter::Skip, str::Split};

use crate::{feature::ParseStr, Str};
use anyhow::{bail, Context, Result};

type TagIterator<'a> = Skip<Split<'a, char>>;

#[derive(Debug, Clone, Copy)]
pub enum GroupingKeyword {
    ScenarioOutline,
    //ScenarioTemplate, // synonym for ScenarioOutline
    Scenario,
    //Example, // synonym for Scenario
    Background,
    Examples,
    //Scenarios, // synonym for Examples
    Feature,
    // Rule, // not supported yet
}

#[derive(Debug)]
pub(crate) enum GherkinLine<'a> {
    Tags(TagIterator<'a>),
    StepLine(StepKeyword, &'a str),
    BeginGroup(GroupingKeyword, &'a str),
    FreeText(&'a str),
    ExampleEntry(&'a str),
}

impl<'a> GherkinLine<'a> {
    pub(crate) fn from_str(mut input: &'a str) -> GherkinLine<'a> {
        use GherkinLine::*;
        use GroupingKeyword::*;
        input = input.trim();
        if let Some((keyword, title)) = input.split_once(':') {
            let keyword = keyword.trim();
            match keyword {
                "Scenario" | "Example " => return BeginGroup(Scenario, title),
                "Examples" | "Scenarios" => return BeginGroup(Examples, title),
                "Scenario Outline" | "Scenario Template" => {
                    return BeginGroup(ScenarioOutline, title)
                }
                "Feature" => return BeginGroup(Feature, title),
                "Background" => return BeginGroup(Background, title),
                _ => {
                    // Let any other data fall through to other cases
                }
            }
        }

        if let Some((keyword, title)) = input.split_once(' ') {
            use StepKeyword::*;
            let keyword = keyword.trim();
            match keyword {
                "Given" => return StepLine(Given, title),
                "When" => return StepLine(When, title),
                "Then" => return StepLine(Then, title),
                "And" => return StepLine(And, title),
                "But" => return StepLine(But, title),
                "*" => return StepLine(Bullet, title),
                _ => {
                    // Let unmatched keywords fall through
                }
            }
        }

        if input.starts_with('@') {
            return Tags(input.split('@').skip(1));
        }

        if input.starts_with('|') {
            return ExampleEntry(input);
        }

        return FreeText(input);
    }
}

#[derive(Debug)]
pub struct Step<'a> {
    pub(crate) keyword: StepKeyword,
    pub(crate) literals: Vec<Str<'a>>,
    pub(crate) variables: Vec<Str<'a>>,
}

impl<'a> Step<'a> {
    pub fn new(keyword: StepKeyword, input: Str<'a>) -> Result<Step<'a>> {
        let mut remaining_text = input.trim();
        let mut literals = vec![];
        let mut variables = vec![];
        loop {
            if let Some((literal, text)) = remaining_text.split_once('<') {
                remaining_text = text;
                literals.push(literal);
                let (variable, text) = remaining_text.split_once('>').with_context(|| {
                    format!(
                        "The following step: \n\
                        `{step}`\n\
                        ends with an unterminated variable expression{}\n\
                        `{expression}`",
                        step = input,
                        expression = remaining_text
                    )
                })?;
                remaining_text = text;
                variables.push(variable);
            } else {
                literals.push(remaining_text);
                break;
            }
        }
        Ok(Step {
            keyword,
            literals,
            variables,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum StepKeyword {
    Given,
    When,
    Then,
    And,
    But,
    Bullet,
}

impl StepKeyword {
    pub fn from_str(input: Str) -> Result<StepKeyword> {
        use StepKeyword::*;
        match input {
            "Given" => Ok(Given),
            "When" => Ok(When),
            "Then" => Ok(Then),
            "And" => Ok(And),
            "But" => Ok(But),
            "*" => Ok(Bullet),
            _ => bail!("Unrecognized Step keyword '{}' (expected to find 'Given', 'When', 'And', 'Then', 'But' or '*')", input),
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum FeatureItemKeyword {
    Scenario,
    ScenarioOutline,
    Background,
}

impl<'a> ParseStr<'a> for FeatureItemKeyword {
    fn from_str(input: &'a str) -> Result<Self>
    where
        Self: Sized,
    {
        use FeatureItemKeyword::*;
        match input {
            "Background" => Ok(Background),
            "Scenario" | "Example" => Ok(Scenario),
            "Scenario Outline" | "Scenario Template" => Ok(ScenarioOutline),
            _ => bail!(
                "Keyword {} was expected to begin a Scenario \
                or Scenario Outline (was not any of 'Scenario', \
                'Scenario Outline', 'Scenario Template', or 'Example')"
            ),
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum Keyword {
    Feature,
    FeatureItem(FeatureItemKeyword),
    Examples,
    Step(StepKeyword),
}

impl<'a> ParseStr<'a> for Keyword {
    fn from_str(input: &str) -> Result<Self>
    where
        Self: Sized,
    {
        use Keyword::*;
        if let Ok(fik) = FeatureItemKeyword::from_str(input) {
            Ok(FeatureItem(fik))
        } else if let Ok(step) = StepKeyword::from_str(input) {
            Ok(Step(step))
        } else {
            match input {
                "Feature" => Ok(Feature),
                "Examples" | "Scenarios" => Ok(Examples),
                _ => bail!("Coult not parse input {} as any known keyword.", input),
            }
        }
    }
}
