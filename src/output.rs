//! Structured output for UnrealDevFlow

use crate::cli::OutputFormat;
use serde::Serialize;

pub fn print_output<T: Serialize>(
    format: &OutputFormat,
    data: &T,
    human_formatter: impl Fn(&T) -> String,
) {
    match format {
        OutputFormat::Json => match serde_json::to_string_pretty(data) {
            Ok(json) => println!("{}", json),
            Err(e) => eprintln!("JSON serialization error: {}", e),
        },
        OutputFormat::Human => {
            println!("{}", human_formatter(data));
        }
    }
}

pub fn print_success(message: &str) {
    println!("✓ {}", message);
}

pub fn print_error(message: &str) {
    eprintln!("✗ {}", message);
}

pub fn print_warning(message: &str) {
    println!("⚠ {}", message);
}

pub fn print_info(message: &str) {
    println!("ℹ {}", message);
}
