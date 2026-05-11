use anyhow::Result;
use std::path::Path;
use std::process::Stdio;

/// Expand shell command template.
/// `{}` is replaced with the focused file path.
pub fn expand_command(template: &str, focused: Option<&Path>) -> String {
    let path_str = focused
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    template.replace("{}", &path_str)
}

/// Run a command that captures stdout (non-interactive).
pub async fn run_capture(shell: &str, cmd: &str, cwd: &Path) -> Result<String> {
    let output = tokio::process::Command::new(shell)
        .arg("-c")
        .arg(cmd)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(stdout)
    } else {
        Err(anyhow::anyhow!("{}", stderr))
    }
}

/// Run a command that takes over the terminal (e.g. $EDITOR).
/// Caller must suspend TUI before calling this.
pub fn run_takeover(shell: &str, cmd: &str, cwd: &Path) -> Result<()> {
    let status = std::process::Command::new(shell)
        .arg("-c")
        .arg(cmd)
        .current_dir(cwd)
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Command exited with status: {}", status))
    }
}
