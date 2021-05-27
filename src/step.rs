use crate::{
    feature::ParseStr,
    gherkin_tags::{FeatureItemKeyword, StepKeyword},
    Str,
};
use anyhow::{bail, Context, Result};

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
