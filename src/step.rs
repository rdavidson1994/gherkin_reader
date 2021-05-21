
use crate::{Str, feature::ParseStr};

#[derive(Debug)]
pub struct Step<'a> {
    // invariant: literals.len() == variables.len() + 1;
    keyword: StepKeyword,
    literals: Vec<Str<'a>>,
    variables: Vec<Str<'a>>
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

    pub fn from_str(input: Str<'a>) -> Option<Step<'a>> {
        let input = input.trim();
        let (keyword, text) = input.split_once(" ")?;
        let keyword = StepKeyword::from_str(keyword.trim())?;
        let text = text.trim();
        let mut tokens = text.split(|c| {c == '<' || c == '>'});
        let mut literals = vec![tokens.next()?];
        let mut variables = vec![];
        loop {
            if let Some(variable) = tokens.next() {
                variables.push(variable);
                literals.push(tokens.next()?);
            } else {
                break;
            }
        }
        Some(Step {
            keyword,
            literals,
            variables
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
    Bullet
}

impl StepKeyword {
    pub fn from_str(input: Str) -> Option<StepKeyword> {
        use StepKeyword::*;
        match input {
            "Given" => Some(Given),
            "When" => Some(When),
            "Then" => Some(Then),
            "And" => Some(And),
            "But" => Some(But),
            "*" => Some(Bullet),
            _ => None,
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum FeatureItemKeyword {
    Scenario,
    ScenarioOutline,
}

impl<'a> ParseStr<'a> for FeatureItemKeyword {
    fn from_str(input: &'a str) -> Option<Self> where Self: Sized {
        use FeatureItemKeyword::*;
        match input {
             "Scenario" | "Example" => Some(Scenario),
             "Scenario Outline" | "Scenario Template" => Some(ScenarioOutline),
             _ => None
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum Keyword {
    Feature,
    FeatureItem(FeatureItemKeyword),
    Examples,
    Step(StepKeyword)
}

impl<'a> ParseStr<'a> for Keyword {
    fn from_str(input: &str) -> Option<Self> where Self: Sized {
        use Keyword::*;
        if let Some(fik) = FeatureItemKeyword::from_str(input) {
            Some(FeatureItem(fik))
        } else if let Some(step) = StepKeyword::from_str(input) {
            Some(Step(step))
        } else {
            match input {
                "Feature" => Some(Feature),
                "Examples" | "Scenarios" => Some(Examples),
                _ => None
            }
        }
    }
}