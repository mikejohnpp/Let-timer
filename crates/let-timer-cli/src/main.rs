use clap::{Parser, Subcommand};
use let_timer_core::{Command, IpcClient};

#[derive(Parser)]
#[command(name = "let-timer", about = "CLI for let-timer daemon")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Start,
    Stop,
    End,
}

fn parse_cli_command_to_protocol(command: &Commands) -> Command {
    match command {
        Commands::Start => Command::Start,
        Commands::Stop => Command::Stop,
        Commands::End => Command::End,
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let cmd_to_protocol = parse_cli_command_to_protocol(&cli.command);

    let path = std::path::Path::new("/tmp/let-timer.sock");
    let mut client = match IpcClient::connect(path).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("không kết nối được daemon: {e}");
            std::process::exit(1);
        }
    };
    match client.request(&cmd_to_protocol).await {
        Ok(resp) => println!("{}", resp.data),
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}
