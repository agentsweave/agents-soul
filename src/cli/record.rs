use clap::Args;

#[derive(Debug, Clone, Args)]
pub struct RecordCmd {
    #[arg(long)]
    pub workspace: String,
    #[arg(long = "interaction-type")]
    pub interaction_type: String,
    #[arg(long)]
    pub outcome: String,
    #[arg(long)]
    pub notes: Option<String>,
}
