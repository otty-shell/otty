use crate::services::shell::{ShellService, ShellServiceImpl};

/// App-owned registry of long-lived services.
pub(crate) struct ServiceRegistry {
    shell: Box<dyn ShellService>,
}

impl ServiceRegistry {
    pub(crate) fn new() -> Self {
        Self {
            shell: Box::new(ShellServiceImpl::new()),
        }
    }

    pub(crate) fn shell_mut(&mut self) -> &mut dyn ShellService {
        self.shell.as_mut()
    }
}
