use crate::Export;
use crate::Str;
use crate::{
    step::{FeatureItemKeyword, Keyword, Step},
    NUnit,
};

pub trait ParseTrimmedLines<'a> {
    fn from_lines(lines: impl Iterator<Item = &'a str>) -> Option<Self>
    where
        Self: Sized;
}

pub trait ParseStr<'a> {
    fn from_str(input: &'a str) -> Option<Self>
    where
        Self: Sized;
}

impl<'a, T> ParseStr<'a> for T
where
    T: ParseTrimmedLines<'a>,
{
    fn from_str(input: &'a str) -> Option<Self>
    where
        Self: Sized,
    {
        Self::from_lines(
            input
                .lines()
                .map(|l| l.trim())
                .filter(|l| !l.is_empty() && !l.contains('#') && !l.contains('@')),
        )
    }
}

#[derive(Debug)]
pub struct ExampleRow<'a> {
    pub entries: Vec<Str<'a>>,
}

impl<'a> ExampleRow<'a> {
    pub fn from_str(input: Str<'a>) -> Option<Self> {
        let mut entries = input.split('|').skip(1).map(str::trim).collect::<Vec<_>>();
        entries.pop()?;
        Some(ExampleRow { entries })
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

fn pascal(input: Str) -> String {
    let mut output = String::new();
    for word in input.split(|c: char| !c.is_alphanumeric()) {
        let mut chars = word.chars();
        if let Some(first_char) = chars.next() {
            let first_upper = first_char.to_uppercase();
            output.extend(first_upper);
            output.extend(chars);
        }
    }
    output
}

impl<'a> Export<NUnit> for Feature<'a> {
    fn export(&self, _nunit: NUnit) -> String {
        let mut output = String::new();
        output.push_str("[TestFixture]\n");
        output.push_str("public class ");
        output.push_str(&pascal(self.name));
        output.push_str("\n");
        output.push_str("{\n");

        for item in &self.items {
            output.push_str(&item.export(NUnit))
        }

        output.push_str("\n}");
        output
    }
}

impl<'a> ParseTrimmedLines<'a> for Feature<'a> {
    fn from_lines(lines: impl Iterator<Item = &'a str>) -> Option<Self>
    where
        Self: Sized,
    {
        let lines = lines.collect::<Vec<_>>();
        let (keyword, name) = lines.get(0)?.split_once(":")?;
        if Keyword::from_str(keyword)? != Keyword::Feature {
            return None;
        }

        let mut i_start = 1;
        let mut free_text = vec![];
        while i_start < lines.len() && !line_begins_feature_item(lines[i_start]) {
            free_text.push(lines[i_start]);
            i_start += 1;
        }

        let mut items = vec![];
        while i_start < lines.len() {
            let mut i_end = i_start + 1;
            while i_end < lines.len() && !line_begins_feature_item(lines[i_end]) {
                i_end += 1;
            }
            items.push(FeatureItem::from_lines(
                lines[i_start..i_end].iter().map(|x| *x),
            )?);
            i_start = i_end;
        }
        Some(Feature {
            name,
            free_text,
            items,
        })
    }
}

#[derive(Debug)]
pub struct Scenario<'a> {
    pub name: Str<'a>,
    pub steps: Vec<Step<'a>>,
}

impl<'a> ParseTrimmedLines<'a> for Scenario<'a> {
    fn from_lines(mut lines: impl Iterator<Item = Str<'a>>) -> Option<Self> {
        let name = lines.next()?;
        let steps = parse_step_list(lines)?;
        Some(Scenario { name, steps })
    }
}

impl<'a> Export<NUnit> for Scenario<'a> {
    fn export(&self, _export_format: NUnit) -> String {
        let mut output = String::new();
        output.push_str("   [Test]\n");
        let x = format!("   public void {}()\n", pascal(self.name));
        output.push_str(&x);
        output.push_str("   {\n");
        output.push_str("\n");
        output.push_str("   }\n");
        output
    }
}

pub fn parse_step_list<'a>(lines: impl Iterator<Item = &'a str>) -> Option<Vec<Step<'a>>> {
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

#[derive(Debug)]
pub struct ScenarioOutline<'a> {
    pub name: Str<'a>,
    pub steps: Vec<Step<'a>>,
    labels: ExampleRow<'a>,
    examples: Vec<ExampleRow<'a>>,
}

impl<'a> ScenarioOutline<'a> {
    fn from_lines(name: Str<'a>, mut lines: impl Iterator<Item = Str<'a>>) -> Option<Self> {
        let mut steps = vec![];
        let mut examples = vec![];
        loop {
            let line = lines.next()?;
            if line == "Examples:" {
                break;
            } else {
                steps.push(Step::from_str(line)?);
            }
        }
        let label_line = lines.next()?;
        let labels = ExampleRow::from_str(label_line)?;

        for line in lines {
            examples.push(ExampleRow::from_str(line)?);
        }

        Some(ScenarioOutline {
            name,
            steps,
            labels,
            examples,
        })
    }
}

impl<'a> Export<NUnit> for ScenarioOutline<'a> {
    fn export(&self, _export_format: NUnit) -> String {
        let mut output = String::new();
        for example in &self.examples {
            output.push_str("   [TestCase(");
            let mut first_arg = true;
            for arg in &example.entries {
                if first_arg {
                    first_arg = false;
                } else {
                    output.push_str(", ");
                }
                output.push_str(arg)
            }
            output.push_str(")]\n");
        }
        let x = format!("   public void {}(", pascal(self.name));
        output.push_str(&x);
        let mut first_arg = true;
        for arg in &self.labels.entries {
            if first_arg {
                first_arg = false;
            } else {
                output.push_str(", ");
            }
            output.push_str("string ");
            output.push_str(&pascal(arg))
        }
        output.push_str(")\n");
        output.push_str("   {\n");
        output.push_str("\n");
        output.push_str("   }\n");
        output
    }
}

#[derive(Debug)]
pub enum FeatureItem<'a> {
    Scenario(Scenario<'a>),
    ScenarioOutline(ScenarioOutline<'a>),
}

impl<'a> ParseTrimmedLines<'a> for FeatureItem<'a> {
    fn from_lines(mut lines: impl Iterator<Item = &'a str>) -> Option<Self> {
        let (keyword, rest) = lines.next()?.split_once(":")?;
        let keyword = keyword.trim();
        let name = rest.trim();
        match FeatureItemKeyword::from_str(keyword) {
            Some(FeatureItemKeyword::Scenario) => match Scenario::from_lines(name, lines) {
                Some(scenario) => Some(FeatureItem::Scenario(scenario)),
                None => None,
            },
            Some(FeatureItemKeyword::ScenarioOutline) => {
                match ScenarioOutline::from_lines(name, lines) {
                    Some(outline) => Some(FeatureItem::ScenarioOutline(outline)),
                    None => None,
                }
            }
            None => None,
        }
    }
}

impl<'a, T> Export<T> for FeatureItem<'a>
where
    Scenario<'a>: Export<T>,
    ScenarioOutline<'a>: Export<T>,
{
    fn export(&self, export_format: T) -> String {
        match self {
            FeatureItem::Scenario(x) => x.export(export_format),
            FeatureItem::ScenarioOutline(x) => x.export(export_format),
        }
    }
}

impl<'a> Scenario<'a> {
    fn from_lines(name: Str<'a>, lines: impl Iterator<Item = &'a str>) -> Option<Self>
    where
        Self: Sized,
    {
        let steps = parse_step_list(lines)?;
        Some(Scenario { name, steps })
    }
}
