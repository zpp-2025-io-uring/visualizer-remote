mod executor;

use axum::{
    Json, Router, routing::{post}
};
use tokio::net::TcpListener;
use std::{net::SocketAddr};

use crate::executor::{cmd::CmdOutput, io_tester::{IoParams, run_io}};


// Entry point
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app = Router::new()
        .route("/io_tester", post(io_tester_endpoint));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;

    axum::serve(listener, app).await?;

    Ok(())
}


async fn io_tester_endpoint(Json(params): Json<IoParams>) -> axum::response::Result<Json<CmdOutput>> {
    let result = run_io(params).await;

    match result {
        Ok(output) => Ok(Json(output)),
        Err(e) => Err(e.to_string().into()),
    }
}
