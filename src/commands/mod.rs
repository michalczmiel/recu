use clap::ValueEnum;

pub mod add;
pub mod calendar;
pub mod category;
pub mod config;
pub mod edit;
pub mod ls;
pub mod rename;
pub mod rm;
pub mod treemap;
pub mod undo;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}
