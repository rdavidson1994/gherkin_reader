use crate::tags::GherkinTags;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub(crate) enum GherkinLine<'a> {
    Tags(GherkinTags<'a>),
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
            let title = title.trim();
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
            let title = title.trim();
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

        if let Some(("", after_at_sign)) = input.split_once('@') {
            return Tags(GherkinTags::new(after_at_sign));
        }

        if input.starts_with('|') {
            return ExampleEntry(input);
        }

        return FreeText(input);
    }
}

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

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum StepKeyword {
    Given,
    When,
    Then,
    And,
    But,
    Bullet,
}

impl StepKeyword {
    pub fn from_str(input: &str) -> Result<StepKeyword> {
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

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum FeatureItemKeyword {
    Scenario,
    ScenarioOutline,
    Background,
}
