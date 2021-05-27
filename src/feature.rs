use std::borrow::Cow;
use std::convert::AsRef;
use std::str;

use crate::step::GherkinLine;
use crate::step::GroupingKeyword;
use crate::CSType;
use crate::Export;
use crate::Str;
use crate::{step::Step, NUnit};
use anyhow::{bail, Context, Result};

type ParseOutcome<'a, T> = (T, Option<GherkinLine<'a>>);

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
    pub entries: Vec<Cow<'a, str>>,
}

impl<'a> ExampleRow<'a> {
    pub fn from_str(input: Str<'a>) -> Result<Self> {
        // Record whether any escapes occurred, so that we
        // can go back and replace them.
        let mut ever_escaped = false;
        // Record whether we are escaping the next character,
        // so we can enclose this in the following closure
        let mut escaping = false;
        let mut entries = input
            // For each character, determine if we should split
            // based on the following critieria:
            .split(|x| {
                if escaping {
                    // If we decided previously to escape the next char,
                    // do not split. Set escaping to falso so only *one*
                    // char is escaped.
                    escaping = false;
                    false
                } else if x == '\\' {
                    // If we encounter an unescaped backslash,
                    // begin escaping and don't split.
                    escaping = true;
                    ever_escaped = true;
                    false
                } else if x == '|' {
                    // If we encounter an unescaped pipe, split.
                    true
                } else {
                    // Otherwise don't split.
                    false
                }
            })
            .skip(1)
            .map(|x| Cow::Borrowed(str::trim(x)))
            .collect::<Vec<Cow<'a, str>>>();

        // If we escaped at any point, go back and correct each affected segment
        // so that it contains the unescaped version.
        if ever_escaped {
            for entry in &mut entries {
                if entry.contains("\\|") {
                    *entry = Cow::Owned(entry.replace("\\|", "|"));
                }
            }
        }
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
    pub background: Option<Scenario<'a>>,
    pub tags: Vec<Str<'a>>,
}

impl<'a> Feature<'a> {
    pub fn from_str(input: &'a str) -> Result<Self> {
        let mut lines = input
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !l.starts_with('#'));
        let mut tags = vec![];
        let mut line = lines.next().context("Feature file was empty.")?;
        let title = loop {
            let parsed_line = GherkinLine::from_str(line);
            match parsed_line {
                GherkinLine::Tags(gherkin_tags) => tags.extend(gherkin_tags.into_iter()),
                GherkinLine::BeginGroup(GroupingKeyword::Feature, title) => {
                    break title;
                }
                _ => bail!(
                    "Unexpected content while parsing feature tags\n{tags}\n\
                    Expected `Feature: feature_name` or `@tag_1[...@tag_n]`",
                    tags = line.clone()
                ),
            }
            line = match lines.next() {
                Some(l) => l,
                None => bail!("Unexpected EOF while reading feature tags."),
            };
        };
        let (mut feature, next_line) = Self::from_str_lines(title, lines)?;
        if let Some(line) = next_line {
            bail!(
                "Finished parsing content, but then encountered this unexpected line: {:?}",
                line
            );
        }
        feature.tags = tags;
        Ok(feature)
    }
}

