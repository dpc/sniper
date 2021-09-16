use crate::{
    auction::ItemBid, event, event_log, persistence::SharedPersistence, service::LoopService,
};
use anyhow::{format_err, Context, Result};
use axum::{
    handler::{get, post},
    Router,
};
use tokio::{runtime::Runtime, sync::oneshot};

pub struct Ui {
    // cancels all tasks on read
    _runtime: Runtime,
    server_rx: oneshot::Receiver<Result<()>>,
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
                || async move {
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
                            &mut *persistence.get_connection().unwrap(), // TODO
                            &[event::Event::Ui(event::UiEvent::MaxBidSet(ItemBid {
                                item: "tbd".to_string(),
                                price: 1,
                            }))],
                        );
                    })
                    .await
                    .unwrap() // TODO;
                }
            }),
        );

    // run it with hyper on localhost:3000
    axum::Server::try_bind(&"0.0.0.0:3000".parse()?)?
        .serve(app.into_make_service())
        .await?;

    Ok(())
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
                    .with_context(|| format!("Failed to run http server")),
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
                Err(format_err!("ui server died with leaving a response?!"))
            }
        }
    }
}
