use std::{
    collections::BTreeMap,
    hash::{DefaultHasher, Hash, Hasher},
    path::PathBuf,
};

use anyhow::anyhow;
use log::{info, trace};
use serde::Deserialize;
use tokio::{
    fs::{OpenOptions, create_dir_all, remove_dir_all},
    io::AsyncWriteExt,
    process::Command,
    time::Instant,
};

use crate::executor::cmd::ProcessHandle;

#[derive(Debug, Hash, Deserialize)]
pub struct IoParams {
    config: String,
    #[serde(flatten)]
    opts: BTreeMap<String, String>,
}

const CONFIG_FILENAME: &str = "conf.yaml";
const STORAGE_DIR: &str = "storage";

pub async fn run_io(params: IoParams, binary_path: PathBuf) -> anyhow::Result<ProcessHandle> {
    info!("starting io_tester opts={:?}", params.opts);
    let mut hasher = DefaultHasher::new();
    params.hash(&mut hasher);
    Instant::now().hash(&mut hasher);
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
    ];

    let opts = params
        .opts
        .iter()
        .map(|(k, v)| [k, v])
        .flatten()
        .map(String::as_str);
    args.extend(opts);

    info!(
        "launching io_tester binary={} with {:?} args",
        binary_path.display(),
        args
    );

    let deleter = async move || -> anyhow::Result<()> { Ok(remove_dir_all(work_dir).await?) };
    ProcessHandle::start(
        Command::new(binary_path).args(args),
        Some(Box::pin(deleter())),
    )
    .await
}
