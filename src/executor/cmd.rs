use std::path::Path;
use anyhow::anyhow;

use serde::Serialize;
use tokio::process::Command;

#[derive(Debug, Serialize)]
pub struct CmdOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub return_code: i32,
}

pub async fn run_command(command: &mut Command) -> anyhow::Result<CmdOutput> {
    let command = command.output().await?;

    Ok(CmdOutput {
        stdout: command.stdout,
        stderr: command.stderr,
        return_code: command.status.code().ok_or(anyhow!("Unknown return code"))?,
    })
}