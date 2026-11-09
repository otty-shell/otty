use otty_containers::{PortProtocol, RestartPolicy};

/// One change the user made in the Run form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RunEdit {
    Image(String),
    Name(String),
    PullMissing(bool),
    CommandInput(String),
    CommandSubmit,
    CommandBackspace,
    CommandRemove(usize),
    PortHost(usize, String),
    PortContainer(usize, String),
    PortProtocol(usize, PortProtocol),
    AddPort,
    RemovePort(usize),
    EnvName(usize, String),
    EnvValue(usize, String),
    AddEnv,
    RemoveEnv(usize),
    VolumeSource(usize, String),
    VolumeTarget(usize, String),
    VolumeReadOnly(usize, bool),
    VolumeRelabel(usize, bool),
    AddVolume,
    RemoveVolume(usize),
    Network(String),
    Restart(RestartPolicy),
    AutoRemove(bool),
}