fn camel(input: Str) -> String {
    let mut output = String::new();
    let mut iterator = input.split(|c: char| !c.is_alphanumeric());
    let first_word = if let Some(first_word) = iterator.next() {
        first_word
    } else {
        return String::from("");
    };
    output += first_word;
    for word in iterator {
        let mut chars = word.chars();
        if let Some(first_char) = chars.next() {
            let first_upper = first_char.to_uppercase();
            output.extend(first_upper);
            output.extend(chars);
        }
    }
    output
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
        output += "[TestFixture]\n";
        output += "public class ";
        output += &pascal(self.name);
        output += "\n";
        output += "{\n";

        for item in &self.items {
            output += &item.export(NUnit);
        }

        output += "\n}";
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
        let mut background = None;
        let mut free_text = vec![];

        let mut tags: Vec<&str> = vec![];
        let (mut group_kw, mut group_name) = loop {
            match lines
                .next()
                .context("Feature terminated without any scenarios.")?
            {
                GherkinLine::FreeText(text) => {
                    free_text.push(text);
                }
                GherkinLine::Tags(new_tags) => tags.extend(new_tags.into_iter()),
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
        let mut item_tags: Vec<&'a str> = vec![];
        loop {
            let line = match group_kw {
                GroupingKeyword::ScenarioOutline => {
                    let (mut data, next_line) = ScenarioOutline::from_lines(group_name, &mut lines)
                        .context(format!(
                            "Failed to parse Scenario Outline `{}` in feature {}`",
                            group_name, name
                        ))?;
                    data.tags.extend(tags.drain(..));
                    items.push(FeatureItem::Outline(data));
                    next_line
                }
                GroupingKeyword::Scenario => {
                    let (scenario, next_line) = Scenario::from_lines(group_name, &mut lines)?;
                    items.push(FeatureItem::Bare(scenario));
                    next_line
                }
                GroupingKeyword::Background => {
                    let (new_background, next_line) = Scenario::from_lines(group_name, &mut lines)?;
                    background = match background {
                        None => Some(new_background),
                        Some(existing) => {
                            bail!(
                                "While parsing Feature `{feature}`, encountered \
                                Background `{background} - but another background \
                                (`{existing}`) was already declared for that feature.",
                                feature = name,
                                background = new_background.name,
                                existing = existing.name
                            )
                        }
                    };
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
                    GherkinLine::Tags(new_tags) => item_tags.extend(new_tags.into_iter()),
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

        // tags are empty because syntactically,
        // the tags are *outside* the feature.
        // The calling context has them cached and can populate them.
        let feature = Feature {
            name,
            free_text,
            items,
            background,
            tags: vec![],
        };

        Ok((feature, None))
    }
}

#[derive(Debug)]
pub struct Scenario<'a> {
    pub name: Str<'a>,
    pub steps: Vec<Step<'a>>,
    pub tags: Vec<&'a str>,
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
                    let step = Step::new(kw, step_text).context(format!(
                        "Invalid step `{:?} {}` in scenario `{}`",
                        kw, step_text, name
                    ))?;
                    steps.push(step);
                }
                other_line => {
                    break other_line;
                }
            }
        };

        let scenario = Scenario {
            name,
            steps,
            tags: vec![],
        };

        Ok((scenario, terminating_line))
    }
}

impl<'a> Export<NUnit> for Scenario<'a> {
    fn export(&self, _export_format: NUnit) -> String {
        let mut output = String::new();
        output.push_str("    [Test]\n");
        let x = format!("    public void {}()\n", pascal(self.name));
        output.push_str(&x);
        output.push_str("    {\n");
        output.push_str("\n");
        output.push_str("    }\n");
        output
    }
}

#[derive(Debug)]
pub struct ExampleBlock<'a> {
    examples: Vec<ExampleRow<'a>>,
    labels: ExampleRow<'a>,
    tags: Vec<&'a str>,
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
                    BeginGroup(_, _) | Tags(_) => {
                        break Some(line);
                    }
                    ExampleEntry(row) => {
                        let example_row = ExampleRow::from_str(row)
                            .context(format!("Failed to read example row : `{}`", row))?;

                        if labels.entries.len() != example_row.entries.len() {
                            bail!(
                                "Encountered row of length {} in data table, \
                                    which was not consistent with the number of \
                                    labels ({}).\n\
                                    The labels in question are:\n{:?}\n\
                                    The examples provided were:\n{:?}",
                                example_row.entries.len(),
                                labels.entries.len(),
                                labels.entries,
                                example_row.entries
                            )
                        };

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

        // Tags begin as empty since they are specified in the enclosing scenario.
        // The scenario itself will push appropriate tags in from its buffer.
        let example_block = ExampleBlock {
            examples,
            labels,
            tags: vec![],
        };
        Ok((example_block, terminator))
    }
}

