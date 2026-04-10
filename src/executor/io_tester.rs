
use std::{hash::{DefaultHasher, Hash, Hasher}, path::{Path, PathBuf}};

use anyhow::{anyhow, bail};
use serde::Deserialize;
use tokio::{fs::{OpenOptions, create_dir_all, remove_dir_all}, io::AsyncWriteExt, process::Command};

use crate::executor::cmd::{CmdOutput, ProcessHandle};

#[derive(Debug, Hash, Deserialize)]
pub struct IoParams {
    config: String,
    backend: String,
    app_cpuset: String,
    async_worker_cpuset: Option<String>,
}

const CONFIG_FILENAME: &str = "conf.yaml";
const STORAGE_DIR: &str = "storage";

pub async fn run_io(params: IoParams) -> anyhow::Result<ProcessHandle> {
    println!("Running with {:?}", params);
    let mut hasher = DefaultHasher::new();
    params.hash(&mut hasher);
    let hash = hasher.finish();

    let work_dir = PathBuf::from(&format!("{}", hash));
    create_dir_all(&work_dir).await?;

    let config_path = work_dir.join(CONFIG_FILENAME);
    let storage_dir = work_dir.join(STORAGE_DIR);

    create_dir_all(&storage_dir).await?;

    let mut conf = OpenOptions::new().create(true).write(true).open(&config_path).await?;
    conf.write_all(params.config.as_bytes()).await?;

    let mut args: Vec<&str> = vec!["--conf", config_path.to_str().ok_or(anyhow!("invalid config path"))?, "--storage", storage_dir.to_str().ok_or(anyhow!("invalid storage dir"))?, "--reactor-backend", &params.backend, "--cpuset", &params.app_cpuset];

    if let Some(ref worker_cpuset) = params.async_worker_cpuset {
        args.extend_from_slice(&["--async-workers-cpuset", &worker_cpuset]);
    }


    let result = ProcessHandle::start(Command::new("").args(args)).await?;

    remove_dir_all(work_dir).await?;

    Ok(result)
}

