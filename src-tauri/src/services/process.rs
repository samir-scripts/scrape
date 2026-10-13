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