#[derive(Debug)]
pub struct ScenarioOutline<'a> {
    pub name: Str<'a>,
    pub steps: Vec<Step<'a>>,
    pub example_blocks: Vec<ExampleBlock<'a>>,
    pub tags: Vec<&'a str>,
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
                    let step = Step::new(kw, step_text).context(format!(
                        "Invalid step `{:?} {}` in scenario `{}`",
                        kw, step_text, name
                    ))?;
                    steps.push(step);
                }
                Some(tag_line @ Tags(_)) => {
                    break tag_line;
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

        let mut tags = vec![];
        let mut line = line_after_steps;
        let mut example_blocks = vec![];

        let terminating_line = loop {
            match line {
                Tags(new_tags) => {
                    tags.extend(new_tags.into_iter());
                    if let Some(next_line) = lines.next() {
                        line = next_line;
                    } else {
                        match tags.last() {
                            Some(last_tag) => {
                                bail!("Unexpected EOF after reading tag @{}", last_tag)
                            }
                            None => bail!("Unexpected EOF after reading tag marker '@'"),
                        }
                    }
                }
                BeginGroup(group_keyword, group_name) => match group_keyword {
                    GroupingKeyword::Examples => {
                        let (mut example_block, next_line) =
                            ExampleBlock::from_lines(group_name, &mut lines).context(format!(
                                "Failed to parse example block #{} in Scenario Outline `{}`",
                                example_blocks.len() + 1,
                                name
                            ))?;
                        example_block.tags.extend(tags.drain(..));
                        example_blocks.push(example_block);
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
            tags: vec![],
        };

        Ok((outline, terminating_line))
    }
}

fn calculate_arg_types(example_blocks: &[ExampleBlock]) -> Vec<CSType> {
    let mut arg_types: Vec<CSType> = vec![];
    let arg_count = match example_blocks.get(0) {
        Some(block) => block.labels.entries.len(),
        None => 0,
    };

    for i in 0..arg_count {
        // Find the best type to use for argument i of this test method
        let best_compatible_type = example_blocks
            // Iterate over all "Examples:" blocks in this scenario outline
            .iter()
            // Lump all the example rows from each block together
            .flat_map(|block| &block.examples)
            .map(|row| {
                row.entries
                    // For each row, examine the ith entry
                    .get(i)
                    .map_or(
                        // If it's absent, asume it's a string
                        CSType::String,
                        // Otherwise, calculate its type.
                        |arg| CSType::from(&arg),
                    )
            })
            // Combine all the calculated types
            .reduce(|x, y| x.lowest_common_type(y))
            // If no types were found (because the blocks were all empty)
            // assume it is of type String.
            .unwrap_or(CSType::String);

        arg_types.push(best_compatible_type);
    }
    arg_types
}

impl NUnit {
    fn escape_literal(&self, literal: &str, add_quotes: bool) -> String {
        // Remove up to one backslash or forward slash from an unquoted literal, in that order of preference.
        let literal = if let Some(stripped_of_backslash) = literal.strip_prefix('\\') {
            stripped_of_backslash
        } else if let Some(stripped_of_forward_slash) = literal.strip_prefix('/') {
            stripped_of_forward_slash
        } else {
            literal
        };
        if add_quotes {
            // When new wrapping quotes and @ are added to bare words,
            // any contained quotes need to be doubled to avoid breaking
            // the verbatime string.
            format!("@\"{}\"", literal.replace('"', "\"\""))
        } else {
            format!("@{}", literal)
        }
    }

    fn interpret_arg(&self, arg: &str, cs_type: CSType) -> String {
        match cs_type {
            CSType::Unknown => format!(
                "0 /*gherkin_reader error: couldn't read argument `{}`*/",
                arg
            ),
            CSType::Bool => {
                let lowercase = arg.to_ascii_lowercase();
                if lowercase == "true" {
                    lowercase
                } else {
                    String::from("false")
                }
            }
            CSType::Int64 => arg.to_owned(),
            CSType::Double => arg.to_owned(),
            CSType::String => {
                let already_quoted = arg.starts_with('"')
                    && arg.ends_with('"')
                    && arg.chars().filter(|&x| x == '"').count() == 2;
                let add_quotes = !already_quoted;
                self.escape_literal(arg, add_quotes)
            }
        }
    }

    fn write_test_case<'a, S: AsRef<str>>(
        &'a self,
        arg_types: &'a [CSType],
        arg_strings: impl Iterator<Item = S>,
        category: &'a str,
    ) -> String {
        let mut output = String::from("    [TestCase(");
        let mut first = true;
        for (&arg_type, arg_string) in arg_types.iter().zip(arg_strings) {
            if !first {
                output += ", ";
            }
            output += &self.interpret_arg(arg_string.as_ref(), arg_type);
            first = false;
        }
        if category != "" {
            output += ", Category=\"";
            output += category;
            output += "\""
        }
        output += ")]\n";
        output
    }
}

impl<'a> Export<NUnit> for ScenarioOutline<'a> {
    fn export(&self, nunit: NUnit) -> String {
        let mut output = String::new();
        let arg_types = calculate_arg_types(&self.example_blocks);
        for block in &self.example_blocks {
            let comma_separated_tags = block.tags.join(",");

            for example in &block.examples {
                let test_case = nunit.write_test_case(
                    &arg_types,
                    example.entries.iter(),
                    &comma_separated_tags,
                );
                output += &test_case;
            }
        }
        output += &format!("    public void {}(", pascal(self.name));
        for (i, arg) in self.example_blocks[0].labels.entries.iter().enumerate() {
            if i != 0 {
                output.push_str(", ");
            }
            output += arg_types.get(i).unwrap_or(&CSType::String).to_str();
            output += " ";
            output += &camel(arg);
        }
        output += ")\n";
        output += "    {\n";

        for step in &self.steps {
            let step_title = step
                .literals
                .iter()
                .map(|&x| pascal(x))
                .reduce(|x, y| x + "___" + &y)
                .unwrap_or(String::from("[Emtpy step text?]"));
            output += &format!(
                "        // {kw:?}({title}(",
                kw = step.keyword,
                title = step_title
            );
            for (i, variable) in step.variables.iter().enumerate() {
                if i != 0 {
                    output += ", "
                }
                output += &camel(variable);
            }
            output += "));\n";
        }
        output += "\n";
        output += "    }\n";
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
