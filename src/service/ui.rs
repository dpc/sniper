use crate::{
    auction::{Amount, ItemBid},
    event, event_log,
    persistence::SharedPersistence,
    service::LoopService,
};
use anyhow::{format_err, Context, Result};
use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use tokio::{runtime::Runtime, sync::oneshot};

pub struct Ui {
    // cancels all tasks on drop
    _runtime: Runtime,
    server_rx: oneshot::Receiver<Result<()>>,
}

#[derive(Deserialize)]
struct BidRequest {
    item: String,
    price: Amount,
}

async fn handle_bid_request(
    persistence: SharedPersistence,
    even_writer: event_log::SharedWriter,
    bid_request: BidRequest,
) -> Result<()> {
    // OK, so here's the deal; mixing sync & async
    // code is a PITA and I don't want to convert
    // the whole project into async, at least ATM.
    // For mixing, one could define new set of traits
    // with async methods, and that works OKish,
    // though I've hit a problem of not being able
    // to share a [tokio::sync::Mutex] between
    // sync & async code in [crate::persistence::InMemoryPersistence].
    //
    // Using `spawn_blocking` is lazy and should work, so I
    // leave it at that.
    tokio::task::spawn_blocking(move || {
        even_writer.write(
            &mut *persistence.get_connection()?,
            &[event::Event::Ui(event::UiEvent::MaxBidSet(ItemBid {
                item: bid_request.item,
                price: bid_request.price,
            }))],
        )
    })
    .await??;
    Ok(())
}

async fn run_http_server(
    persistence: SharedPersistence,
    even_writer: event_log::SharedWriter,
) -> Result<()> {
    // build our application with a single route
    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route(
            "/bid/",
            post({
                let even_writer = even_writer.clone();
                let persistence = persistence.clone();

                |Json(bid_request): Json<BidRequest>| async move {
                    match handle_bid_request(persistence, even_writer, bid_request).await {
                        Ok(()) => (StatusCode::OK, "".into()),
                        Err(e) => handle_anyhow_error(e).await,
                    }
                }
            }),
        );

    // run it with hyper on localhost:3000
    axum::Server::try_bind(&"0.0.0.0:3000".parse()?)?
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

async fn handle_anyhow_error(err: anyhow::Error) -> (StatusCode, String) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        format!("Something went wrong: {}", err),
    )
}

impl Ui {
    pub fn new(
        persistence: SharedPersistence,
        even_writer: event_log::SharedWriter,
    ) -> Result<Self> {
        let runtime = Runtime::new()?;

        let (tx, rx) = oneshot::channel();

        runtime.spawn(async move {
            tx.send(
                run_http_server(persistence, even_writer)
                    .await
                    .with_context(|| "Failed to run http server".to_string()),
            )
            .expect("send to work");
        });

        Ok(Self {
            _runtime: runtime,
            server_rx: rx,
        })
    }
}

impl LoopService for Ui {
    fn run_iteration<'a>(&mut self) -> Result<()> {
        // don't hog the cpu
        std::thread::sleep(std::time::Duration::from_millis(100));

        match self.server_rx.try_recv() {
            Ok(res) => res,
            Err(oneshot::error::TryRecvError::Empty) => Ok(()),
            Err(oneshot::error::TryRecvError::Closed) => {
                Err(format_err!("ui server died without leaving a response?!"))
            }
        }
    }
}
