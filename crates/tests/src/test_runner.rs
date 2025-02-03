use miette::IntoDiagnostic;
use miette::{SourceOffset, SourceSpan};
use std::path::Path;

use crate::test_builder::TestBuilder;

pub struct TestElement {
    pub test: String,
    pub expected_output: String,
    #[allow(dead_code)]
    pub span: SourceSpan,
}

pub struct Tests {
    tests: Vec<TestElement>,
}

impl Tests {
    pub fn load_from_file(path: &Path) -> miette::Result<Self> {
        let content = std::fs::read_to_string(path).into_diagnostic()?;
        let mut tests = Vec::new();

        let mut current_test = String::new();
        let mut current_output = String::new();
        let mut start_line = 0;
        let mut current_line = 0;
        let mut source_offset = SourceOffset::from_location(&content, 0, 0);

        for line in content.lines() {
            source_offset = SourceOffset::from_location(&content, current_line, 0);

            current_line += 1;

            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }

            if line.starts_with('>') {
                if !current_test.is_empty() && !current_output.is_empty() {
                    // Empty output is signified by a single % character
                    if current_output == "%empty" {
                        current_output = String::new();
                    }
                    tests.push(TestElement {
                        test: std::mem::take(&mut current_test),
                        expected_output: std::mem::take(&mut current_output),
                        span: SourceSpan::new(source_offset, current_line - start_line),
                    });
                }
                if current_test.is_empty() {
                    start_line = current_line;
                }
                if !current_test.is_empty() {
                    current_test.push('\n');
                }
                current_test.push_str(line.trim_start_matches('>').trim());
            } else if !current_test.is_empty() {
                if !current_output.is_empty() {
                    current_output.push('\n');
                }
                current_output.push_str(line);
            }
        }

        // Add final test if exists
        if !current_test.is_empty() && !current_output.is_empty() {
            tests.push(TestElement {
                test: current_test,
                expected_output: current_output,
                span: SourceSpan::new(source_offset, current_line - start_line),
            });
        }

        Ok(Self { tests })
    }

    pub async fn execute(&self) -> miette::Result<()> {
        for test in &self.tests {
            let expected = format!("{}\n", test.expected_output.clone());

            TestBuilder::new()
                .command(&test.test)
                .assert_stdout(&expected)
                .run()
                .await;
        }

        Ok(())
    }
}

#[tokio::test]
async fn tests_from_files() {
    let test_folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-data");

    // read all files from the test folder
    let files = std::fs::read_dir(&test_folder).unwrap();
    for file in files {
        let file = file.unwrap();
        let path = file.path();

        let tests = Tests::load_from_file(&path).unwrap();
        tests.execute().await.unwrap();
    }
}
