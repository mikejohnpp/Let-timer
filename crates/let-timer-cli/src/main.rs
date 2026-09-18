use clap::{Parser, Subcommand};
use let_timer_core::{Command, IpcClient, Response, SortOrder, TaskStatus, UpdateTask};

#[derive(Parser)]
#[command(name = "let-timer", about = "CLI for let-timer daemon")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Create {
        #[arg(short, long)]
        name: String,
        #[arg(short, long)]
        description: Option<String>,
        #[arg(short, long, default_value = "not-yet")]
        priority: String, // "urgent" | "immediate" | "not-yet"
        #[arg(short = 't', long)]
        estimated_mins: Option<i64>,
    },
    Delete {
        #[arg(short, long)]
        id: i64,
    },
    Edit {
        #[arg(short, long)]
        id: i64,
        #[arg(short, long)]
        name: Option<String>,
        #[arg(short, long)]
        description: Option<String>,
        #[arg(short, long)]
        priority: Option<String>,
    },
    Find {
        query: String,
    },
    List {
        #[arg(long)]
        priority: Option<String>, // "ascending" | "descending"
        #[arg(long)]
        status: Option<String>, // "pending" | "in_progress" | "done"
    },
    Current,
    Start {
        #[arg(short, long)]
        id: Option<i64>,
    },
    Stop,
    Done,
}

fn parse_cli_command_to_protocol(command: &Commands) -> Command {
    match command {
        Commands::Create {
            name,
            description,
            priority,
            estimated_mins,
        } => {
            let new_task = let_timer_core::NewTask {
                name: name.clone(),
                description: description.clone(),
                priority: match priority.as_str() {
                    "urgent" => let_timer_core::Priority::Urgent,
                    "immediate" => let_timer_core::Priority::Immediate,
                    "not-yet" => let_timer_core::Priority::NotYet,
                    _ => panic!("Invalid priority value"),
                },
                estimated_mins: *estimated_mins,
            };
            Command::Create(new_task)
        }
        Commands::Delete { id } => Command::Delete { id: *id },
        Commands::Edit {
            id,
            name,
            description,
            priority,
        } => {
            let update_task = UpdateTask {
                name: name.clone(),
                description: match description {
                    Some(desc) if desc.is_empty() => None,
                    _ => Some(description.clone()),
                },
                priority: priority.as_ref().map(|p| match p.as_str() {
                    "urgent" => let_timer_core::Priority::Urgent,
                    "immediate" => let_timer_core::Priority::Immediate,
                    "not-yet" => let_timer_core::Priority::NotYet,
                    _ => panic!("Invalid priority value"),
                }),
                estimated_mins: None,
            };
            Command::Edit {
                id: *id,
                update: update_task,
            }
        }
        Commands::Find { query } => Command::Find {
            query: query.clone(),
        },
        Commands::List { priority, status } => {
            let sort_priority = priority.as_ref().map(|p| match p.as_str() {
                "ascending" => SortOrder::Ascending,
                "descending" => SortOrder::Descending,
                _ => panic!("Invalid sort order value"),
            });
            let filter_status = status.as_ref().map(|s| match s.as_str() {
                "pending" => let_timer_core::TaskStatus::Pending,
                "in_progress" => let_timer_core::TaskStatus::InProgress,
                "done" => let_timer_core::TaskStatus::Done,
                _ => panic!("Invalid status value"),
            });
            Command::List {
                sort_priority,
                filter_status,
            }
        }
        Commands::Current => Command::Current,
        Commands::Start { id } => Command::Start { id: *id },
        Commands::Stop => Command::Stop,
        Commands::Done => Command::Done,
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
            eprintln!("Failed to connect daemon: {e}");
            std::process::exit(1);
        }
    };
    match client.request(&cmd_to_protocol).await {
        Ok(resp) => match resp {
            Response::Ok(payload) => {
                println!("Success: {:?}", payload);
            }
            Response::OkList(tasks) => {
                println!("Tasks: {:?}", tasks);
            }
            Response::OkEmpty => {
                println!("Success: No data returned");
            }
            Response::Error { message } => {
                eprintln!("Error from daemon: {message}");
                std::process::exit(1);
            }
        },
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}
