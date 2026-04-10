mod executor;

use anyhow::anyhow;
use axum::{Json, Router, extract::State, http::StatusCode, routing::post};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::{net::TcpListener, sync::Mutex};

use crate::executor::{
    cmd::{CmdOutput, ProcessHandle},
    io_tester::{IoParams, run_io},
    rpc_tester::{RpcParams, run_rpc},
};

type Pid = u64;

struct App {
    processes: Mutex<HashMap<Pid, ProcessHandle>>,
    index: Mutex<Pid>,
}

impl App {
    pub fn new() -> App {
        App {
            processes: Mutex::new(HashMap::new()),
            index: Mutex::new(0),
        }
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

    pub async fn remove_process(&self, pid: &Pid) -> anyhow::Result<ProcessHandle> {
        self.processes
            .lock()
            .await
            .remove(pid)
            .ok_or(anyhow!("non existant pid"))
    }
}

// Entry point
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app = Router::new()
        .route("/io_tester", post(io_tester_endpoint))
        .route("/rpc_tester", post(rpc_tester_endpoint))
        .route("/wait_and_output", post(wait_and_output_endpoint))
        .route("/kill", post(kill_endpoint))
        .route("/terminate", post(terminate_endpoint))
        .route("/poll", post(poll_endpoint))
        .with_state(Arc::new(App::new()));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
    let listener = TcpListener::bind(addr).await?;

    axum::serve(listener, app).await?;

    Ok(())
}

async fn handle_io_tester_endpoint(app: &App, params: IoParams) -> anyhow::Result<Pid> {
    app.add_process(run_io(params).await?).await
}

async fn io_tester_endpoint(
    State(state): State<Arc<App>>,
    Json(params): Json<IoParams>,
) -> axum::response::Result<Json<Pid>> {
    match handle_io_tester_endpoint(&state, params).await {
        Ok(output) => Ok(Json(output)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into()),
    }
}

async fn handle_rpc_tester_endpoint(app: &App, params: RpcParams) -> anyhow::Result<Pid> {
    app.add_process(run_rpc(params).await?).await
}

async fn rpc_tester_endpoint(
    State(state): State<Arc<App>>,
    Json(params): Json<RpcParams>,
) -> axum::response::Result<Json<Pid>> {
    match handle_rpc_tester_endpoint(&state, params).await {
        Ok(output) => Ok(Json(output)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into()),
    }
}

async fn handle_wait_and_output_endpoint(app: &App, pid: Pid) -> anyhow::Result<CmdOutput> {
    app.remove_process(&pid).await?.wait().await
}

async fn wait_and_output_endpoint(
    State(state): State<Arc<App>>,
    Json(pid): Json<Pid>,
) -> axum::response::Result<Json<CmdOutput>> {
    match handle_wait_and_output_endpoint(&state, pid).await {
        Ok(output) => Ok(Json(output)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into()),
    }
}

async fn handle_kill_endpoint(app: &App, pid: Pid) -> anyhow::Result<()> {
    let mut processes = app.processes.lock().await;
    let handle = processes.get_mut(&pid).ok_or(anyhow!("non existant pid"))?;
    handle.kill().await
}

async fn kill_endpoint(
    State(state): State<Arc<App>>,
    Json(pid): Json<Pid>,
) -> axum::response::Result<Json<()>> {
    match handle_kill_endpoint(&state, pid).await {
        Ok(output) => Ok(Json(output)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into()),
    }
}

async fn handle_terminate_endpoint(app: &App, pid: Pid) -> anyhow::Result<()> {
    let mut processes = app.processes.lock().await;
    let handle = processes.get_mut(&pid).ok_or(anyhow!("non existant pid"))?;
    handle.terminate().await
}

async fn terminate_endpoint(
    State(state): State<Arc<App>>,
    Json(pid): Json<Pid>,
) -> axum::response::Result<Json<()>> {
    match handle_terminate_endpoint(&state, pid).await {
        Ok(output) => Ok(Json(output)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into()),
    }
}

async fn handle_poll_endpoint(app: &App, pid: Pid) -> anyhow::Result<Option<i32>> {
    let mut processes = app.processes.lock().await;
    let handle = processes.get_mut(&pid).ok_or(anyhow!("non existant pid"))?;
    Ok(handle.poll().await?.code())
}

async fn poll_endpoint(
    State(state): State<Arc<App>>,
    Json(pid): Json<Pid>,
) -> axum::response::Result<Json<Option<i32>>> {
    match handle_poll_endpoint(&state, pid).await {
        Ok(output) => Ok(Json(output)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into()),
    }
}
