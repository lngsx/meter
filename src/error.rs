use miette::Diagnostic;
use thiserror::Error;

// My ideas are...
// meter::parse -> cli argument parsing/validation.
// meter::config -> environment, credentials, settings.
// meter::api -> http/provider errors (future).
// meter::api::request -> for example.
// meter::internal -> "This thing shouldn't happen" errors (future).
// meter::internal::unexpected -> for example.

#[derive(Debug, Error, Diagnostic)]
pub enum Error {
    #[error("Invalid duration format: expected integer before unit, got '{0}'")]
    #[diagnostic(
        code(meter::parse::duration),
        help("Please provide a valid integer, like '1' or '30'.")
    )]
    InvalidDuration(String),

    #[error(
        "Unsupported time unit. Currently only 'd' (days) suffix (e.g., '1d') is supported, got '{0}'."
    )]
    #[diagnostic(
        code(meter::parse::time_unit),
        help("Try using 'd' for days, example: '2d'.")
    )]
    UnsupportedTimeUnit(String),

    /// Kaboom
    #[error("Anthropic API key not found.")]
    #[diagnostic(
        code(meter::config::api_key),
        help(
"Ensure that the ANTHROPIC_ADMIN_API_KEY environment variable is set with your API key.\n\
Try running `echo $ANTHROPIC_ADMIN_API_KEY` to check if it's present or restart your shell.
            "
        ),
        url("https://platform.claude.com/docs/en/build-with-claude/administration-api")
    )]
    AnthropicKeyNotFound,

    #[error("OpenAI API key not found.")]
    #[diagnostic(
        code(meter::config::api_key),
        help(
"Ensure that the ANTHROPIC_ADMIN_API_KEY environment variable is set with your API key.\n\
Try running `echo $ANTHROPIC_ADMIN_API_KEY` to check if it's present or restart your shell.
            "
        ),
        url("https://platform.claude.com/docs/en/build-with-claude/administration-api")
    )]
    OpenaiKeyNotFound,

    /// I am too lazy to add every model to the table, so this is the price for users.
    /// This will take them to the GitHub issue form, prefilled.
    /// The good thing is, at least I have something to play with in the miette error report.
    #[error(
        "🙏 Sorry! The system is fine, but pricing information is missing for:\nmodel: {model:?}\ncontext_window: {context_window:?}"
    )]
    #[diagnostic(
        code(meter::pricing::missing_configuration),
        help("Please report this using the URL above so we can add the pricing. Thank you!"),
        url(
            "https://github.com/lngsx/meter/issues/new?title=%F0%9F%92%B8%20Missing%20pricing%20configuration&body=model: {model:?}%0Acontext_window: {context_window:?}"
        )
    )]
    PricingNotFound {
        model: String,
        context_window: String,
    },
}
