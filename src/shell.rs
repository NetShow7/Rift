use anyhow::Result;
use std::path::Path;
use std::process::Stdio;

fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

/// Expand shell command template.
/// `{}` is replaced with the focused file path, safely single-quoted.
pub fn expand_command(template: &str, focused: Option<&Path>) -> String {
    let path_str = focused
        .map(|p| shell_quote(&p.to_string_lossy()))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn expand_command_with_focused_path() {
        let result = expand_command("$EDITOR {}", Some(Path::new("/path/to/file.txt")));
        assert_eq!(result, "$EDITOR '/path/to/file.txt'");
    }

    #[test]
    fn expand_command_without_focused() {
        let result = expand_command("$SHELL", None);
        assert_eq!(result, "$SHELL");
    }

    #[test]
    fn expand_command_multiple_placeholders() {
        let result = expand_command("cp {} {}", Some(Path::new("src.txt")));
        assert_eq!(result, "cp 'src.txt' 'src.txt'");
    }

    #[test]
    fn expand_command_no_placeholder() {
        let result = expand_command("ls -la", Some(Path::new("/ignored")));
        assert_eq!(result, "ls -la");
    }

    #[test]
    fn expand_command_path_with_spaces() {
        let result = expand_command("cat {}", Some(Path::new("/path/with spaces/file.txt")));
        assert_eq!(result, "cat '/path/with spaces/file.txt'");
    }

    #[test]
    fn expand_command_path_with_shell_metacharacters() {
        let result = expand_command("cat {}", Some(Path::new("/tmp/; rm -rf ~")));
        assert_eq!(result, "cat '/tmp/; rm -rf ~'");
    }

    #[test]
    fn expand_command_empty_template() {
        let result = expand_command("", Some(Path::new("/path")));
        assert_eq!(result, "");
    }
}
