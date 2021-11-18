use httpmock::prelude::*;

#[tokio::test]
async fn async_logging_from_sync_thread() {
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(POST).path("/loki/api/v1/push");
        then.status(200);
    });

    loki_logger::init(server.url("/loki/api/v1/push"), log::LevelFilter::Info).unwrap();

    std::thread::spawn(|| {
        log::info!("Logged into Loki !");
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    mock.assert();
}
