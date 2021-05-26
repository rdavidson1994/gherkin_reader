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
    // invariant: literals.len() == variables.len() + 1;
    keyword: StepKeyword,
    literals: Vec<Str<'a>>,
    variables: Vec<Str<'a>>,
}

impl<'a> Step<'a> {
    // pub fn arity(&self) -> usize {
    //     self.variables.len()
    // }

    // pub fn content(&self) -> (&[Str], &[Str]) {
    //     (&self.literals, &self.variables)
    // }

    // pub(crate) fn invariant(&self) -> bool {
    //     self.variables.len() == self.literals.len() - 1
    // }

    pub fn new(keyword: StepKeyword, input: Str<'a>) -> Result<Step<'a>> {
        let text = input.trim();
        let text = text.trim();
        let mut tokens = text.split(|c| c == '<' || c == '>');
        let mut literals = vec![tokens.next().with_context(|| {
            format!(
                "Step content appears empty after splitting angle brackets and trimming: `{}`",
                input
            )
        })?];
        let mut variables = vec![];
        loop {
            if let Some(variable) = tokens.next() {
                variables.push(variable);
                literals.push(tokens.next().with_context(|| {
                    format!(
                        "Step ends with unterminated variable expression : {}",
                        input
                    )
                })?);
            } else {
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
}

impl<'a> ParseStr<'a> for FeatureItemKeyword {
    fn from_str(input: &'a str) -> Result<Self>
    where
        Self: Sized,
    {
        use FeatureItemKeyword::*;
        match input {
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

// pub fn read_keyword_and_title(input: &str) -> Result<(Keyword, &str)> {
//     // Try to parse the content before the first space as a keyword
//     let mut candidate_whitespace_keyword = None;
//     if let Some((kw, title)) = input.split_once(" ") {
//         let kw = kw.trim();
//         candidate_whitespace_keyword = Some(kw);
//         if let Ok(step_keyword) = StepKeyword::from_str(kw) {
//             return Ok((Keyword::Step(step_keyword), title.trim()))
//         }
//     }

//     let mut candidate_colon_keyword = None;
//     // Try to parse the content before the first colon as a keyword
//     if let Some((kw, title)) = input.split_once(':') {
//         let kw = kw.trim();
//         candidate_colon_keyword = Some(kw);
//         if let Ok(keyword) = Keyword::from_str(kw) {
//             return Ok((keyword, title.trim()))
//         }
//     }

//     // If both parses failed, return an error
//     match (candidate_colon_keyword, candidate_whitespace_keyword) {
//         (None, None) => bail!("Input string contains no keyword: {}", input),
//         (None, Some(bs)) => bail!("Keyword not recognized : {}", bs),
//         (Some(bc), None) => bail!("Keyword not recognized before colon: {}", bc),
//         (Some(bc), Some(bs)) => bail!(fmt_err!(both_potential_keywords_invalid, bs, bc))
//     }
// }

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
