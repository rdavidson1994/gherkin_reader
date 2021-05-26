use crate::step::GherkinLine;
use crate::step::GroupingKeyword;
use crate::Export;
use crate::Str;
use crate::{step::Step, NUnit};
use anyhow::{bail, Context, Result};
use fmt_err;

pub(crate) struct ParseOutcome<'a, T> {
    data: T,
    next_line: Option<GherkinLine<'a>>,
}

fn ok_parsed<'a, T>(data: T, next_line: Option<GherkinLine<'a>>) -> Result<ParseOutcome<'a, T>> {
    Ok(ParseOutcome { data, next_line })
}

pub(crate) trait ParseTrimmedLines<'a> {
    fn from_lines(
        title: &'a str,
        lines: impl Iterator<Item = GherkinLine<'a>>,
    ) -> Result<ParseOutcome<'a, Self>>
    where
        Self: Sized;

    fn from_str_lines(
        title: &'a str,
        lines: impl Iterator<Item = &'a str>,
    ) -> Result<ParseOutcome<'a, Self>>
    where
        Self: Sized,
    {
        Self::from_lines(title, lines.map(GherkinLine::from_str))
    }
}

pub trait ParseStr<'a> {
    fn from_str(input: &'a str) -> Result<Self>
    where
        Self: Sized;
}

#[derive(Debug)]
pub struct ExampleRow<'a> {
    pub entries: Vec<Str<'a>>,
}

impl<'a> ExampleRow<'a> {
    pub fn from_str(input: Str<'a>) -> Result<Self> {
        let mut entries = input.split('|').skip(1).map(str::trim).collect::<Vec<_>>();
        entries.pop().with_context(|| {
            format!(
                "This example row seems to be malformed, containing less than two pipes:\n{}",
                input
            )
        })?;
        Ok(ExampleRow { entries })
    }
}

#[derive(Debug)]
pub struct Feature<'a> {
    pub name: Str<'a>,
    pub free_text: Vec<Str<'a>>,
    pub items: Vec<FeatureItem<'a>>,
}

impl<'a> Feature<'a> {
    pub fn from_str(input: &'a str) -> Result<Self> {
        let mut lines = input
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.starts_with('@') && !l.is_empty() && !l.starts_with('#'));
        let first_line = lines.next().context("Feature file was empty.")?;
        let parsed_line = GherkinLine::from_str(first_line);
        let title = {
            if let GherkinLine::BeginGroup(GroupingKeyword::Feature, title) = parsed_line {
                title
            } else {
                bail!(
                    "Expected to read line of the form \
                    `Feature: <feature name>`, but got this: {:?}",
                    parsed_line
                );
            }
        };
        let parse_outcome = Self::from_str_lines(title, lines)?;
        if let Some(line) = parse_outcome.next_line {
            bail!(
                "Finished parsing content, but then encountered this unexpected line: {:?}",
                line
            );
        }
        Ok(parse_outcome.data)
    }
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

#[derive(Debug)]
pub enum FeatureItem<'a> {
    Bare(Scenario<'a>),
    Outline(ScenarioOutline<'a>),
}

impl<'a> ParseTrimmedLines<'a> for Feature<'a> {
    fn from_lines(
        name: &'a str,
        mut lines: impl Iterator<Item = GherkinLine<'a>>,
    ) -> Result<ParseOutcome<'a, Self>>
    where
        Self: Sized,
    {
        // First, read free text description
        let mut free_text = vec![];
        let (mut group_kw, mut group_name) = loop {
            match lines
                .next()
                .context("Feature terminated without any scenarios.")?
            {
                GherkinLine::FreeText(text) => {
                    free_text.push(text);
                }
                GherkinLine::Tags(tags) => {
                    bail!("Tags aren't supported yet: {:?}", tags);
                }
                GherkinLine::BeginGroup(group_kw, group_name) => {
                    break (group_kw, group_name);
                }
                bad_line => {
                    bail!(
                        "Unexpected content in text description for feature `{}` - `{:?}`",
                        name,
                        bad_line
                    )
                }
            }
        };
        let mut items = vec![];

        loop {
            let line = match group_kw {
                GroupingKeyword::ScenarioOutline => {
                    let ParseOutcome { data, next_line } =
                        ScenarioOutline::from_lines(group_name, &mut lines).context(format!(
                            "Failed to parse Scenario Outline `{}` in feature {}`",
                            group_name, name
                        ))?;
                    items.push(FeatureItem::Outline(data));
                    next_line
                }
                GroupingKeyword::Scenario => {
                    let ParseOutcome { data, next_line } =
                        Scenario::from_lines(group_name, &mut lines)?;
                    items.push(FeatureItem::Bare(data));
                    next_line
                }
                _ => {
                    bail!(
                        "Unexpected keyword at top level of feature: `_{:?}_ {}`",
                        group_kw,
                        group_name
                    );
                }
            };

            if let Some(line) = line {
                match line {
                    GherkinLine::Tags(_) => {
                        bail!("Tags aren't supported yet: {:?}", line)
                    }
                    GherkinLine::BeginGroup(k, n) => {
                        group_kw = k;
                        group_name = n;
                    }
                    _ => {
                        bail!(
                        "Unexpected content encountered while parsing items of Feature `{}` - `{:?}",
                        name, line
                    )
                    }
                }
            } else {
                break;
            }
        }

        // Then, read items

        let feature = Feature {
            name,
            free_text,
            items,
        };

        ok_parsed(feature, None)
    }
}

#[derive(Debug)]
pub struct Scenario<'a> {
    pub name: Str<'a>,
    pub steps: Vec<Step<'a>>,
}

