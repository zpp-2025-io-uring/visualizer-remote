mod executor;

use anyhow::anyhow;
use axum::{Json, Router, extract::State, http::StatusCode, routing::post};
use log::{error, info, debug};
use std::{collections::HashMap, net::{IpAddr, SocketAddr}, path::PathBuf, sync::Arc};
use tokio::{net::TcpListener, sync::Mutex};
use clap::Parser;

use crate::executor::{
    cmd::{CmdOutput, ProcessHandle},
    io_tester::{IoParams, run_io},
    rpc_tester::{RpcParams, run_rpc},
};

type Pid = u64;

struct App {
    processes: Mutex<HashMap<Pid, ProcessHandle>>,
    index: Mutex<Pid>,
    rpc_tester_path: PathBuf,
    io_tester_path: PathBuf,
}

impl App {
    pub fn new(rpc_tester_path: PathBuf, io_tester_path: PathBuf) -> App {
        App {
            processes: Mutex::new(HashMap::new()),
            index: Mutex::new(0),
            rpc_tester_path,
            io_tester_path
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
        info!("registered process pid={}", pid);
        Ok(pid)
    }

    pub async fn remove_process(&self, pid: &Pid) -> anyhow::Result<ProcessHandle> {
        info!("removing process pid={}", pid);
        self.processes
            .lock()
            .await
            .remove(pid)
            .ok_or(anyhow!("non existant pid"))
    }

    pub fn rpc_tester_path(&self) -> &PathBuf {
        &self.rpc_tester_path
    }

    pub fn io_tester_path(&self) -> &PathBuf {
        &self.io_tester_path
    }
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Host to listen on.
    #[arg(long, default_value_t = IpAddr::from([127, 0, 0, 1]))]
    host: IpAddr,

    /// Port to listen on.
    #[arg(long, default_value_t = 3000)]
    port: u16,

    /// Path to the rpc_tester binary
    #[arg(short, long)]
    rpc_tester_path: PathBuf,

    /// Path to the io_tester binary
    #[arg(short, long)]
    io_tester_path: PathBuf,
}

// Entry point
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();
    debug!("starting visualizer-remote with {:?}", args);

    let app = Router::new()
        .route("/io_tester", post(io_tester_endpoint))
        .route("/rpc_tester", post(rpc_tester_endpoint))
        .route("/wait_and_output", post(wait_and_output_endpoint))
        .route("/kill", post(kill_endpoint))
        .route("/terminate", post(terminate_endpoint))
        .route("/poll", post(poll_endpoint))
        .with_state(Arc::new(App::new(args.rpc_tester_path, args.io_tester_path)));

    let addr = SocketAddr::new(args.host, args.port);
    let listener = TcpListener::bind(addr).await?;
    info!("listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}

async fn handle_io_tester_endpoint(app: &App, params: IoParams) -> anyhow::Result<Pid> {
    app.add_process(run_io(params, app.io_tester_path().clone()).await?).await
}

async fn io_tester_endpoint(
    State(state): State<Arc<App>>,
    Json(params): Json<IoParams>,
) -> axum::response::Result<Json<Pid>> {
    info!("POST /io_tester");
    match handle_io_tester_endpoint(&state, params).await {
        Ok(output) => {
            info!("/io_tester started pid={}", output);
            Ok(Json(output))
        }
        Err(e) => {
            error!("/io_tester failed: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into())
        }
    }
}

async fn handle_rpc_tester_endpoint(app: &App, params: RpcParams) -> anyhow::Result<Pid> {
    app.add_process(run_rpc(params, app.rpc_tester_path().clone()).await?).await
}

async fn rpc_tester_endpoint(
    State(state): State<Arc<App>>,
    Json(params): Json<RpcParams>,
) -> axum::response::Result<Json<Pid>> {
    info!("POST /rpc_tester");
    match handle_rpc_tester_endpoint(&state, params).await {
        Ok(output) => {
            info!("/rpc_tester started pid={}", output);
            Ok(Json(output))
        }
        Err(e) => {
            error!("/rpc_tester failed: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into())
        }
    }
}

async fn handle_wait_and_output_endpoint(app: &App, pid: Pid) -> anyhow::Result<CmdOutput> {
    app.remove_process(&pid).await?.wait().await
}

async fn wait_and_output_endpoint(
    State(state): State<Arc<App>>,
    Json(pid): Json<Pid>,
) -> axum::response::Result<Json<CmdOutput>> {
    info!("POST /wait_and_output pid={}", pid);
    match handle_wait_and_output_endpoint(&state, pid).await {
        Ok(output) => {
            info!("/wait_and_output completed");
            Ok(Json(output))
        }
        Err(e) => {
            error!("/wait_and_output failed: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into())
        }
    }
}

async fn handle_kill_endpoint(app: &App, pid: Pid) -> anyhow::Result<()> {
    info!("killing process pid={}", pid);
    let mut processes = app.processes.lock().await;
    let handle = processes.get_mut(&pid).ok_or(anyhow!("non existant pid"))?;
    handle.kill().await
}

async fn kill_endpoint(
    State(state): State<Arc<App>>,
    Json(pid): Json<Pid>,
) -> axum::response::Result<Json<()>> {
    info!("POST /kill pid={}", pid);
    match handle_kill_endpoint(&state, pid).await {
        Ok(output) => {
            info!("/kill succeeded pid={}", pid);
            Ok(Json(output))
        }
        Err(e) => {
            error!("/kill failed pid={}: {}", pid, e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into())
        }
    }
}

async fn handle_terminate_endpoint(app: &App, pid: Pid) -> anyhow::Result<()> {
    info!("terminating process pid={}", pid);
    let mut processes = app.processes.lock().await;
    let handle = processes.get_mut(&pid).ok_or(anyhow!("non existant pid"))?;
    handle.terminate().await
}

async fn terminate_endpoint(
    State(state): State<Arc<App>>,
    Json(pid): Json<Pid>,
) -> axum::response::Result<Json<()>> {
    info!("POST /terminate pid={}", pid);
    match handle_terminate_endpoint(&state, pid).await {
        Ok(output) => {
            info!("/terminate succeeded pid={}", pid);
            Ok(Json(output))
        }
        Err(e) => {
            error!("/terminate failed pid={}: {}", pid, e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into())
        }
    }
}

async fn handle_poll_endpoint(app: &App, pid: Pid) -> anyhow::Result<Option<i32>> {
    info!("polling process pid={}", pid);
    let mut processes = app.processes.lock().await;
    let handle = processes.get_mut(&pid).ok_or(anyhow!("non existant pid"))?;
    Ok(handle.poll().await?.code())
}

async fn poll_endpoint(
    State(state): State<Arc<App>>,
    Json(pid): Json<Pid>,
) -> axum::response::Result<Json<Option<i32>>> {
    info!("POST /poll pid={}", pid);
    match handle_poll_endpoint(&state, pid).await {
        Ok(output) => {
            info!("/poll result pid={} status={:?}", pid, output);
            Ok(Json(output))
        }
        Err(e) => {
            error!("/poll failed pid={}: {}", pid, e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into())
        }
    }
}
