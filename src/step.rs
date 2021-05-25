use crate::{feature::ParseStr, Str};
use anyhow::{bail, Context, Error, Result};

#[derive(Debug)]
pub struct Step<'a> {
    // invariant: literals.len() == variables.len() + 1;
    keyword: StepKeyword,
    literals: Vec<Str<'a>>,
    variables: Vec<Str<'a>>,
}

impl<'a> Step<'a> {
    pub fn arity(&self) -> usize {
        self.variables.len()
    }

    // pub fn content(&self) -> (&[Str], &[Str]) {
    //     (&self.literals, &self.variables)
    // }

    // pub(crate) fn invariant(&self) -> bool {
    //     self.variables.len() == self.literals.len() - 1
    // }

    pub fn from_str(input: Str<'a>) -> Result<Step<'a>> {
        let input = input.trim();
        let (keyword, text) = input
            .split_once(" ")
            .context("Step line contained no whitespace to delimit keyword")?;
        let keyword = StepKeyword::from_str(keyword.trim())?;
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

#[derive(Debug, PartialEq, Eq)]
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
