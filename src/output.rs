use clap::ValueEnum;

/// How `graph`, `rules`, `find`, `api`, `new` and `test` print what they found.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum Format {
    Text,
    Json,
}
