#[tokio::main]
async fn main() {
    loki_logger::init("http://loki:3100/loki/api/v1/push", log::LevelFilter::Info).unwrap();

    log::info!("Logged into Loki !");

    loop {}
}
