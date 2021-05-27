use crate::{
    feature::{ExampleBlock, ScenarioOutline},
    Str,
};

pub trait Export<T> {
    fn export(&self, export_format: T) -> String;
}
pub trait Language {
    type ArgTypes;
}

pub trait TestFramework {
    type Lang: Language;
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum CSType {
    Bool,
    Int64,
    Double,
    String,
}

impl CSType {
    fn lowest_common_type(self, other: CSType) -> CSType {
        use CSType::*;
        match (self, other) {
            // Types remain the same unless contradicted
            (x, y) if x == y => x,
            // If a contradiction occurs, we default back to string
            _ => String,
        }
    }
    fn from(input: &str) -> CSType {
        if input.parse::<i64>().is_ok() {
            CSType::Int64
        } else if input.parse::<f64>().is_ok() {
            CSType::Double
        } else if input.parse::<bool>().is_ok() {
            CSType::Bool
        } else {
            CSType::String
        }
    }

    fn to_str(self) -> &'static str {
        match self {
            CSType::Bool => "bool",
            CSType::Int64 => "long",
            CSType::Double => "double",
            CSType::String => "string",
        }
    }
}
pub struct NUnit;
pub fn camel(input: Str) -> String {
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

pub fn pascal(input: Str) -> String {
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
