use serde::Deserialize;

/// Identifier for a user-facing string in the translation catalogs.
///
/// Variants map to `snake_case` keys in the locale JSON files under
/// `locales/`. Every variant must be present in every catalog; the
/// `catalogs_cover_every_key` test enforces that, and an unknown key in a JSON
/// file fails deserialization at startup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Key {
    // Header menus (New session, Launches).
    /// "New terminal" entry in the New session menu.
    MenuNewTerminal,
    /// "Split current pane right" entry in the New session menu.
    MenuSplitCurrentPane,
    /// "New launch..." entry in the New session menu.
    MenuNewLaunch,
    /// Tooltip of the New session menu chevron.
    NewSessionMenu,
    /// "New folder" entry in the Launches menu.
    MenuNewFolder,
    /// Tooltip of the Launches menu chevron.
    LaunchesMenu,

    // Placeholders shown when tab state is missing.
    /// Shown when a terminal tab has no backing state.
    TerminalTabNotInitialized,
    /// Shown when a quick launch wizard tab has no editor state.
    QuickLaunchEditorNotInitialized,
    /// Shown when a quick launch error tab has no payload.
    QuickLaunchErrorPayloadMissing,
    /// Shown when the terminal widget cannot be rendered.
    TerminalUnavailable,
    /// Banner on a kept-open pane whose process ended without an exit code.
    ProcessTerminated,

    // Tab titles.
    /// Title of the settings tab.
    SettingsTabTitle,
    /// Title of the quick launch creation wizard tab.
    QuickLaunchCreateTabTitle,

    // Terminal pane context menu.
    /// Copy the current text selection.
    CopySelection,
    /// Paste clipboard contents into the terminal.
    Paste,
    /// Copy the selected block's output.
    CopyContent,
    /// Copy the selected block's prompt.
    CopyPrompt,
    /// Copy the selected block's command line.
    CopyCommand,
    /// Split the pane into a top and a bottom pane.
    SplitDown,
    /// Split the pane into a left and a right pane.
    SplitRight,
    /// Close the pane.
    ClosePane,
    /// Name of this computer when its host name cannot be read.
    ThisComputer,
    /// Tooltip of the title bar button that maximizes or restores the window.
    WindowToggleMaximize,
    /// Tooltip of the title bar minimize button.
    WindowMinimize,
    /// Tooltip of the title bar button that shows or hides the files panel.
    WindowFilesPanel,
    /// Tooltip of the title bar button that shows or hides the left panel.
    WindowLeftPanel,
    /// Tooltip of the title bar focus mode button.
    WindowFocusMode,
    /// Focus mode note: says the panels are hidden.
    FocusNoteHidden,
    /// Focus mode note: text after the shortcut.
    FocusNoteShow,
    /// Tooltip of the title bar button that closes the window.
    WindowClose,

    // Quick launch context menu.
    /// Terminate a running quick launch.
    Kill,
    /// Edit the selected quick launch.
    Edit,
    /// Rename the selected entry.
    Rename,
    /// Duplicate the selected quick launch.
    Duplicate,
    /// Remove the selected quick launch.
    Remove,
    /// Create a folder in the quick launch tree.
    CreateFolder,
    /// Create a quick launch entry.
    CreateQuickLaunch,
    /// Delete the selected folder.
    Delete,
    /// Create a launch entry inside a folder.
    CreateLaunch,

    // Sessions section of the left panel.
    /// Title of the Sessions section of the left panel.
    SessionsPanelTitle,
    /// Tooltip of the Sessions header button that opens a terminal.
    NewSession,
    /// Muted note after this computer's group name.
    ThisComputerNote,
    /// Muted note after an SSH host's group name.
    SshMachineNote,
    /// State word on the Settings row.
    SessionStateSettings,
    /// State word on a launch editor row.
    SessionStateEditor,
    /// State word on a launch error row.
    SessionStateFailed,
    /// Hint in the Sessions section when no session is open.
    NoOpenSessions,

    // Empty state shown in the main area when no session is open.
    /// Title of the empty card.
    EmptyTitle,
    /// Description under the title of the empty card.
    EmptyDescription,
    /// Label of the empty card tile that opens a terminal.
    EmptyNewTerminal,
    /// Label of the empty card tile that creates a launch.
    EmptyNewLaunch,

    // Quick launch sidebar panel.
    /// Title of the Launches section of the left panel.
    LaunchesPanelTitle,
    /// Muted word on a launch row while the launch is running.
    LaunchRunning,

    // Explorer sidebar.
    /// Shown when the explorer has no active folder.
    NoActiveFolder,
    /// Title of the right panel files section.
    FilesTitle,

    // Status bar.
    /// Status bar text for exactly one pane.
    StatusPaneOne,
    /// Status bar pane count. Placeholder: `{count}`.
    TplStatusPanes,
    /// Status bar text for exactly one session.
    StatusSessionOne,
    /// Status bar session count. Placeholder: `{count}`.
    TplStatusSessions,
    // Modes: the switcher and the All modes menu.
    /// Name of the Terminal mode.
    ModeTerminal,
    /// Name of the Docker mode.
    ModeDocker,
    /// Tooltip of the "⋯" button that opens the All modes menu.
    ModesAllModes,
    /// Label of the menu section with Terminal and the pinned modes.
    ModesPinnedSection,
    /// Note after the Pinned label, e.g. "1 of 4". Placeholders: `{count}`,
    /// `{limit}`.
    TplModesPinnedNote,
    /// Label of the menu section with the modes that are not pinned.
    ModesOtherSection,
    /// Tooltip of the pin toggle of a mode that is not pinned.
    ModesPin,
    /// Tooltip of the pin toggle of a pinned mode.
    ModesUnpin,
    /// Hint on the disabled pin toggle while every pin slot is taken.
    ModesUnpinAnotherFirst,
    /// Hint in the empty Other modes section of the All modes menu.
    ModesNoOtherModes,
    // Docker mode: engine tree, tables and inspector.
    /// Containers list in the engine tree and the main area title; also the
    /// Networks column with the container count.
    DockerContainers,
    /// Images list in the engine tree and the main area title.
    DockerImages,
    /// Volumes list in the engine tree and the main area title.
    DockerVolumes,
    /// Networks list in the engine tree and the main area title.
    DockerNetworks,
    /// Create button above the containers table.
    DockerRunContainer,
    /// Create button above the images table.
    DockerPullImage,
    /// Create button above the volumes table.
    DockerCreateVolume,
    /// Create button above the networks table.
    DockerCreateNetwork,
    /// Tooltip of the button that loads every list again.
    DockerRefresh,
    /// Tooltip of the "⋯" button that holds the header actions that do not
    /// fit, and of the "⋯" button at the end of a table row.
    DockerMoreActions,
    /// Placeholder of the name filter above the table.
    DockerFilterPlaceholder,
    /// Running containers next to Containers in the tree. Placeholder:
    /// `{count}`.
    TplDockerRunning,
    /// Chip above the containers table that names the Compose project the
    /// list is limited to. Placeholder: `{name}`.
    TplDockerComposeChip,
    /// Tooltip of the chip's cross that shows all containers again.
    DockerShowAllContainers,
    /// Word next to a tree row whose list failed to load.
    DockerListFailedShort,
    /// Table message while a list loads.
    DockerLoading,
    /// Message of an empty containers table.
    DockerNoContainers,
    /// Message of an empty images table.
    DockerNoImages,
    /// Message of an empty volumes table.
    DockerNoVolumes,
    /// Message of an empty networks table.
    DockerNoNetworks,
    /// Table message when no row matches the filter. Placeholder: `{filter}`.
    TplDockerNothingMatches,
    /// Notice above a table whose list failed to load. Placeholder: `{error}`.
    TplDockerListFailed,
    /// Main area text while OTTY looks for engines.
    DockerSearching,
    /// Title of the card when neither Docker nor Podman is installed.
    DockerNotInstalledTitle,
    /// Text of the card when neither Docker nor Podman is installed.
    DockerNotInstalledText,
    /// Button that opens the Docker install page.
    DockerInstallDocker,
    /// Button that opens the Podman install page.
    DockerInstallPodman,
    /// Button that searches for engines again.
    DockerCheckAgain,
    /// Button that copies a command to the clipboard.
    DockerCopy,
    /// Button that opens the documentation of the fix.
    DockerHowToFix,
    /// Label above a command to run in a terminal.
    DockerRunInTerminal,
    /// Title of the card when Docker is installed but not running.
    DockerStoppedTitle,
    /// Text of the card when Docker is installed but not running.
    DockerStoppedText,
    /// Hint under the command that starts Docker, for macOS.
    DockerStoppedHint,
    /// Title of the card when the Podman API service is off.
    DockerPodmanOffTitle,
    /// Text of the card when the Podman API service is off.
    DockerPodmanOffText,
    /// Hint under the command that turns on Podman, for macOS.
    DockerPodmanOffHint,
    /// Title of the card when OTTY may not use the Docker socket.
    DockerNoAccessTitle,
    /// Text of the no access card. Placeholder: `{socket}`.
    TplDockerNoAccessText,
    /// Label above the command that adds the user to the docker group.
    DockerNoAccessLabel,
    /// Hint under the command: log out again; the group has root rights.
    DockerNoAccessHint,
    /// Left panel text when no engine is installed.
    DockerPanelNotInstalled,
    /// Left panel text when Docker is not running.
    DockerPanelStopped,
    /// Left panel text when the Podman API service is off.
    DockerPanelPodmanOff,
    /// Left panel text when OTTY may not use the Docker socket.
    DockerPanelNoAccess,
    /// Right panel text when no row is selected.
    DockerNothingToInspect,
    /// Line under the no engine cards. Placeholder: `{places}`.
    TplDockerLookedFor,
    /// Table column: the name of a container, volume or network.
    DockerColumnName,
    /// Table column: the state of a container.
    DockerColumnStatus,
    /// Table column: the image of a container.
    DockerColumnImage,
    /// Table column: the ports of a container.
    DockerColumnPorts,
    /// Table column: how long ago the item was created.
    DockerColumnAge,
    /// Table column: the repository and tag of an image.
    DockerColumnRepositoryTag,
    /// Table column: the short ID of an image.
    DockerColumnId,
    /// Table column: the size of an image.
    DockerColumnSize,
    /// Table column: the containers that use an image or a volume.
    DockerColumnUsedBy,
    /// Table column: the driver of a volume or a network.
    DockerColumnDriver,
    /// Table column: the scope of a network.
    DockerColumnScope,
    /// Table column: the subnet of a network.
    DockerColumnSubnet,
    /// Container state badge: running.
    DockerStateRunning,
    /// Container state badge: paused.
    DockerStatePaused,
    /// Container state badge: restarting.
    DockerStateRestarting,
    /// Container state badge: created, never started.
    DockerStateCreated,
    /// Container state badge: exited, code unknown.
    DockerStateExited,
    /// Container state badge: exited with a code. Placeholder: `{code}`.
    TplDockerStateExitedCode,
    /// Container state badge: dead.
    DockerStateDead,
    /// Container state badge: being removed.
    DockerStateRemoving,
    /// Container state badge: stopping.
    DockerStateStopping,
    /// Container state badge: a state OTTY does not know.
    DockerStateUnknown,
    /// Inspector status of an image without a tag that no container uses.
    DockerImageDangling,
    /// Inspector status of an image one container uses.
    DockerUsedByOneContainer,
    /// Inspector status of an image several containers use. Placeholder:
    /// `{count}`.
    TplDockerUsedByContainers,
    /// Inspector status of an image or volume no container uses.
    DockerNotUsed,
    /// Inspector status of a volume: the containers that use it. Placeholder:
    /// `{names}`.
    TplDockerUsedBy,
    /// Inspector status of a network the engine creates itself.
    DockerBuiltInNetwork,
    /// Inspector status of a network with one container.
    DockerOneContainerConnected,
    /// Inspector status of a network with several containers. Placeholder:
    /// `{count}`.
    TplDockerContainersConnected,
    /// Age of an item created less than a minute ago.
    DockerAgeUnderMinute,
    /// Age in minutes. Placeholder: `{count}`.
    TplDockerAgeMinutes,
    /// Age in hours. Placeholder: `{count}`.
    TplDockerAgeHours,
    /// Age in days. Placeholder: `{count}`.
    TplDockerAgeDays,
    /// Age in weeks. Placeholder: `{count}`.
    TplDockerAgeWeeks,
    /// Age in months. Placeholder: `{count}`.
    TplDockerAgeMonths,
    /// Age in years. Placeholder: `{count}`.
    TplDockerAgeYears,
    /// Name of Docker mode in the status bar while no engine is shown.
    DockerStatusMode,
    /// Status bar text while OTTY looks for engines.
    DockerStatusSearching,
    /// Status bar text when no engine was found.
    DockerStatusNoEngine,
    /// Status bar text when Docker is not running.
    DockerStatusStopped,
    /// Status bar text when the Podman socket is off.
    DockerStatusPodmanOff,
    /// Status bar text when the socket is denied. Placeholder: `{socket}`.
    TplDockerStatusNoAccess,
    /// Status bar count of one container.
    DockerStatusOneContainer,
    /// Status bar count of containers. Placeholder: `{count}`.
    TplDockerStatusContainers,
    /// Status bar count of one image.
    DockerStatusOneImage,
    /// Status bar count of images. Placeholder: `{count}`.
    TplDockerStatusImages,
    /// Status bar count of one volume.
    DockerStatusOneVolume,
    /// Status bar count of volumes. Placeholder: `{count}`.
    TplDockerStatusVolumes,
    /// Status bar count of one network.
    DockerStatusOneNetwork,
    /// Status bar count of networks. Placeholder: `{count}`.
    TplDockerStatusNetworks,
    /// Status bar note that another engine answered too. Placeholder: `{engine}`.
    TplDockerStatusAlsoFound,
    /// Button and menu item that starts a stopped container.
    DockerActionStart,
    /// Button and menu item that stops a container.
    DockerActionStop,
    /// Button and menu item that restarts a container.
    DockerActionRestart,
    /// Button and menu item that pauses a running container.
    DockerActionPause,
    /// Button and menu item that lets a paused container run again.
    DockerActionUnpause,
    /// Button and menu item that opens a shell in a running container.
    DockerActionShell,
    /// Hint under a disabled Shell button: the `docker` command is missing.
    DockerShellNeedsDocker,
    /// Hint under a disabled Shell button: the `podman` command is missing.
    DockerShellNeedsPodman,
    /// Title of the terminal tab of a container shell. Placeholder: `{name}`.
    TplDockerShellTabTitle,
    /// Button, menu item and dialog action that removes a row.
    DockerActionRemove,
    /// Button above the containers table that removes every stopped container.
    DockerPruneStopped,
    /// Action of a prune confirmation dialog.
    DockerPrune,
    /// Title of the remove container dialog. Placeholder: `{name}`.
    TplDockerRemoveContainerTitle,
    /// Text of the remove container dialog.
    DockerRemoveContainerText,
    /// Switch in the remove dialog of a running container.
    DockerStopItFirst,
    /// Title of the prune stopped containers dialog.
    DockerPruneContainersTitle,
    /// Text of the prune stopped containers dialog. Placeholder: `{count}`.
    TplDockerPruneContainersText,
    /// Tooltip of the cross that hides a Docker notice.
    DockerDismiss,
    /// Notice when a container did not start. Placeholders: `{name}`, `{error}`.
    TplDockerStartFailed,
    /// Notice when a container did not stop. Placeholders: `{name}`, `{error}`.
    TplDockerStopFailed,
    /// Notice when a container did not restart. Placeholders: `{name}`, `{error}`.
    TplDockerRestartFailed,
    /// Notice when a container did not pause. Placeholders: `{name}`, `{error}`.
    TplDockerPauseFailed,
    /// Notice when a container did not unpause. Placeholders: `{name}`, `{error}`.
    TplDockerUnpauseFailed,
    /// Notice when a container was not removed. Placeholders: `{name}`, `{error}`.
    TplDockerRemoveContainerFailed,
    /// Notice when the stopped containers were not pruned. Placeholder: `{error}`.
    TplDockerPruneContainersFailed,
    /// Button and menu item that pulls the selected image again.
    DockerActionPullAgain,
    /// Button above the images table that removes dangling images.
    DockerPruneDangling,
    /// Button above the volumes and networks tables that removes unused ones.
    DockerPruneUnused,
    /// Title of the remove image dialog. Placeholder: `{name}`.
    TplDockerRemoveImageTitle,
    /// Text of the remove image dialog.
    DockerRemoveImageText,
    /// Switch in the remove image dialog when a container uses the image.
    DockerForceRemoveImage,
    /// Switch in the prune volumes dialog: remove named unused volumes too.
    DockerAlsoRemoveNamedVolumes,
    /// Title of the prune dangling images dialog.
    DockerPruneImagesTitle,
    /// Text of the prune dangling images dialog. Placeholder: `{count}`.
    TplDockerPruneImagesText,
    /// Title of the remove volume dialog. Placeholder: `{name}`.
    TplDockerRemoveVolumeTitle,
    /// Text of the remove volume dialog.
    DockerRemoveVolumeText,
    /// Title of the prune unused volumes dialog.
    DockerPruneVolumesTitle,
    /// Text of the prune unused volumes dialog. Placeholder: `{count}`.
    TplDockerPruneVolumesText,
    /// Title of the remove network dialog. Placeholder: `{name}`.
    TplDockerRemoveNetworkTitle,
    /// Text of the remove network dialog.
    DockerRemoveNetworkText,
    /// Title of the prune unused networks dialog.
    DockerPruneNetworksTitle,
    /// Text of the prune unused networks dialog. Placeholder: `{count}`.
    TplDockerPruneNetworksText,
    /// Hint under a disabled Remove of a volume a container uses. Placeholder: `{names}`.
    TplDockerVolumeInUseHint,
    /// Hint under a disabled Remove of a built-in network.
    DockerBuiltInNetworkHint,
    /// Hint under a disabled Remove of a network containers are connected to.
    DockerNetworkInUseHint,
    /// Label of the image field of the pull dialog.
    DockerFieldImage,
    /// Label of the tag field of the pull dialog.
    DockerFieldTag,
    /// Label of the name field of the create volume and create network dialogs.
    DockerFieldName,
    /// Label of the driver field of the create volume and create network dialogs.
    DockerFieldDriver,
    /// Placeholder of the image field of the pull dialog.
    DockerImagePlaceholder,
    /// Placeholder of the name field of the create volume dialog.
    DockerVolumeNamePlaceholder,
    /// Action button of the pull dialog.
    DockerActionPull,
    /// Action button of the create volume and create network dialogs.
    DockerActionCreate,
    /// Progress text of a running pull. Placeholder: `{image}`.
    TplDockerPulling,
    /// Bytes of a running pull. Placeholders: `{done}`, `{total}`.
    TplDockerPullAmount,
    /// Notice when a pull failed. Placeholders: `{name}`, `{error}`.
    TplDockerPullFailed,
    /// Notice when an image was not removed. Placeholders: `{name}`, `{error}`.
    TplDockerRemoveImageFailed,
    /// Notice when the dangling images were not pruned. Placeholder: `{error}`.
    TplDockerPruneImagesFailed,
    /// Notice when a volume was not created. Placeholder: `{error}`.
    TplDockerCreateVolumeFailed,
    /// Notice when a volume was not removed. Placeholders: `{name}`, `{error}`.
    TplDockerRemoveVolumeFailed,
    /// Notice when the unused volumes were not pruned. Placeholder: `{error}`.
    TplDockerPruneVolumesFailed,
    /// Notice when a network was not created. Placeholders: `{name}`, `{error}`.
    TplDockerCreateNetworkFailed,
    /// Notice when a network was not removed. Placeholders: `{name}`, `{error}`.
    TplDockerRemoveNetworkFailed,
    /// Notice when the unused networks were not pruned. Placeholder: `{error}`.
    TplDockerPruneNetworksFailed,
    // Docker mode inspector tabs.
    /// Inspector tab with the key facts of the selected row.
    DockerTabOverview,
    /// Inspector tab with the logs of the selected container.
    DockerTabLogs,
    /// Inspector tab with the engine's inspect JSON.
    DockerTabInspect,
    /// Hint above the logs of a container.
    DockerLogsHint,
    /// Label of the switch that keeps the logs at the newest line.
    DockerLogsFollow,
    /// Count of one log line waiting while Follow is off.
    DockerLogsOneNewLine,
    /// Count of log lines waiting while Follow is off. Placeholder: `{count}`.
    TplDockerLogsNewLines,
    /// Notice when the logs could not be read. Placeholder: `{error}`.
    TplDockerLogsFailed,
    /// Notice when the inspect JSON could not be loaded. Placeholder: `{error}`.
    TplDockerInspectFailed,
    /// Title of the group of top-level values in the inspect tree.
    DockerInspectGeneral,
    /// Label of the switch that shows empty values in the inspect tree.
    DockerInspectShowEmpty,
    /// Label of the switch that shows the inspect JSON instead of the tree.
    DockerInspectRawJson,
    /// Button that copies the inspect JSON.
    DockerInspectCopyJson,

    // Run container form (spec Decision 18).
    /// "Run" action on an image and the action button of the Run form.
    DockerActionRun,
    /// Line under the Run form title.
    DockerRunSubtitle,
    /// Section title: network, restart and auto remove.
    DockerRunSectionOptions,
    /// Label of the engine select.
    DockerFieldEngine,
    /// Placeholder of the optional container name.
    DockerRunNamePlaceholder,
    /// Switch: pull a missing image first.
    DockerRunPullMissing,
    /// Placeholder of the command word input.
    DockerRunCommandPlaceholder,
    /// Hint under the command while the image's command is not known.
    DockerRunCommandHint,
    /// Hint under the command. Placeholder: `{command}`.
    TplDockerRunCommandHint,
    /// Column heading of the host ports.
    DockerRunHostPort,
    /// Column heading of the container ports.
    DockerRunContainerPort,
    /// Button that adds a port row.
    DockerRunAddPort,
    /// Tooltip of the cross of a port row.
    DockerRunRemovePort,
    /// Hint under the ports. Placeholder: `{ports}`.
    TplDockerRunImageListens,
    /// Placeholder of an environment value.
    DockerRunEnvValue,
    /// Button that adds an environment variable.
    DockerRunAddVariable,
    /// Column heading of the volume sources.
    DockerRunVolumeSource,
    /// Column heading of the paths in the container.
    DockerRunVolumeTarget,
    /// Switch of a volume row.
    DockerRunReadOnly,
    /// Switch of a folder volume row on a computer with SELinux.
    DockerRunRelabel,
    /// Hint beside the SELinux switch of a volume row.
    DockerRunRelabelHint,
    /// Button that adds a volume row.
    DockerRunAddVolume,
    /// Tooltip of the cross of a volume row.
    DockerRunRemoveVolume,
    /// Label of the network select.
    DockerFieldNetwork,
    /// Label of the restart policy.
    DockerFieldRestart,
    /// Restart policy `no`.
    DockerRestartNo,
    /// Restart policy `on-failure`.
    DockerRestartOnFailure,
    /// Restart policy `unless-stopped`.
    DockerRestartUnlessStopped,
    /// Restart policy `always`.
    DockerRestartAlways,
    /// What the `no` policy does.
    DockerRestartHintNo,
    /// What the `on-failure` policy does.
    DockerRestartHintOnFailure,
    /// What the `unless-stopped` policy does.
    DockerRestartHintUnlessStopped,
    /// What the `always` policy does.
    DockerRestartHintAlways,
    /// Why the restart choices are disabled while auto remove is on.
    DockerRestartHintAutoRemove,
    /// Switch: remove the container when it stops.
    DockerRunAutoRemove,
    /// Error: the image is empty.
    DockerRunErrImage,
    /// Error: a port is not 1 to 65535.
    DockerRunErrPort,
    /// Error: a path in the container is not absolute.
    DockerRunErrContainerPath,
    /// Error: a volume row has no source.
    DockerRunErrSourceRequired,
    /// Error: a host folder does not exist.
    DockerRunErrSourceMissing,
    /// Footer hint of a ready form. Placeholder: `{engine}`.
    TplDockerRunReady,
    /// Footer hint while some fields have errors.
    DockerRunFixFields,
    /// Footer hint and image status while the image is looked up.
    DockerRunCheckingImage,
    /// Label of the action button while the run goes on.
    DockerRunStarting,
    /// Notice of a failed run. Placeholders: `{name}`, `{error}`.
    TplDockerRunFailed,
    /// Hint at the top of the image panel.
    DockerRunPanelHint,
    /// Image status: on this computer. Placeholder: `{size}`.
    TplDockerImageHere,
    /// Image status: not on this computer.
    DockerImageMissing,
    /// Image hint: missing, the pull switch is on.
    DockerRunImagePulledFirst,
    /// Image hint: missing, the pull switch is off.
    DockerRunImageNotPulled,
    /// Image panel field: exposed ports.
    DockerFieldListensOn,
    /// Image panel field: the entrypoint.
    DockerFieldEntrypoint,
    /// Image panel field: the working directory.
    DockerFieldWorkdir,
    /// Image panel text while no image is typed.
    DockerRunNoImage,
    /// Image panel notice of a failed lookup. Placeholder: `{error}`.
    TplDockerImageLookupFailed,
    /// Overview field: the mounts of a container.
    DockerFieldMounts,
    /// Overview field: the Compose project.
    DockerFieldCompose,
    /// Overview field: the command of a container.
    DockerFieldCommand,
    /// Overview field: when the item was made.
    DockerFieldCreated,
    /// Overview field: where a volume keeps its data.
    DockerFieldMountpoint,
    /// Overview field: the gateway of a network.
    DockerFieldGateway,
    // Command palette and its title bar trigger.
    /// Title bar search text while no tab is open.
    TitleSearchNoSessions,
    /// Text in the empty palette input.
    PalettePlaceholder,
    /// Line shown when no palette entry matches the query.
    PaletteNoMatches,
    /// Footer hint after the ↑↓ keys.
    PaletteHintMove,
    /// Footer hint after the Enter key.
    PaletteHintOpen,
    /// Group label of the app actions.
    PaletteGroupActions,
    /// Action that opens a terminal tab.
    PaletteNewTerminal,
    /// Action that opens the editor for a new launch.
    PaletteNewLaunch,
    /// Action that starts a new launch folder.
    PaletteNewFolder,
    /// Action that splits the focused pane.
    PaletteSplitPane,
    /// Closing a session: the palette action and the tooltip of the close
    /// cross on a session row.
    CloseSession,
    /// Action that shows or hides the left panel.
    PaletteToggleLeftPanel,
    /// Action that shows or hides the files panel.
    PaletteToggleFilesPanel,
    /// Action that switches to Terminal mode.
    PaletteSwitchToTerminalMode,
    /// Action that switches to Docker mode.
    PaletteSwitchToDockerMode,

    // Quick launch wizard.
    /// Title of the form that creates a launch.
    WizardTitleNew,
    /// Title of the form that edits a launch.
    WizardTitleEdit,
    /// Muted line under the launch form title.
    WizardSubtitle,
    /// Entry title field label.
    FieldTitle,
    /// Command type field label.
    FieldType,
    /// Custom command type option.
    TypeCustom,
    /// SSH command type option.
    TypeSsh,
    /// Custom command section header.
    SectionCustomCommand,
    /// Program path field label.
    FieldProgram,
    /// Program arguments field label.
    FieldArguments,
    /// Working directory field label.
    FieldWorkdir,
    /// SSH connection section header.
    SectionSshConnection,
    /// SSH host field label.
    FieldHost,
    /// SSH port field label.
    FieldPort,
    /// SSH user field label.
    FieldUser,
    /// SSH identity file field label.
    FieldIdentityFile,
    /// Additional SSH argument field label.
    FieldExtraArgs,
    /// Environment variable list label.
    FieldEnvironment,
    /// Label of the choice for what a tab does once its command exits.
    FieldAfterExit,
    /// Muted hint under the exit choice.
    ExitBehaviorHint,
    /// Exit choice: close the tab.
    ExitBehaviorClose,
    /// Exit choice: close the tab only when the command succeeds.
    ExitBehaviorCloseOnSuccess,
    /// Exit choice: keep the tab open.
    ExitBehaviorKeepOpen,
    /// Environment variable name placeholder.
    EnvKeyPlaceholder,
    /// Environment variable value placeholder.
    EnvValuePlaceholder,
    /// Shown when the custom editor state is inconsistent.
    InvalidCustomEditorState,
    /// Shown when the SSH editor state is inconsistent.
    InvalidSshEditorState,

    // Shared buttons.
    /// Persist pending changes.
    ButtonSave,
    /// Discard pending changes.
    ButtonCancel,
    /// Remove a list entry.
    ButtonRemove,
    /// Append a list entry.
    ButtonAdd,
    /// Append an environment variable entry.
    ButtonAddEnv,
    /// Restore settings to the persisted baseline.
    ButtonReset,
    /// Connect with the secret typed in the SSH secret dialog.
    ButtonConnect,

    // SSH secret dialog.
    /// Dialog title when a private key needs its passphrase.
    SecretPromptPassphraseTitle,
    /// Dialog title when the server needs a password.
    SecretPromptPasswordTitle,
    /// Shown when the typed passphrase did not unlock the key.
    SecretPromptWrongPassphrase,
    /// Shown when the server rejected the typed password.
    SecretPromptPasswordRejected,
    /// Field label for the key passphrase.
    SecretPromptPassphraseLabel,
    /// Field label for the account password.
    SecretPromptPasswordLabel,

    // Settings sections and fields.
    /// General settings section.
    SectionGeneral,
    /// Terminal settings section.
    SectionTerminal,
    /// Appearance settings section.
    SectionAppearance,
    /// Interface language field label.
    FieldLanguage,
    /// Shell command field label.
    FieldShell,
    /// Default editor field label.
    FieldDefaultEditor,
    /// Equalize panes setting label.
    OptionEqualizePanes,
    /// Line under the equalize panes setting.
    DescEqualizePanes,
    /// Theme selector field label.
    FieldTheme,
    /// Line under the theme setting.
    DescTheme,
    /// Line under the language setting.
    DescLanguage,
    /// Line under the shell setting.
    DescShell,
    /// Line under the default editor setting.
    DescDefaultEditor,
    /// Language option that follows the operating system locale.
    LanguageSystem,

    // Quick launch errors.
    /// Prefix for I/O error messages.
    ErrIoPrefix,
    /// Prefix for JSON error messages.
    ErrJsonPrefix,
    /// Rejected because the title was blank.
    ErrTitleEmpty,
    /// Rejected because a sibling already uses the title.
    ErrTitleDuplicate,
    /// Wizard validation: title missing.
    ErrTitleRequired,
    /// Wizard validation: program missing.
    ErrProgramRequired,
    /// Wizard validation: host missing.
    ErrHostRequired,
    /// Wizard validation: port is not numeric.
    ErrInvalidPort,
    /// Wizard validation: custom draft absent.
    ErrMissingCustomDraft,
    /// Wizard validation: SSH draft absent.
    ErrMissingSshDraft,
    /// SSH validation: port is not positive.
    ErrSshPortPositive,
    /// Wizard save target folder is missing.
    ErrMissingTargetFolder,
    /// Wizard save parent folder is missing.
    ErrMissingParentFolder,
    /// Wizard edit target no longer exists.
    ErrCommandNoLongerExists,

    // Templates. These carry `{placeholder}` markers substituted at call time.
    /// Tab title for editing an existing quick launch. Placeholder: `{title}`.
    TplEditTabTitle,
    /// Error tab title for a failed launch. Placeholder: `{title}`.
    TplLaunchFailedTitle,
    /// Error tab body for a failed launch. Placeholders: `{command}`, `{error}`.
    TplLaunchFailedBody,
    /// Message for a terminal that failed to start. Placeholder: `{error}`.
    TplTerminalInitFailed,
    /// SSH connection failure. Placeholder: `{error}`.
    TplSshConnectionFailed,
    /// Missing working directory. Placeholder: `{path}`.
    TplWorkingDirectoryNotFound,
    /// Working path is not a directory. Placeholder: `{path}`.
    TplWorkingDirectoryNotDirectory,
    /// Missing SSH identity file. Placeholder: `{path}`.
    TplIdentityFileNotFound,
    /// SSH identity path is not a file. Placeholder: `{path}`.
    TplIdentityFileNotFile,
    /// SSH secret dialog title for a key passphrase. Placeholder: `{path}`.
    TplSecretPromptPassphraseTitle,
    /// SSH secret dialog title for a password. Placeholder: `{target}`.
    TplSecretPromptPasswordTitle,
    /// Muted line under the passphrase title. Placeholders: `{launch}`,
    /// `{account}`.
    TplSecretPromptPassphraseHint,
    /// Muted line under the password title. Placeholders: `{launch}`,
    /// `{account}`.
    TplSecretPromptPasswordHint,
    /// Program is absent from `PATH`. Placeholder: `{program}`.
    TplProgramNotFoundInPath,
    /// Explicit program path is missing. Placeholder: `{program}`.
    TplProgramNotFound,
    /// Program path is a directory. Placeholder: `{program}`.
    TplProgramIsDirectory,
    /// Program path is not executable. Placeholder: `{program}`.
    TplProgramNotExecutable,
    /// Banner on a kept-open pane whose process exited. Placeholder: `{code}`.
    TplProcessExited,
}

