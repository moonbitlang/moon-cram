/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

use anyhow::Result;

use super::generator::TestCaseGenerator;
use super::generator::UpdateGenerator;
use super::outcome::OutcomeTestGenerator;
use crate::formatln;
use crate::outcome::Outcome;
use crate::parsers::cram::DEFAULT_CRAM_INDENTION;

/// Update [`crate::testcase::TestCase`]s in an existing Cram document
pub struct CramUpdateGenerator {
    pub indention: usize,
}

impl CramUpdateGenerator {
    pub fn new(indention: usize) -> Self {
        Self { indention }
    }
}

impl Default for CramUpdateGenerator {
    fn default() -> Self {
        Self::new(DEFAULT_CRAM_INDENTION)
    }
}

impl UpdateGenerator for CramUpdateGenerator {
    fn generate_update(
        &self,
        original_document: &str,
        outcomes: &[&Outcome],
        /* testcase: &[&TestCase],
        outputs: &[&Output], */
    ) -> Result<String> {
        if outcomes.is_empty() {
            return Ok(original_document.into());
        }

        let indent = " ".repeat(self.indention);
        let mut testcases = vec![];
        for outcome in outcomes {
            let mut testcase = if outcome.testcase.title.is_empty() {
                "".into()
            } else {
                formatln!("{}", outcome.testcase.title)
            };
            testcase.push_str(&cram_indented(&indent, &outcome.generate_testcase()?));
            testcases.push(testcase);
        }

        Ok(testcases.join("\n\n"))
    }
}

/// Generate a new Cram [`crate::testcase::TestCase`] document from shell
/// expression and it's [`crate::output::Output`]
pub struct CramTestCaseGenerator {
    pub indention: usize,
}

impl CramTestCaseGenerator {
    pub fn new(indention: usize) -> Self {
        Self { indention }
    }
}

impl Default for CramTestCaseGenerator {
    fn default() -> Self {
        Self::new(DEFAULT_CRAM_INDENTION)
    }
}

impl TestCaseGenerator for CramTestCaseGenerator {
    fn generate_testcases(&self, outcomes: &[&Outcome]) -> Result<String> {
        outcomes
            .iter()
            .map(|outcome| {
                let mut rendered = String::new();
                if !outcome.testcase.title.is_empty() {
                    rendered.push_str(&outcome.testcase.title);
                    rendered.push('\n');
                }

                let indent = " ".repeat(self.indention);
                let generated = outcome.generate_testcase()?;
                rendered.push_str(&cram_indented(&indent, &generated));

                Ok(rendered)
            })
            .collect::<Result<Vec<_>>>()
            .map(|result| result.join("\n\n"))
    }
}

fn cram_indented(indent: &str, from: &str) -> String {
    if from.is_empty() {
        "".into()
    } else {
        from.strip_suffix('\n')
            .unwrap_or(from)
            .split('\n')
            .map(|line| format!("{}{}", indent, line))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n"
    }
}

#[cfg(test)]
mod tests {
    use super::CramTestCaseGenerator;
    use super::CramUpdateGenerator;
    use super::cram_indented;
    use crate::diff::Diff;
    use crate::diff::DiffLine;
    use crate::escaping::Escaper;
    use crate::formatln;
    use crate::generators::generator::UpdateGenerator;
    use crate::generators::generator::tests::UpdateGeneratorTest;
    use crate::generators::generator::tests::run_update_generator_tests;
    use crate::generators::generator::tests::standard_testcase_generator_test_suite;
    use crate::outcome::Outcome;
    use crate::parsers::parser::ParserType;
    use crate::test_expectation;
    use crate::testcase::TestCase;
    use crate::testcase::TestCaseError;
    use crate::validation::OutputBody;
    use crate::validation::ValidationBody;
    use crate::validation::ValidationFailure;

    #[test]
    fn cram_indented_preserves_trailing_empty_output_line() {
        let from = "$ printf \"a\\n\\n\"\na\n\n";

        assert_eq!(
            cram_indented("  ", from),
            "  $ printf \"a\\n\\n\"\n  a\n  \n"
        );
    }

