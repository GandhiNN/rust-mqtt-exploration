use clap::Parser;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[arg(long)]
    pub config: Option<String>,
    #[arg(long)]
    pub duration: Option<i32>,
    #[arg(long)]
    pub output: Option<String>,
}