impl Key {
    /// Every key in the catalogs, used by tests to verify full coverage.
    #[cfg(test)]
    pub(super) const ALL: [Self; 407] = [
        Self::MenuNewTerminal,
        Self::MenuSplitCurrentPane,
        Self::MenuNewLaunch,
        Self::NewSessionMenu,
        Self::MenuNewFolder,
        Self::LaunchesMenu,
        Self::TerminalTabNotInitialized,
        Self::QuickLaunchEditorNotInitialized,
        Self::QuickLaunchErrorPayloadMissing,
        Self::TerminalUnavailable,
        Self::ProcessTerminated,
        Self::SettingsTabTitle,
        Self::QuickLaunchCreateTabTitle,
        Self::CopySelection,
        Self::Paste,
        Self::CopyContent,
        Self::CopyPrompt,
        Self::CopyCommand,
        Self::SplitDown,
        Self::SplitRight,
        Self::ClosePane,
        Self::ThisComputer,
        Self::WindowToggleMaximize,
        Self::WindowMinimize,
        Self::WindowFilesPanel,
        Self::WindowLeftPanel,
        Self::WindowFocusMode,
        Self::FocusNoteHidden,
        Self::FocusNoteShow,
        Self::WindowClose,
        Self::Kill,
        Self::Edit,
        Self::Rename,
        Self::Duplicate,
        Self::Remove,
        Self::CreateFolder,
        Self::CreateQuickLaunch,
        Self::Delete,
        Self::CreateLaunch,
        Self::SessionsPanelTitle,
        Self::NewSession,
        Self::ThisComputerNote,
        Self::SshMachineNote,
        Self::SessionStateSettings,
        Self::SessionStateEditor,
        Self::SessionStateFailed,
        Self::NoOpenSessions,
        Self::EmptyTitle,
        Self::EmptyDescription,
        Self::EmptyNewTerminal,
        Self::EmptyNewLaunch,
        Self::LaunchesPanelTitle,
        Self::LaunchRunning,
        Self::NoActiveFolder,
        Self::FilesTitle,
        Self::StatusPaneOne,
        Self::TplStatusPanes,
        Self::StatusSessionOne,
        Self::TplStatusSessions,
        Self::ModeTerminal,
        Self::ModeDocker,
        Self::ModesAllModes,
        Self::ModesPinnedSection,
        Self::TplModesPinnedNote,
        Self::ModesOtherSection,
        Self::ModesPin,
        Self::ModesUnpin,
        Self::ModesUnpinAnotherFirst,
        Self::ModesNoOtherModes,
        Self::DockerContainers,
        Self::DockerImages,
        Self::DockerVolumes,
        Self::DockerNetworks,
        Self::DockerRunContainer,
        Self::DockerPullImage,
        Self::DockerCreateVolume,
        Self::DockerCreateNetwork,
        Self::DockerRefresh,
        Self::DockerMoreActions,
        Self::DockerFilterPlaceholder,
        Self::TplDockerRunning,
        Self::TplDockerComposeChip,
        Self::DockerShowAllContainers,
        Self::DockerListFailedShort,
        Self::DockerLoading,
        Self::DockerNoContainers,
        Self::DockerNoImages,
        Self::DockerNoVolumes,
        Self::DockerNoNetworks,
        Self::TplDockerNothingMatches,
        Self::TplDockerListFailed,
        Self::DockerSearching,
        Self::DockerNotInstalledTitle,
        Self::DockerNotInstalledText,
        Self::DockerInstallDocker,
        Self::DockerInstallPodman,
        Self::DockerCheckAgain,
        Self::DockerCopy,
        Self::DockerHowToFix,
        Self::DockerRunInTerminal,
        Self::DockerStoppedTitle,
        Self::DockerStoppedText,
        Self::DockerStoppedHint,
        Self::DockerPodmanOffTitle,
        Self::DockerPodmanOffText,
        Self::DockerPodmanOffHint,
        Self::DockerNoAccessTitle,
        Self::TplDockerNoAccessText,
        Self::DockerNoAccessLabel,
        Self::DockerNoAccessHint,
        Self::DockerPanelNotInstalled,
        Self::DockerPanelStopped,
        Self::DockerPanelPodmanOff,
        Self::DockerPanelNoAccess,
        Self::DockerNothingToInspect,
        Self::TplDockerLookedFor,
        Self::DockerColumnName,
        Self::DockerColumnStatus,
        Self::DockerColumnImage,
        Self::DockerColumnPorts,
        Self::DockerColumnAge,
        Self::DockerColumnRepositoryTag,
        Self::DockerColumnId,
        Self::DockerColumnSize,
        Self::DockerColumnUsedBy,
        Self::DockerColumnDriver,
        Self::DockerColumnScope,
        Self::DockerColumnSubnet,
        Self::DockerStateRunning,
        Self::DockerStatePaused,
        Self::DockerStateRestarting,
        Self::DockerStateCreated,
        Self::DockerStateExited,
        Self::TplDockerStateExitedCode,
        Self::DockerStateDead,
        Self::DockerStateRemoving,
        Self::DockerStateStopping,
        Self::DockerStateUnknown,
        Self::DockerImageDangling,
        Self::DockerUsedByOneContainer,
        Self::TplDockerUsedByContainers,
        Self::DockerNotUsed,
        Self::TplDockerUsedBy,
        Self::DockerBuiltInNetwork,
        Self::DockerOneContainerConnected,
        Self::TplDockerContainersConnected,
        Self::DockerAgeUnderMinute,
        Self::TplDockerAgeMinutes,
        Self::TplDockerAgeHours,
        Self::TplDockerAgeDays,
        Self::TplDockerAgeWeeks,
        Self::TplDockerAgeMonths,
        Self::TplDockerAgeYears,
        Self::DockerStatusMode,
        Self::DockerStatusSearching,
        Self::DockerStatusNoEngine,
        Self::DockerStatusStopped,
        Self::DockerStatusPodmanOff,
        Self::TplDockerStatusNoAccess,
        Self::DockerStatusOneContainer,
        Self::TplDockerStatusContainers,
        Self::DockerStatusOneImage,
        Self::TplDockerStatusImages,
        Self::DockerStatusOneVolume,
        Self::TplDockerStatusVolumes,
        Self::DockerStatusOneNetwork,
        Self::TplDockerStatusNetworks,
        Self::TplDockerStatusAlsoFound,
        Self::DockerActionStart,
        Self::DockerActionStop,
        Self::DockerActionRestart,
        Self::DockerActionPause,
        Self::DockerActionUnpause,
        Self::DockerActionShell,
        Self::DockerShellNeedsDocker,
        Self::DockerShellNeedsPodman,
        Self::TplDockerShellTabTitle,
        Self::DockerActionRemove,
        Self::DockerPruneStopped,
        Self::DockerPrune,
        Self::TplDockerRemoveContainerTitle,
        Self::DockerRemoveContainerText,
        Self::DockerStopItFirst,
        Self::DockerPruneContainersTitle,
        Self::TplDockerPruneContainersText,
        Self::DockerDismiss,
        Self::TplDockerStartFailed,
        Self::TplDockerStopFailed,
        Self::TplDockerRestartFailed,
        Self::TplDockerPauseFailed,
        Self::TplDockerUnpauseFailed,
        Self::TplDockerRemoveContainerFailed,
        Self::TplDockerPruneContainersFailed,
        Self::DockerActionPullAgain,
        Self::DockerPruneDangling,
        Self::DockerPruneUnused,
        Self::TplDockerRemoveImageTitle,
        Self::DockerRemoveImageText,
        Self::DockerForceRemoveImage,
        Self::DockerAlsoRemoveNamedVolumes,
        Self::DockerPruneImagesTitle,
        Self::TplDockerPruneImagesText,
        Self::TplDockerRemoveVolumeTitle,
        Self::DockerRemoveVolumeText,
        Self::DockerPruneVolumesTitle,
        Self::TplDockerPruneVolumesText,
        Self::TplDockerRemoveNetworkTitle,
        Self::DockerRemoveNetworkText,
        Self::DockerPruneNetworksTitle,
        Self::TplDockerPruneNetworksText,
        Self::TplDockerVolumeInUseHint,
        Self::DockerBuiltInNetworkHint,
        Self::DockerNetworkInUseHint,
        Self::DockerFieldImage,
        Self::DockerFieldTag,
        Self::DockerFieldName,
        Self::DockerFieldDriver,
        Self::DockerImagePlaceholder,
        Self::DockerVolumeNamePlaceholder,
        Self::DockerActionPull,
        Self::DockerActionCreate,
        Self::TplDockerPulling,
        Self::TplDockerPullAmount,
        Self::TplDockerPullFailed,
        Self::TplDockerRemoveImageFailed,
        Self::TplDockerPruneImagesFailed,
        Self::TplDockerCreateVolumeFailed,
        Self::TplDockerRemoveVolumeFailed,
        Self::TplDockerPruneVolumesFailed,
        Self::TplDockerCreateNetworkFailed,
        Self::TplDockerRemoveNetworkFailed,
        Self::TplDockerPruneNetworksFailed,
        Self::DockerTabOverview,
        Self::DockerTabLogs,
        Self::DockerTabInspect,
        Self::DockerLogsHint,
        Self::DockerLogsFollow,
        Self::DockerLogsOneNewLine,
        Self::TplDockerLogsNewLines,
        Self::TplDockerLogsFailed,
        Self::TplDockerInspectFailed,
        Self::DockerInspectGeneral,
        Self::DockerInspectShowEmpty,
        Self::DockerInspectRawJson,
        Self::DockerInspectCopyJson,
        Self::DockerActionRun,
        Self::DockerRunSubtitle,
        Self::DockerRunSectionOptions,
        Self::DockerFieldEngine,
        Self::DockerRunNamePlaceholder,
        Self::DockerRunPullMissing,
        Self::DockerRunCommandPlaceholder,
        Self::DockerRunCommandHint,
        Self::TplDockerRunCommandHint,
        Self::DockerRunHostPort,
        Self::DockerRunContainerPort,
        Self::DockerRunAddPort,
        Self::DockerRunRemovePort,
        Self::TplDockerRunImageListens,
        Self::DockerRunEnvValue,
        Self::DockerRunAddVariable,
        Self::DockerRunVolumeSource,
        Self::DockerRunVolumeTarget,
        Self::DockerRunReadOnly,
        Self::DockerRunRelabel,
        Self::DockerRunRelabelHint,
        Self::DockerRunAddVolume,
        Self::DockerRunRemoveVolume,
        Self::DockerFieldNetwork,
        Self::DockerFieldRestart,
        Self::DockerRestartNo,
        Self::DockerRestartOnFailure,
        Self::DockerRestartUnlessStopped,
        Self::DockerRestartAlways,
        Self::DockerRestartHintNo,
        Self::DockerRestartHintOnFailure,
        Self::DockerRestartHintUnlessStopped,
        Self::DockerRestartHintAlways,
        Self::DockerRestartHintAutoRemove,
        Self::DockerRunAutoRemove,
        Self::DockerRunErrImage,
        Self::DockerRunErrPort,
        Self::DockerRunErrContainerPath,
        Self::DockerRunErrSourceRequired,
        Self::DockerRunErrSourceMissing,
        Self::TplDockerRunReady,
        Self::DockerRunFixFields,
        Self::DockerRunCheckingImage,
        Self::DockerRunStarting,
        Self::TplDockerRunFailed,
        Self::DockerRunPanelHint,
        Self::TplDockerImageHere,
        Self::DockerImageMissing,
        Self::DockerRunImagePulledFirst,
        Self::DockerRunImageNotPulled,
        Self::DockerFieldListensOn,
        Self::DockerFieldEntrypoint,
        Self::DockerFieldWorkdir,
        Self::DockerRunNoImage,
        Self::TplDockerImageLookupFailed,
        Self::DockerFieldMounts,
        Self::DockerFieldCompose,
        Self::DockerFieldCommand,
        Self::DockerFieldCreated,
        Self::DockerFieldMountpoint,
        Self::DockerFieldGateway,
        Self::TitleSearchNoSessions,
        Self::PalettePlaceholder,
        Self::PaletteNoMatches,
        Self::PaletteHintMove,
        Self::PaletteHintOpen,
        Self::PaletteGroupActions,
        Self::PaletteNewTerminal,
        Self::PaletteNewLaunch,
        Self::PaletteNewFolder,
        Self::PaletteSplitPane,
        Self::CloseSession,
        Self::PaletteToggleLeftPanel,
        Self::PaletteToggleFilesPanel,
        Self::PaletteSwitchToTerminalMode,
        Self::PaletteSwitchToDockerMode,
        Self::WizardTitleNew,
        Self::WizardTitleEdit,
        Self::WizardSubtitle,
        Self::ExitBehaviorHint,
        Self::FieldTitle,
        Self::FieldType,
        Self::TypeCustom,
        Self::TypeSsh,
        Self::SectionCustomCommand,
        Self::FieldProgram,
        Self::FieldArguments,
        Self::FieldWorkdir,
        Self::SectionSshConnection,
        Self::FieldHost,
        Self::FieldPort,
        Self::FieldUser,
        Self::FieldIdentityFile,
        Self::FieldExtraArgs,
        Self::FieldEnvironment,
        Self::FieldAfterExit,
        Self::ExitBehaviorClose,
        Self::ExitBehaviorCloseOnSuccess,
        Self::ExitBehaviorKeepOpen,
        Self::EnvKeyPlaceholder,
        Self::EnvValuePlaceholder,
        Self::InvalidCustomEditorState,
        Self::InvalidSshEditorState,
        Self::ButtonSave,
        Self::ButtonCancel,
        Self::ButtonRemove,
        Self::ButtonAdd,
        Self::ButtonAddEnv,
        Self::ButtonReset,
        Self::ButtonConnect,
        Self::SecretPromptPassphraseTitle,
        Self::SecretPromptPasswordTitle,
        Self::SecretPromptWrongPassphrase,
        Self::SecretPromptPasswordRejected,
        Self::SecretPromptPassphraseLabel,
        Self::SecretPromptPasswordLabel,
        Self::SectionGeneral,
        Self::SectionTerminal,
        Self::SectionAppearance,
        Self::FieldLanguage,
        Self::FieldShell,
        Self::FieldDefaultEditor,
        Self::OptionEqualizePanes,
        Self::DescEqualizePanes,
        Self::FieldTheme,
        Self::DescTheme,
        Self::DescLanguage,
        Self::DescShell,
        Self::DescDefaultEditor,
        Self::LanguageSystem,
        Self::ErrIoPrefix,
        Self::ErrJsonPrefix,
        Self::ErrTitleEmpty,
        Self::ErrTitleDuplicate,
        Self::ErrTitleRequired,
        Self::ErrProgramRequired,
        Self::ErrHostRequired,
        Self::ErrInvalidPort,
        Self::ErrMissingCustomDraft,
        Self::ErrMissingSshDraft,
        Self::ErrSshPortPositive,
        Self::ErrMissingTargetFolder,
        Self::ErrMissingParentFolder,
        Self::ErrCommandNoLongerExists,
        Self::TplEditTabTitle,
        Self::TplLaunchFailedTitle,
        Self::TplLaunchFailedBody,
        Self::TplTerminalInitFailed,
        Self::TplSshConnectionFailed,
        Self::TplWorkingDirectoryNotFound,
        Self::TplWorkingDirectoryNotDirectory,
        Self::TplIdentityFileNotFound,
        Self::TplIdentityFileNotFile,
        Self::TplSecretPromptPassphraseTitle,
        Self::TplSecretPromptPasswordTitle,
        Self::TplSecretPromptPassphraseHint,
        Self::TplSecretPromptPasswordHint,
        Self::TplProgramNotFoundInPath,
        Self::TplProgramNotFound,
        Self::TplProgramIsDirectory,
        Self::TplProgramNotExecutable,
        Self::TplProcessExited,
    ];
}
