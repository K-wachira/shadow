use clap::{Parser, Subcommand};

/// Simple shadow program
#[derive(Parser)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Load new Json logs from iCloud to the DB.
    Ingest,
    
    /// Ask shadow for a recommendation
    Ask { query: Option<String> },
    
    /// Log a new entry directly into the DB 
    Log { content: Option<String> },
    
    /// Show recent logs ( Should take an argument )
    Recent  { content: Option<i32> },
    
    /// Show stats and patterns
    Stats,

}
// Commands::Ingest => ingest(),
// Commands::Ask { query } => ask(query),
// Commands::Log { content } => log(content),
// Commands::Recent => recent(),
// Commands::Stats => stats(),