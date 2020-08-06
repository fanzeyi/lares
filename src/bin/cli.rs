#[async_std::main]
async fn main() {
    match lares::cli().await {
        Ok(()) => (),
        Err(e) => eprintln!("Error: {:?}", e),
    }
}
