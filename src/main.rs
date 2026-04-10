mod executor;

use axum::{
    Json, Router, extract::State, http::StatusCode, routing::post
};
use tokio::{net::TcpListener, sync::Mutex};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use crate::executor::{cmd::{CmdOutput, ProcessHandle}, io_tester::{IoParams, run_io}};

type Pid = u64;

struct App {
    processes: Mutex<HashMap<Pid, ProcessHandle>>,
    index: Mutex<Pid>
}

impl App {
    pub fn new() -> App {
        App { processes: Mutex::new(HashMap::new()), index: Mutex::new(0) }
    }

    async fn get_new_pid(&self) -> Pid {
        let mut index = self.index.lock().await;
        *index += 1;
        *index
    }

    pub async fn add_process(&self, handle: ProcessHandle) -> anyhow::Result<Pid> {
        let pid = self.get_new_pid().await;
        self.processes.lock().await.insert(pid, handle);
        Ok(pid)
    }
}

// Entry point
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app = Router::new()
        .route("/io_tester", post(io_tester_endpoint))
        .route("/wait_and_output", post(io_tester_endpoint))
        .route("/kill", post(io_tester_endpoint))
        .route("/terminate", post(io_tester_endpoint))
        .with_state(Arc::new(App::new()));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;

    axum::serve(listener, app).await?;

    Ok(())
}

async fn handle_io_tester_endpoint(app: &App, params: IoParams) -> anyhow::Result<Pid> {
    let handle = run_io(params).await?;
    let pid = app.add_process(handle).await?;
    Ok(pid)
}

async fn io_tester_endpoint(State(state): State<Arc<App>> ,Json(params): Json<IoParams>) -> axum::response::Result<Json<Pid>> {
    match handle_io_tester_endpoint(&state, params).await {
        Ok(output) => Ok(Json(output)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into()),
    }
}

