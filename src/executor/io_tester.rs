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
pub struct IoParams {
    config: String,
    backend: String,
    app_cpuset: String,
    async_worker_cpuset: Option<String>,
}

const CONFIG_FILENAME: &str = "conf.yaml";
const STORAGE_DIR: &str = "storage";

pub async fn run_io(params: IoParams, binary_path: PathBuf) -> anyhow::Result<ProcessHandle> {
    info!("starting io_tester backend={} app_cpuset={}", params.backend, params.app_cpuset);
    let mut hasher = DefaultHasher::new();
    params.hash(&mut hasher);
    let hash = hasher.finish();
    trace!("io_tester run hash={}", hash);

    let work_dir = PathBuf::from(&format!("{}", hash));
    create_dir_all(&work_dir).await?;
    trace!("io_tester work dir={}", work_dir.display());

    let config_path = work_dir.join(CONFIG_FILENAME);
    let storage_dir = work_dir.join(STORAGE_DIR);

    create_dir_all(&storage_dir).await?;
    trace!("io_tester storage dir={}", storage_dir.display());

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
        "--storage",
        storage_dir.to_str().ok_or(anyhow!("invalid storage dir"))?,
        "--reactor-backend",
        &params.backend,
        "--cpuset",
        &params.app_cpuset,
    ];

    if let Some(ref worker_cpuset) = params.async_worker_cpuset {
        trace!("io_tester async worker cpuset={}", worker_cpuset);
        args.extend_from_slice(&["--async-workers-cpuset", worker_cpuset]);
    }

    info!("launching io_tester binary={} with {:?} args", binary_path.display(), args);

    ProcessHandle::start(Command::new(binary_path).args(args)).await
}
