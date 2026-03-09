mod coordinator;
mod worker;
mod messages;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 && args[1] == "worker" {
        worker::run_worker().await.unwrap();
    } else {
        coordinator::run_coordinator().await.unwrap();
    }
}