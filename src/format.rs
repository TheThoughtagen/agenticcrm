use anyhow::Result;
use serde::Serialize;

#[derive(clap::ValueEnum, Clone, Debug, Default)]
pub enum OutputFormat {
    #[default]
    Human,
    Json,
}

/// Output data in the specified format.
/// For Human: uses the Display implementation.
/// For Json: uses serde_json serialization.
pub fn output<T: Serialize + std::fmt::Display>(data: &T, format: &OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Human => {
            println!("{data}");
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(data)?;
            println!("{json}");
        }
    }
    Ok(())
}

/// Output a list of items in the specified format.
/// For Human: prints each item using Display, then a count line.
/// For Json: serializes the entire vec.
pub fn output_list<T: Serialize + std::fmt::Display>(
    data: &[T],
    format: &OutputFormat,
    count_label: &str,
) -> Result<()> {
    match format {
        OutputFormat::Human => {
            for item in data {
                println!("{item}");
            }
            println!("\n{} {count_label}", data.len());
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(data)?;
            println!("{json}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt;

    #[derive(Serialize)]
    struct TestData {
        name: String,
        value: i32,
    }

    impl fmt::Display for TestData {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}: {}", self.name, self.value)
        }
    }

    #[test]
    fn test_output_human() {
        // Just verify it doesn't panic
        let data = TestData {
            name: "test".to_string(),
            value: 42,
        };
        output(&data, &OutputFormat::Human).unwrap();
    }

    #[test]
    fn test_output_json() {
        let data = TestData {
            name: "test".to_string(),
            value: 42,
        };
        // Should not panic, output valid JSON
        output(&data, &OutputFormat::Json).unwrap();
    }

    #[test]
    fn test_output_list_human() {
        let data = vec![
            TestData {
                name: "a".to_string(),
                value: 1,
            },
            TestData {
                name: "b".to_string(),
                value: 2,
            },
        ];
        output_list(&data, &OutputFormat::Human, "items").unwrap();
    }

    #[test]
    fn test_output_list_json() {
        let data = vec![
            TestData {
                name: "a".to_string(),
                value: 1,
            },
            TestData {
                name: "b".to_string(),
                value: 2,
            },
        ];
        output_list(&data, &OutputFormat::Json, "items").unwrap();
    }
}
