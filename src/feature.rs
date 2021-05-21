use crate::step::{Keyword, Step, FeatureItemKeyword};
use crate::Str;

pub trait ParseTrimmedLines<'a> {
    fn from_lines(lines: impl Iterator<Item=&'a str>) -> Option<Self> where Self: Sized;
}

pub trait ParseStr<'a> {
    fn from_str(input: &'a str) -> Option<Self> where Self: Sized;
}

impl<'a,T> ParseStr<'a> for T where T : ParseTrimmedLines<'a> {
    fn from_str(input: &'a str) -> Option<Self> where Self: Sized {
        Self::from_lines(input.lines().map(|l| l.trim()).filter(|l| !l.is_empty()))
    }
}

#[derive(Debug)]
pub struct ExampleRow<'a> {
    pub entries: Vec<Str<'a>>
}

impl<'a> ExampleRow<'a> {
    pub fn from_str(input: Str<'a>) -> Option<Self> {
        let mut entries = input.split('|').skip(1).map(str::trim).collect::<Vec<_>>();
        entries.pop()?;
        Some(ExampleRow {
            entries
        })
    }
}

// pub struct ExampleTable<'a> {
//     labels: Vec<Str<'a>>,
//     examples: Vec<ExampleRow<'a>>
// }

// impl<'a> ExampleTable<'a> {
//     pub(crate) fn invariant(&self) -> bool {
//         self.examples.iter().all(|example| example.entries.len() == self.labels.len() )
//     }
// }

#[derive(Debug)]
pub struct Feature<'a> {
    pub name: Str<'a>,
    pub free_text: Vec<Str<'a>>,
    pub items: Vec<FeatureItem<'a>>,
}

impl<'a> ParseTrimmedLines<'a> for Feature<'a> {
    fn from_lines(lines: impl Iterator<Item=&'a str>) -> Option<Self> where Self: Sized {
        let lines = lines.collect::<Vec<_>>();
        let (keyword, name) = lines.get(0)?.split_once(":")?;
        if Keyword::from_str(keyword)? != Keyword::Feature {
            return None;
        }

        let mut i_start = 1;
        let mut free_text = vec![];
        while i_start < lines.len() && !line_begins_feature_item(dbg!(lines[i_start])) {
            println!("Yummy free text!");
            free_text.push(lines[i_start]);
            i_start += 1;
        }

        let mut items = vec![];
        while i_start < lines.len() {
            let mut i_end = i_start + 1;
            while i_end < lines.len() && !line_begins_feature_item(dbg!(lines[i_end])) {
                i_end += 1;
            }
            items.push(FeatureItem::from_lines(lines[i_start..i_end].iter().map(|x| *x))?);
            i_start = i_end;

        };
        Some(Feature {
            name,
            free_text,
            items
        })
    }
}


#[derive(Debug)]
pub struct Scenario<'a> {
    pub name: Str<'a>,
    pub steps: Vec<Step<'a>>
}

pub fn parse_step_list<'a>(lines: impl Iterator<Item=&'a str>) -> Option<Vec<Step<'a>>> {
    lines.map(Step::from_str).collect()
}


// todo - factor this into something that FeatureItem::from_str can reuse
// Introduce another layer of enums for FeatureItemKeywords to accomplish this.
pub fn line_begins_feature_item(line: Str) -> bool {
    if let Some((keyword, _rest)) = line.split_once(":") {
        FeatureItemKeyword::from_str(keyword).is_some()
    } else {
        false
    }
}

impl<'a> ParseTrimmedLines<'a> for Scenario<'a> {
    fn from_lines(mut lines: impl Iterator<Item=Str<'a>>) -> Option<Self> {
        let name = lines.next()?;
        let steps = parse_step_list(lines)?;
        Some(Scenario {
            name,
            steps
        })
    }
}

#[derive(Debug)]
pub struct ScenarioOutline<'a> {
    pub name: Str<'a>,
    pub steps: Vec<Step<'a>>,
    labels: ExampleRow<'a>,
    examples: Vec<ExampleRow<'a>>
}

impl<'a> ScenarioOutline<'a> {
    fn from_lines(name: Str<'a>, mut lines: impl Iterator<Item=Str<'a>>) -> Option<Self> {
        let mut steps = vec![];
        let mut examples = vec![];
        loop {
            let line = lines.next()?;
            if let Some((keyword, rest)) = line.split_once(":") {
                if Keyword::from_str(keyword) == Some(Keyword::Examples) && rest.is_empty() {
                    break;
                }
                else {
                    dbg!("Malformed 'Examples:' line:\n{}",line);
                    return None;
                }
            }
            else {
                steps.push(Step::from_str(line)?);
            }
        };
        let label_line = lines.next()?;
        let labels = ExampleRow::from_str(label_line)?;

        for line in lines {
            examples.push(ExampleRow::from_str(line)?);
        }

        Some(ScenarioOutline {
            name,
            steps,
            labels,
            examples
        })
    }
}

#[derive(Debug)]
pub enum FeatureItem<'a> {
    Scenario(Scenario<'a>),
    ScenarioOutline(ScenarioOutline<'a>)
}

impl<'a> ParseTrimmedLines<'a> for FeatureItem<'a> {
    fn from_lines(mut lines: impl Iterator<Item=&'a str>) -> Option<Self> {
        let (keyword, rest) = lines.next()?.split_once(":")?;
        let keyword = keyword.trim();
        let name = rest.trim();
        match FeatureItemKeyword::from_str(keyword) {
            Some(FeatureItemKeyword::Scenario) => match Scenario::from_lines(name, lines) {
                Some(scenario) => Some(FeatureItem::Scenario(scenario)),
                None => None   
            },
            Some(FeatureItemKeyword::ScenarioOutline) => match ScenarioOutline::from_lines(name, lines) {
                Some(outline) => Some(FeatureItem::ScenarioOutline(outline)),
                None => None
            }
            None => None
        }
    }
}

impl<'a> Scenario<'a> {
    fn from_lines(name: Str<'a>, lines: impl Iterator<Item=&'a str>) -> Option<Self> where Self: Sized {
        let steps = parse_step_list(lines)?;
        Some(Scenario {
            name,
            steps
        })
    }
}
