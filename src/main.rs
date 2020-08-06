#[async_std::main]
async fn main() -> Result<(), std::io::Error> {
    lares::server().await
}
