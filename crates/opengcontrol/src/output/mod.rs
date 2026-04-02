use clap::ValueEnum;

pub mod json;

#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub enum OutputFormat {
    #[default]
    Human,
    Json,
}