    #[test]
    fn update_generator_preserves_trailing_empty_output_line_before_separator() {
        let output_with_trailing_empty_line = Outcome {
            location: None,
            testcase: TestCase {
                shell_expression: r#"printf "a\n\n""#.to_string(),
                line_number: 1,
                ..Default::default()
            },
            output: ("a\n\n", "").into(),
            result: Err(TestCaseError::ValidationFailed(
                ValidationFailure::MalformedOutput(Diff::new(vec![DiffLine::UnexpectedLines {
                    lines: vec![(0, b"a\n".to_vec()), (1, b"\n".to_vec())],
                }])),
            )),
            escaping: Escaper::default(),
            format: ParserType::Cram,
        };
        let following_testcase = Outcome {
            location: None,
            testcase: TestCase {
                shell_expression: "true".to_string(),
                line_number: 4,
                ..Default::default()
            },
            output: ("", "").into(),
            result: Ok(()),
            escaping: Escaper::default(),
            format: ParserType::Cram,
        };

        let generator = CramUpdateGenerator::default();
        let result = generator
            .generate_update("", &[&output_with_trailing_empty_line, &following_testcase])
            .unwrap();

        assert!(result.contains("  a\n  \n"));
        assert!(!result.contains("  a\n\n\n  $ true"));
    }

    #[test]
    fn test_update_generator() {
        let tests: &[(&str, UpdateGeneratorTest)] = &[
            (
                "simple_unchanged",
                UpdateGeneratorTest {
                    original_document: "This is a test\n  $ the command\n  an expectation\n",
                    outcomes: vec![Outcome {
                        location: None,
                        testcase: TestCase {
                            title: "This is a test".to_string(),
                            shell_expression: "the command".to_string(),
                            body: ValidationBody::Output(OutputBody {
                                expectations: vec![test_expectation!(
                                    "equal",
                                    "an expectation",
                                    false,
                                    false,
                                    "an expectation"
                                )],
                            }),
                            exit_code: None,
                            line_number: 234,
                            config: Default::default(),
                            ..Default::default()
                        },
                        output: ("an expectation\n", "").into(),
                        result: Ok(()),
                        escaping: Escaper::default(),
                        format: ParserType::Cram,
                    }],
                },
            ),
            (
                "complex_unchanged",
                UpdateGeneratorTest {
                    original_document: "This is a test\n  $ the command\n  line * (glob+)\n",
                    outcomes: vec![Outcome {
                        testcase: TestCase {
                            title: "This is a test".to_string(),
                            shell_expression: "the command".to_string(),
                            body: ValidationBody::Output(OutputBody {
                                expectations: vec![test_expectation!(
                                    "glob",
                                    "line *",
                                    false,
                                    true,
                                    "line * (glob+)"
                                )],
                            }),
                            exit_code: None,
                            line_number: 234,
                            ..Default::default()
                        },
                        output: ("line 1\nline 2\nline 3\n", "").into(),
                        result: Ok(()),
                        location: None,
                        escaping: Escaper::default(),
                        format: ParserType::Cram,
                    }],
                },
            ),
            (
                "updated_output",
                UpdateGeneratorTest {
                    original_document: "This is a test\n  $ the command\n  an expectation\n",
                    outcomes: vec![Outcome {
                        testcase: TestCase {
                            title: "This is a test".to_string(),
                            shell_expression: "the command".to_string(),
                            body: ValidationBody::Output(OutputBody {
                                expectations: vec![test_expectation!("equal", "an expectation")],
                            }),
                            exit_code: None,
                            line_number: 234,
                            ..Default::default()
                        },
                        output: ("new output\n", "").into(),
                        result: Err(TestCaseError::ValidationFailed(
                            ValidationFailure::MalformedOutput(Diff::new(vec![
                                DiffLine::UnmatchedExpectation {
                                    index: 0,
                                    expectation: test_expectation!("equal", "an expectation"),
                                },
                                DiffLine::UnexpectedLines {
                                    lines: vec![(0, formatln!("new output").as_bytes().to_vec())],
                                },
                            ])),
                        )),
                        location: None,
                        escaping: Escaper::default(),
                        format: ParserType::Cram,
                    }],
                },
            ),
        ];

        let generator = CramUpdateGenerator::default();
        run_update_generator_tests(generator, "cram", tests);
    }

    #[test]
    fn test_testcase_generator() {
        let generator = CramTestCaseGenerator::default();
        standard_testcase_generator_test_suite(generator, "cram");
    }
}
