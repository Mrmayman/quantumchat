use std::sync::{Arc, mpsc::Sender};

use whatsapp_rust::{
    bot::Bot,
    store::{Backend, SqliteStore},
    transport::{TokioWebSocketTransportFactory, UreqHttpClient},
};

use crate::WEvent;

pub async fn init(sender: Sender<WEvent>) {
    let backend = Arc::new(
        SqliteStore::new("listener.db")
            .await
            .expect("Failed to create listener backend"),
    ) as Arc<dyn Backend>;

    let transport_factory = TokioWebSocketTransportFactory::new();
    let http_client = UreqHttpClient::new();

    let mut bot = Bot::builder()
        .with_backend(backend)
        .with_transport_factory(transport_factory)
        .with_http_client(http_client)
        .on_event(move |event, _client| {
            let sender = sender.clone();
            async move {
                _ = sender.send(event);
            }
        })
        .build()
        .await
        .expect("Failed to build listener bot");

    bot.run().await.unwrap();
}
