#[async_std::main]
async fn main() -> Result<(), anyhow::Error> {
    lares::cli().await
}
