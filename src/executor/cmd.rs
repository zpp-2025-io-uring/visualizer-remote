use anyhow::anyhow;

use serde::Serialize;
use tokio::process::Command;

#[derive(Debug, Serialize)]
pub struct CmdOutput {
    pub stdout: String,
    pub stderr: String,
    pub return_code: i32,
}

pub async fn run_command(command: &mut Command) -> anyhow::Result<CmdOutput> {
    let command = command.output().await?;

    Ok(CmdOutput {
        stdout: command.stdout.try_into()?,
        stderr: command.stderr.try_into()?,
        return_code: command.status.code().ok_or(anyhow!("Unknown return code"))?,
    })
}