use tauri_plugin_shell::process::CommandChild;
/// Ensures repository/protocol errors and dropped execution futures kill the child.
/// The shell plugin's wait thread owns reaping; normal completion disarms this guard.
pub struct ChildGuard(Option<CommandChild>);
impl ChildGuard {
    pub fn new(child: CommandChild) -> Self {
        Self(Some(child))
    }
    pub fn write(&mut self, bytes: &[u8]) -> std::result::Result<(), tauri_plugin_shell::Error> {
        self.0.as_mut().expect("active child").write(bytes)
    }
    pub fn kill(&mut self) -> std::result::Result<(), tauri_plugin_shell::Error> {
        if let Some(child) = self.0.take() {
            child.kill()
        } else {
            Ok(())
        }
    }
    pub fn disarm(&mut self) {
        self.0.take();
    }
}
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.kill();
    }
}

pub fn worker_command(
    app: &tauri::AppHandle,
) -> crate::error::Result<tauri_plugin_shell::process::Command> {
    use tauri_plugin_shell::ShellExt;
    if let Some(directory) = super::native_smoke::directory() {
        let fixture = directory.join("worker-fixture");
        if fixture.is_file() {
            return Ok(app.shell().command(fixture));
        }
    }
    app.shell().sidecar("scrape-worker").map_err(|error| {
        crate::error::AppError::Internal(format!("Failed to find sidecar: {error}"))
    })
}

pub fn execution_seconds(default: u64) -> u64 {
    super::native_smoke::directory()
        .and_then(|directory| std::fs::read_to_string(directory.join("timeout-seconds")).ok())
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| (1..=5).contains(value))
        .unwrap_or(default)
}
