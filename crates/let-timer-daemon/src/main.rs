use let_timer_core::{Command, IPCServer, Response};

fn invoke_command(cmd: Command) -> Response {
    Response {
        data: format!("{:?}", cmd),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = "/tmp/let-timer.sock";
    let server = IPCServer::new(path, invoke_command).await?;
    server.run().await?;
    println!("daemon exited cleanly");
    Ok(())
}
