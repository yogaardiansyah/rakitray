use crate::models::{LogEntry, LogStream};
use std::process::Stdio;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;

pub struct ProcessHandle {
    pub child: Child,
    pub instance_id: String,
}

pub struct Runner;

impl Runner {
    pub fn spawn(
        instance_id: &str,
        mut command: Command,
        log_tx: mpsc::Sender<LogEntry>,
    ) -> Result<ProcessHandle, std::io::Error> {
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        #[cfg(unix)]
        unsafe {
            command.pre_exec(|| {
                libc::setpgid(0, 0);
                #[cfg(target_os = "linux")]
                libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM);
                Ok(())
            });
        }

        let mut child = command.spawn()?;

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let id_out = instance_id.to_string();
        let id_err = instance_id.to_string();
        let tx_out = log_tx.clone();
        let tx_err = log_tx;

        if let Some(stdout) = stdout {
            tokio::spawn(async move {
                let mut reader = BufReader::new(stdout).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    let ts = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;
                    let _ = tx_out
                        .send(LogEntry {
                            instance_id: id_out.clone(),
                            timestamp_ms: ts,
                            stream: LogStream::Stdout,
                            line,
                        })
                        .await;
                }
            });
        }

        if let Some(stderr) = stderr {
            tokio::spawn(async move {
                let mut reader = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    let ts = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;
                    let _ = tx_err
                        .send(LogEntry {
                            instance_id: id_err.clone(),
                            timestamp_ms: ts,
                            stream: LogStream::Stderr,
                            line,
                        })
                        .await;
                }
            });
        }

        Ok(ProcessHandle {
            child,
            instance_id: instance_id.to_string(),
        })
    }

    pub async fn stop(handle: &mut ProcessHandle) -> Result<(), std::io::Error> {
        #[cfg(unix)]
        if let Some(pid) = handle.child.id() {
            unsafe {
                libc::kill(-(pid as i32), libc::SIGTERM);
            }
        }

        let timeout = tokio::time::sleep(std::time::Duration::from_millis(5000));
        tokio::pin!(timeout);

        tokio::select! {
            _ = handle.child.wait() => Ok(()),
            _ = &mut timeout => {
                #[cfg(unix)]
                if let Some(pid) = handle.child.id() {
                    unsafe {
                        libc::kill(-(pid as i32), libc::SIGKILL);
                    }
                }
                let _ = handle.child.kill().await;
                Ok(())
            }
        }
    }
}
