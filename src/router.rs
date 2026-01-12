use miette::IntoDiagnostic;

use crate::calculation::unified::{collapse_cost, collapse_tokens, fold, make_primitives};
use crate::calculation::usage_report::UsageReport;
use crate::cli::{Commands, Grouping, Metric, SumArgs};

use crate::app::App;
use crate::io::unified_dtos::UnifiedBucketByTime;

/// We will see...
pub fn does_the_thing(
    ctx: &App,
    unified_usages: Vec<UnifiedBucketByTime>,
) -> miette::Result<UsageReport> {
    let primitive_form = make_primitives(unified_usages.clone())?;

    let output: UsageReport = match &ctx.cli.command {
        // meter sum.
        Commands::Sum(args) => match args {
            SumArgs {
                metric: Metric::Tokens,
                group_by: Some(Grouping::Model),
            } => collapse_tokens(primitive_form).into(),

            SumArgs {
                metric: Metric::Tokens,
                group_by: None,
            } => fold(collapse_tokens(primitive_form)).into(),

            SumArgs {
                metric: Metric::Cost,
                group_by: Some(Grouping::Model),
            } => collapse_cost(primitive_form).into(),

            SumArgs {
                metric: Metric::Cost,
                group_by: None,
                ..
            } => fold(collapse_cost(primitive_form)).into(),
        },

        // meter raw.
        Commands::Raw => {
            let json = if ctx.cli.unformatted {
                serde_json::to_string(&unified_usages).into_diagnostic()?
            } else {
                serde_json::to_string_pretty(&unified_usages).into_diagnostic()?
            };

            UsageReport::Raw(json)
        }
    };

    Ok(output)
}
