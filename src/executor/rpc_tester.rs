use std::{
    hash::{DefaultHasher, Hash, Hasher},
    path::PathBuf,
};

use anyhow::anyhow;
use log::{info, trace};
use serde::Deserialize;
use tokio::{
    fs::{OpenOptions, create_dir_all},
    io::AsyncWriteExt,
    process::Command,
};

use crate::executor::cmd::ProcessHandle;

#[derive(Debug, Hash, Deserialize)]
pub struct RpcParams {
    config: String,
    backend: String,
    ip_address: String,
    port: String,
    is_server: bool,
    app_cpuset: String,
    async_worker_cpuset: Option<String>,
}

const CONFIG_FILENAME: &str = "conf.yaml";

pub async fn run_rpc(params: RpcParams, binary_path: PathBuf) -> anyhow::Result<ProcessHandle> {
    info!(
        "starting rpc_tester backend={} mode={} ip={} port={} app_cpuset={}",
        params.backend,
        if params.is_server { "server" } else { "client" },
        params.ip_address,
        params.port,
        params.app_cpuset
    );
    let mut hasher = DefaultHasher::new();
    params.hash(&mut hasher);
    let hash = hasher.finish();
    trace!("rpc_tester run hash={}", hash);

    let work_dir = PathBuf::from(&format!("{}", hash));
    create_dir_all(&work_dir).await?;
    trace!("rpc_tester work dir={}", work_dir.display());

    let config_path = work_dir.join(CONFIG_FILENAME);

    let mut conf = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&config_path)
        .await?;
    conf.write_all(params.config.as_bytes()).await?;

    let mut args: Vec<&str> = vec![
        "--conf",
        config_path.to_str().ok_or(anyhow!("invalid config path"))?,
        "--reactor-backend",
        &params.backend,
        "--cpuset",
        &params.app_cpuset,
        "--port",
        &params.port
    ];

    if params.is_server {
        trace!("rpc_tester listen mode on {}", params.ip_address);
        args.extend_from_slice(&["--listen", &params.ip_address]);
    } else {
        trace!("rpc_tester connect mode to {}", params.ip_address);
        args.extend_from_slice(&["--connect", &params.ip_address]);
    }

    if let Some(ref worker_cpuset) = params.async_worker_cpuset {
        trace!("rpc_tester async worker cpuset={}", worker_cpuset);
        args.extend_from_slice(&["--async-workers-cpuset", worker_cpuset]);
    }

    info!("launching rpc_tester binary={} with {:?} args", binary_path.display(), args);

    ProcessHandle::start(Command::new(binary_path).args(args)).await
}
