use std::{pin::Pin, process::Output};

use anyhow::anyhow;

use serde::Serialize;
use tokio::{io, process::{Child, Command}};

pub struct ProcessHandle(Child);

#[derive(Debug, Serialize)]
pub struct CmdOutput {
    pub stdout: String,
    pub stderr: String,
    pub return_code: i32,
}

impl ProcessHandle {
    pub async fn start(command: &mut Command) -> anyhow::Result<ProcessHandle> {
        Ok(ProcessHandle(command.spawn()?))
    }

    pub async fn wait(self) -> anyhow::Result<CmdOutput> {
        let output = self.0.wait_with_output().await?;

        Ok(CmdOutput {
            stdout: output.stdout.try_into()?,
            stderr: output.stderr.try_into()?,
            return_code: output.status.code().ok_or(anyhow!("Unknown return code"))?,
        })
    }

    pub async fn kill(mut self) -> anyhow::Result<()> {
        Ok(self.0.kill().await?)
    }

    pub async fn terminate(self) -> anyhow::Result<()> {
        self.kill().await // SIGTERM not available, fallback to SIGKILL
    }
}