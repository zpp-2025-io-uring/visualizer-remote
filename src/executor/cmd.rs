use std::process::{ExitStatus, Stdio};

use log::{debug, error, info};
use serde::Serialize;
use tokio::process::{Child, Command};

#[derive(Debug)]
pub struct ProcessHandle(Child);

#[derive(Debug, Serialize)]
pub struct CmdOutput {
    pub stdout: String,
    pub stderr: String,
    pub return_code: Option<i32>,
}

impl ProcessHandle {
    pub async fn start(command: &mut Command) -> anyhow::Result<ProcessHandle> {
        debug!("Running command: {:?}", command);
        match command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => {
                info!("spawned child process pid={:?}", child.id());
                Ok(ProcessHandle(child))
            }
            Err(e) => {
                error!("failed to spawn child process: {}", e);
                Err(e.into())
            }
        }
    }

    pub async fn wait(self) -> anyhow::Result<CmdOutput> {
        let child = self.0;
        let pid = child.id();
        info!("waiting for child process pid={:?}", pid);
        let output = child.wait_with_output().await?;

        info!(
            "child process finished pid={:?} return_code={:?} stdout_bytes={} stderr_bytes={}",
            pid,
            output.status.code(),
            output.stdout.len(),
            output.stderr.len()
        );

        Ok(CmdOutput {
            stdout: output.stdout.try_into()?,
            stderr: output.stderr.try_into()?,
            return_code: output.status.code(),
        })
    }

    pub async fn kill(&mut self) -> anyhow::Result<()> {
        info!("killing child process pid={:?}", self.0.id());
        Ok(self.0.kill().await?)
    }

    pub async fn terminate(&mut self) -> anyhow::Result<()> {
        info!("terminating child process pid={:?}", self.0.id());
        self.kill().await // SIGTERM not available, fallback to SIGKILL
    }

    pub async fn poll(&mut self) -> anyhow::Result<ExitStatus> {
        info!("polling child process pid={:?}", self.0.id());
        Ok(self.0.wait().await?)
    }
}