impl<'a> ParseTrimmedLines<'a> for Scenario<'a> {
    fn from_lines(
        name: &'a str,
        mut lines: impl Iterator<Item = GherkinLine<'a>>,
    ) -> Result<ParseOutcome<'a, Self>> {
        let mut steps = vec![];
        use GherkinLine::*;
        let terminating_line = loop {
            match lines.next() {
                Some(StepLine(kw, step_text)) => {
                    let step = Step::new(kw, step_text)
                        .context(fmt_err!(bad_step, kw, step_text, name))?;
                    steps.push(step);
                }
                other_line => {
                    break other_line;
                }
            }
        };

        let scenario = Scenario { name, steps };

        Ok(ParseOutcome {
            data: scenario,
            next_line: terminating_line,
        })
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

#[derive(Debug)]
pub struct ExampleBlock<'a> {
    examples: Vec<ExampleRow<'a>>,
    labels: ExampleRow<'a>,
}

impl<'a> ParseTrimmedLines<'a> for ExampleBlock<'a> {
    fn from_lines(
        title: &'a str,
        mut lines: impl Iterator<Item = GherkinLine<'a>>,
    ) -> Result<ParseOutcome<'a, Self>>
    where
        Self: Sized,
    {
        use GherkinLine::*;
        // Ensure we are reading `Examples:` and not `Examples: Some other junk`
        if !title.trim().is_empty() {
            bail!(
                "`Examples:` or `Scenarios:` blocks can't have a title, but this one was given: {}",
                title
            )
        }

        let label_line = lines
            .next()
            .context("Expected to find the labels for an example table, but got EOF.")?;
        let labels = match label_line {
            GherkinLine::ExampleEntry(row) => ExampleRow::from_str(row).context(format!(
                "Couldn't parse this row of labels for an example table: `{:?}`",
                label_line
            ))?,
            _ => bail!(
                "Expected to find labels for a data table, got this instead: {:?}",
                label_line
            ),
        };
        let mut examples = vec![];
        let terminator = loop {
            match lines.next() {
                Some(line) => match line {
                    BeginGroup(_, _) => {
                        break Some(line);
                    }
                    ExampleEntry(row) => {
                        let example_row = ExampleRow::from_str(row)
                            .context(format!("Failed to read example row : `{}`", row))?;
                        examples.push(example_row);
                    }
                    _ => {
                        bail!("Did not expect this line inside data table: `{:?}`", line);
                    }
                },
                None => {
                    break None;
                }
            }
        };

        let example_block = ExampleBlock { examples, labels };
        ok_parsed(example_block, terminator)
    }
}

#[derive(Debug)]
pub struct ScenarioOutline<'a> {
    pub name: Str<'a>,
    pub steps: Vec<Step<'a>>,
    pub example_blocks: Vec<ExampleBlock<'a>>,
}

impl<'a> ParseTrimmedLines<'a> for ScenarioOutline<'a> {
    fn from_lines(
        name: &'a str,
        mut lines: impl Iterator<Item = GherkinLine<'a>>,
    ) -> Result<ParseOutcome<'a, Self>>
    where
        Self: Sized,
    {
        use GherkinLine::*;

        let mut steps = vec![];

        let line_after_steps = loop {
            match lines.next() {
                Some(StepLine(kw, step_text)) => {
                    let step = Step::new(kw, step_text)
                        .context(fmt_err!(bad_step, kw, step_text, name))?;
                    steps.push(step);
                }
                Some(tag_line @ Tags(_)) => {
                    bail!("Tags like `{:?}` aren't supported yet", tag_line)
                }
                Some(group_line @ BeginGroup(_, _)) => {
                    break group_line;
                }
                unexpected => {
                    bail!(
                        "Unexpected line `{:?}` while reading steps of scenario outline {}. \
                        Expected to find more steps, or an `Examples:` block.",
                        unexpected,
                        name
                    )
                }
            }
        };

        let mut line = line_after_steps;
        let mut example_blocks = vec![];

        let terminating_line = loop {
            match line {
                tag_line @ Tags(_) => {
                    bail!("Tags like `{:?}` aren't supported yet", tag_line);
                }
                BeginGroup(group_keyword, group_name) => match group_keyword {
                    GroupingKeyword::Examples => {
                        let ParseOutcome { data, next_line } =
                            ExampleBlock::from_lines(group_name, &mut lines).context(format!(
                                "Failed to parse example block #{} in Scenario Outline `{}`",
                                example_blocks.len() + 1,
                                name
                            ))?;
                        example_blocks.push(data);
                        if let Some(next_line) = next_line {
                            line = next_line;
                        } else {
                            break None;
                        }
                    }
                    _ => {
                        break Some(line);
                    }
                },
                _ => {
                    break Some(line);
                }
            }
        };

        let outline = ScenarioOutline {
            name,
            steps,
            example_blocks,
        };

        ok_parsed(outline, terminating_line)
    }
}

impl<'a> Export<NUnit> for ScenarioOutline<'a> {
    fn export(&self, _export_format: NUnit) -> String {
        let mut output = String::new();
        for block in &self.example_blocks {
            for example in &block.examples {
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
        }
        let x = format!("   public void {}(", pascal(self.name));
        output.push_str(&x);
        let mut first_arg = true;
        for arg in &self.example_blocks[0].labels.entries {
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

impl<'a, T> Export<T> for FeatureItem<'a>
where
    Scenario<'a>: Export<T>,
    ScenarioOutline<'a>: Export<T>,
{
    fn export(&self, export_format: T) -> String {
        match self {
            FeatureItem::Bare(x) => x.export(export_format),
            FeatureItem::Outline(x) => x.export(export_format),
        }
    }
}
