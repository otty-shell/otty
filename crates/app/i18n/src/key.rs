use serde::Deserialize;

/// Identifier for a user-facing string in the translation catalogs.
///
/// Variants map to `snake_case` keys in the locale JSON files under
/// `locales/`. Every variant must be present in every catalog; the
/// `catalogs_cover_every_key` test enforces that, and an unknown key in a JSON
/// file fails deserialization at startup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Key {
    // Header menus (New session, Launches).
    /// "New terminal" entry in the New session menu.
    MenuNewTerminal,
    /// "Split current pane right" entry in the New session menu.
    MenuSplitCurrentPane,
    /// "Split current pane down" entry in the New session menu.
    MenuSplitCurrentPaneDown,
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
    /// Shown when the terminal widget cannot be rendered.
    TerminalUnavailable,
    /// Banner on a kept-open pane whose process ended without an exit code.
    ProcessTerminated,
    /// Status of a command block whose command still runs.
    BlockRunning,
    /// Badge after the status of a folded block that hides one line.
    BlockOneLineHidden,
    /// Name of the arrow that folds a command block.
    BlockCollapse,
    /// Name of the arrow that unfolds a folded command block.
    BlockExpand,
    /// Sticky header of a block whose start is above the pane: one line of
    /// the block is above it.
    BlockOneLineAbove,
    /// Name of the button in the sticky header that scrolls to the start
    /// of its block.
    BlockGoToStart,

    // Tab titles.
    /// Title of the quick launch creation wizard tab.
    QuickLaunchCreateTabTitle,

    // Terminal pane context menu.
    /// Copy the current text selection.
    CopySelection,
    /// Paste clipboard contents into the terminal.
    Paste,
    /// Copy the selected block's output.
    CopyContent,
    /// Copy the selected block's command line.
    CopyCommand,
    /// Fold the selected block.
    Collapse,
    /// Unfold the selected folded block.
    Expand,
    /// Type the command of the selected block at the prompt and run it.
    RunAgain,
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
    /// Tooltip of the title bar button that shows or hides the right panel.
    WindowRightPanel,
    /// Tooltip of the right panel button in a mode without that panel.
    WindowRightPanelNotUsed,
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
    /// State word on a launch editor row.
    SessionStateEditor,
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
    /// Files context menu: open the file in the editor or the folder in the
    /// tree.
    FilesOpen,
    /// Files context menu: open a new terminal tab in the folder.
    FilesOpenInTerminal,
    /// Files context menu: name a new file in the folder.
    FilesNewFile,
    /// A new or renamed entry in Files has a blank name.
    FilesNameEmpty,
    /// A new or renamed entry in Files has a name that is no single path
    /// part: it holds a slash or is `.` or `..`.
    FilesNameInvalid,
    /// A new or renamed entry in Files takes the name of an entry next to it.
    FilesNameTaken,
    /// Text of the dialog that asks before a file is deleted.
    FilesDeleteFileText,
    /// Text of the dialog that asks before a folder is deleted.
    FilesDeleteFolderText,

    // Status bar.
    /// Status bar text for exactly one pane.
    StatusPaneOne,
    /// Status bar pane count. Placeholder: `{count}`.
    TplStatusPanes,
    /// Status bar text for exactly one session.
    StatusSessionOne,
    /// Status bar session count. Placeholder: `{count}`.
    TplStatusSessions,
    /// Status bar counter of the selected command block of the focused
    /// pane, e.g. "Block 5 of 6". Placeholders: `{number}`, the place of
    /// the block counted from the oldest, and `{count}`, the number of
    /// command blocks.
    TplStatusBlockPlace,
    // Modes: the switcher and the All modes menu.
    /// Name of the Terminal mode.
    ModeTerminal,
    /// Name of the Containers mode.
    ModeContainers,
    /// Name of the Agents mode.
    ModeAgents,
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
    // Containers mode: engine tree, tables and inspector.
    /// Containers list in the engine tree and the main area title; also the
    /// Networks column with the container count.
    ContainersContainers,
    /// Images list in the engine tree and the main area title.
    ContainersImages,
    /// Volumes list in the engine tree and the main area title.
    ContainersVolumes,
    /// Networks list in the engine tree and the main area title.
    ContainersNetworks,
    /// Create button above the containers table.
    ContainersRunContainer,
    /// Create button above the images table.
    ContainersPullImage,
    /// Create button above the volumes table.
    ContainersCreateVolume,
    /// Create button above the networks table.
    ContainersCreateNetwork,
    /// Tooltip of the button that loads every list again.
    ContainersRefresh,
    /// Tooltip of the "⋯" button that holds the header actions that do not
    /// fit, and of the "⋯" button at the end of a table row.
    ContainersMoreActions,
    /// Placeholder of the tag search above the table.
    ContainersSearchPlaceholder,
    /// Tooltip of the cross of a tag above the table.
    ContainersTagRemove,
    /// Button at the end of the tags above the table.
    ContainersTagsClearAll,
    /// Running containers next to Containers in the tree. Placeholder:
    /// `{count}`.
    TplContainersRunning,
    /// Word next to a tree row whose list failed to load.
    ContainersListFailedShort,
    /// Table message while a list loads.
    ContainersLoading,
    /// Message of an empty containers table.
    ContainersNoContainers,
    /// Message of an empty images table.
    ContainersNoImages,
    /// Message of an empty volumes table.
    ContainersNoVolumes,
    /// Message of an empty networks table.
    ContainersNoNetworks,
    /// Table message when no row passes the search.
    ContainersNothingMatches,
    /// Notice above a table whose list failed to load. Placeholder: `{error}`.
    TplContainersListFailed,
    /// Title of the card when neither Docker nor Podman is installed.
    ContainersNotInstalledTitle,
    /// Text of the card when neither Docker nor Podman is installed.
    ContainersNotInstalledText,
    /// Button that opens the Docker install page.
    ContainersInstallDocker,
    /// Button that opens the Podman install page.
    ContainersInstallPodman,
    /// Button that searches for engines again.
    ContainersCheckAgain,
    /// Button that copies a command to the clipboard.
    ContainersCopy,
    /// Button that opens the documentation of the fix.
    ContainersHowToFix,
    /// Button that opens how to fix Docker, when Docker and Podman are both off.
    ContainersHowToFixDocker,
    /// Button that opens how to fix Podman, when Docker and Podman are both off.
    ContainersHowToFixPodman,
    /// Label above a command to run in a terminal.
    ContainersRunInTerminal,
    /// Title of the card when Docker is installed but not running.
    ContainersDockerStoppedTitle,
    /// Text of the card when Docker is installed but not running.
    ContainersDockerStoppedText,
    /// Hint under the command that starts Docker, for macOS.
    ContainersDockerStoppedHint,
    /// Title of the card when the Podman API service is off.
    ContainersPodmanOffTitle,
    /// Text of the card when the Podman API service is off.
    ContainersPodmanOffText,
    /// Hint under the command that turns on Podman, for macOS.
    ContainersPodmanOffHint,
    /// Title of the card when Docker and Podman are installed and neither
    /// answers.
    ContainersBothOffTitle,
    /// Description of the card when Docker and Podman are both off.
    ContainersBothOffText,
    /// Label over the Docker command when Docker and Podman are both off.
    ContainersBothOffDockerLabel,
    /// Label over the Podman command when Docker and Podman are both off.
    ContainersBothOffPodmanLabel,
    /// Title of the card when OTTY may not use the Docker socket.
    ContainersDockerNoAccessTitle,
    /// Text of the no access card. Placeholder: `{socket}`.
    TplContainersDockerNoAccessText,
    /// Label above the command that adds the user to the docker group.
    ContainersDockerNoAccessLabel,
    /// Hint under the command: log out again; the group has root rights.
    ContainersDockerNoAccessHint,
    /// Title of the card when OTTY may not use a Podman socket.
    ContainersPodmanNoAccessTitle,
    /// Text of the Podman no access card. Placeholder: `{socket}`.
    TplContainersPodmanNoAccessText,
    /// Label above the command that turns on the user's own Podman socket.
    ContainersPodmanNoAccessLabel,
    /// Hint under that command: whose containers it shows, and macOS.
    ContainersPodmanNoAccessHint,
    /// Title of the card when the Docker socket is denied and Podman is off.
    ContainersDockerNoAccessPodmanOffTitle,
    /// Text of that card. Placeholder: `{socket}`.
    TplContainersDockerNoAccessPodmanOffText,
    /// Title of the card when a Podman socket is denied and Docker is off.
    ContainersPodmanNoAccessDockerOffTitle,
    /// Text of that card. Placeholder: `{socket}`.
    TplContainersPodmanNoAccessDockerOffText,
    /// Left panel text when no engine is installed.
    ContainersPanelNotInstalled,
    /// Left panel text when Docker is not running.
    ContainersPanelDockerStopped,
    /// Left panel text when the Podman API service is off.
    ContainersPanelPodmanOff,
    /// Left panel text when Docker and Podman are both off.
    ContainersPanelBothOff,
    /// Left panel text when OTTY may not use the Docker socket.
    ContainersPanelDockerNoAccess,
    /// Left panel text when OTTY may not use a Podman socket.
    ContainersPanelPodmanNoAccess,
    /// Left panel text when the Docker socket is denied and Podman is off.
    ContainersPanelDockerNoAccessPodmanOff,
    /// Left panel text when a Podman socket is denied and Docker is off.
    ContainersPanelPodmanNoAccessDockerOff,
    /// Right panel text when no row is selected.
    ContainersNothingToInspect,
    /// Line under the no engine cards. Placeholder: `{places}`.
    TplContainersLookedFor,
    /// Line under the no engine cards when no socket was tried.
    ContainersLookedForCommands,
    /// Table column: the name of a container, volume or network.
    ContainersColumnName,
    /// Table column: the state of a container.
    ContainersColumnStatus,
    /// Table column: the image of a container.
    ContainersColumnImage,
    /// Table column: the ports of a container.
    ContainersColumnPorts,
    /// Table column: how long ago the item was created.
    ContainersColumnAge,
    /// Table column: the repository of an image.
    ContainersColumnRepository,
    /// Table column: the tag of an image.
    ContainersColumnTag,
    /// Table column: the short ID of an image.
    ContainersColumnId,
    /// Table column: the size of an image.
    ContainersColumnSize,
    /// Table column: the containers that use an image or a volume.
    ContainersColumnUsedBy,
    /// Table column: the driver of a volume or a network.
    ContainersColumnDriver,
    /// Table column: the scope of a network.
    ContainersColumnScope,
    /// Table column: the subnet of a network.
    ContainersColumnSubnet,
    /// Container state badge: running.
    ContainersStateRunning,
    /// Container state badge: paused.
    ContainersStatePaused,
    /// Container state badge: restarting.
    ContainersStateRestarting,
    /// Container state badge: created, never started.
    ContainersStateCreated,
    /// Container state badge: exited, code unknown.
    ContainersStateExited,
    /// Container state badge: exited with a code. Placeholder: `{code}`.
    TplContainersStateExitedCode,
    /// Container state badge: dead.
    ContainersStateDead,
    /// Container state badge: being removed.
    ContainersStateRemoving,
    /// Container state badge: stopping.
    ContainersStateStopping,
    /// Container state badge: a state OTTY does not know.
    ContainersStateUnknown,
    /// Inspector status of an image without a tag that no container uses.
    ContainersImageDangling,
    /// Inspector status of an image one container uses.
    ContainersUsedByOneContainer,
    /// Inspector status of an image several containers use. Placeholder:
    /// `{count}`.
    TplContainersUsedByContainers,
    /// Inspector status of an image or volume no container uses.
    ContainersNotUsed,
    /// Inspector status of a volume: the containers that use it. Placeholder:
    /// `{names}`.
    TplContainersUsedBy,
    /// Inspector status of a network the engine creates itself.
    ContainersBuiltInNetwork,
    /// Inspector status of a network with one container.
    ContainersOneContainerConnected,
    /// Inspector status of a network with several containers. Placeholder:
    /// `{count}`.
    TplContainersContainersConnected,
    /// Age of an item created less than a minute ago.
    ContainersAgeUnderMinute,
    /// Age in minutes. Placeholder: `{count}`.
    TplContainersAgeMinutes,
    /// Age in hours. Placeholder: `{count}`.
    TplContainersAgeHours,
    /// Age in days. Placeholder: `{count}`.
    TplContainersAgeDays,
    /// Age in weeks. Placeholder: `{count}`.
    TplContainersAgeWeeks,
    /// Age in months. Placeholder: `{count}`.
    TplContainersAgeMonths,
    /// Age in years. Placeholder: `{count}`.
    TplContainersAgeYears,
    /// Name of Containers mode in the status bar while no engine is shown.
    ContainersStatusMode,
    /// Status bar text while OTTY looks for engines.
    ContainersStatusSearching,
    /// Status bar text when no engine was found.
    ContainersStatusNoEngine,
    /// Status bar text when Docker is not running.
    ContainersStatusDockerStopped,
    /// Status bar text when the Podman socket is off.
    ContainersStatusPodmanOff,
    /// Status bar text when Docker and Podman are both off.
    ContainersStatusBothOff,
    /// Status bar text when the socket is denied. Placeholder: `{socket}`.
    TplContainersStatusNoAccess,
    /// Status bar text when the Docker socket is denied and Podman is off.
    /// Placeholder: `{socket}`.
    TplContainersStatusDockerNoAccessPodmanOff,
    /// Status bar text when a Podman socket is denied and Docker is off.
    /// Placeholder: `{socket}`.
    TplContainersStatusPodmanNoAccessDockerOff,
    /// Status bar count of one container.
    ContainersStatusOneContainer,
    /// Status bar count of containers. Placeholder: `{count}`.
    TplContainersStatusContainers,
    /// Status bar count of one image.
    ContainersStatusOneImage,
    /// Status bar count of images. Placeholder: `{count}`.
    TplContainersStatusImages,
    /// Status bar count of one volume.
    ContainersStatusOneVolume,
    /// Status bar count of volumes. Placeholder: `{count}`.
    TplContainersStatusVolumes,
    /// Status bar count of one network.
    ContainersStatusOneNetwork,
    /// Status bar count of networks. Placeholder: `{count}`.
    TplContainersStatusNetworks,
    /// Button and menu item that starts a stopped container.
    ContainersActionStart,
    /// Button and menu item that stops a container.
    ContainersActionStop,
    /// Button and menu item that restarts a container.
    ContainersActionRestart,
    /// Button and menu item that pauses a running container.
    ContainersActionPause,
    /// Button and menu item that lets a paused container run again.
    ContainersActionUnpause,
    /// Button and menu item that opens a shell in a running container.
    ContainersActionShell,
    /// Hint under a disabled Shell button: the `docker` command is missing.
    ContainersShellNeedsDocker,
    /// Hint under a disabled Shell button: the `podman` command is missing.
    ContainersShellNeedsPodman,
    /// Title of the terminal tab of a container shell. Placeholder: `{name}`.
    TplContainersShellTabTitle,
    /// Button, menu item and dialog action that removes a row.
    ContainersActionRemove,
    /// Button above the containers table that removes every stopped container.
    ContainersPruneStopped,
    /// Action of a prune confirmation dialog.
    ContainersPrune,
    /// Title of the remove container dialog. Placeholder: `{name}`.
    TplContainersRemoveContainerTitle,
    /// Text of the remove container dialog.
    ContainersRemoveContainerText,
    /// Switch in the remove dialog of a running container.
    ContainersStopItFirst,
    /// Title of the prune stopped containers dialog.
    ContainersPruneContainersTitle,
    /// Text of the prune stopped containers dialog. Placeholder: `{count}`.
    TplContainersPruneContainersText,
    /// Tooltip of the cross that hides a notice or a toast.
    NotificationDismiss,
    /// Notice when a container did not start. Placeholders: `{name}`, `{error}`.
    TplContainersStartFailed,
    /// Notice when a container did not stop. Placeholders: `{name}`, `{error}`.
    TplContainersStopFailed,
    /// Notice when a container did not restart. Placeholders: `{name}`, `{error}`.
    TplContainersRestartFailed,
    /// Notice when a container did not pause. Placeholders: `{name}`, `{error}`.
    TplContainersPauseFailed,
    /// Notice when a container did not unpause. Placeholders: `{name}`, `{error}`.
    TplContainersUnpauseFailed,
    /// Notice when a container was not removed. Placeholders: `{name}`, `{error}`.
    TplContainersRemoveContainerFailed,
    /// Notice when the stopped containers were not pruned. Placeholder: `{error}`.
    TplContainersPruneContainersFailed,
    /// Button and menu item that pulls the selected image again.
    ContainersActionPullAgain,
    /// Button above the images table that removes dangling images.
    ContainersPruneDangling,
    /// Button above the volumes and networks tables that removes unused ones.
    ContainersPruneUnused,
    /// Title of the remove image dialog. Placeholder: `{name}`.
    TplContainersRemoveImageTitle,
    /// Text of the remove image dialog.
    ContainersRemoveImageText,
    /// Switch in the remove image dialog when a container uses the image.
    ContainersForceRemoveImage,
    /// Title of the dialog that removes only a tag. Placeholder: `{name}`.
    TplContainersRemoveTagTitle,
    /// Remove dialog text: the image has other tags, so only this one goes.
    ContainersRemoveTagText,
    /// Remove dialog text, Force on: a container keeps the image, the tag goes.
    ContainersForceUntagsImageText,
    /// Remove dialog text, Force on: the image goes, its containers stay.
    ContainersForceDeletesImageText,
    /// Remove dialog text, Force on: the image and its containers go.
    ContainersForceDeletesContainersText,
    /// Remove dialog text, Force off: the engine will refuse.
    ContainersRemoveImageNeedsForceText,
    /// Remove dialog text: the engine refuses, and Force would not help.
    ContainersRemoveImageRefusedText,
    /// Switch in the prune volumes dialog: remove named unused volumes too.
    ContainersAlsoRemoveNamedVolumes,
    /// Title of the prune dangling images dialog.
    ContainersPruneImagesTitle,
    /// Text of the prune dangling images dialog. Placeholder: `{count}`.
    TplContainersPruneImagesText,
    /// Title of the remove volume dialog. Placeholder: `{name}`.
    TplContainersRemoveVolumeTitle,
    /// Text of the remove volume dialog.
    ContainersRemoveVolumeText,
    /// Title of the prune unused volumes dialog.
    ContainersPruneVolumesTitle,
    /// Text of the prune unused volumes dialog. Placeholder: `{count}`.
    TplContainersPruneVolumesText,
    /// Text of the prune unused volumes dialog while named volumes stay
    /// (switch off). Placeholder: `{count}`.
    TplContainersPruneAnonymousVolumesText,
    /// Title of the remove network dialog. Placeholder: `{name}`.
    TplContainersRemoveNetworkTitle,
    /// Text of the remove network dialog.
    ContainersRemoveNetworkText,
    /// Title of the prune unused networks dialog.
    ContainersPruneNetworksTitle,
    /// Text of the prune unused networks dialog. Placeholder: `{count}`.
    TplContainersPruneNetworksText,
    /// Hint under a disabled Remove of a volume a container uses. Placeholder: `{names}`.
    TplContainersVolumeInUseHint,
    /// Hint under a disabled Remove of a built-in network.
    ContainersBuiltInNetworkHint,
    /// Hint under a disabled Remove of a network containers are connected to.
    ContainersNetworkInUseHint,
    /// Label of the image field of the pull dialog.
    ContainersFieldImage,
    /// Label of the tag field of the pull dialog.
    ContainersFieldTag,
    /// Label of the name field of the create volume and create network dialogs.
    ContainersFieldName,
    /// Label of the driver field of the create volume and create network dialogs.
    ContainersFieldDriver,
    /// Placeholder of the image field of the pull dialog.
    ContainersImagePlaceholder,
    /// Placeholder of the name field of the create volume dialog.
    ContainersVolumeNamePlaceholder,
    /// Action button of the pull dialog.
    ContainersActionPull,
    /// Action button of the create volume and create network dialogs.
    ContainersActionCreate,
    /// Progress text of a running pull. Placeholder: `{image}`.
    TplContainersPulling,
    /// Bytes of a running pull. Placeholders: `{done}`, `{total}`.
    TplContainersPullAmount,
    /// Notice when a pull failed. Placeholders: `{name}`, `{error}`.
    TplContainersPullFailed,
    /// Notice when an image was not removed. Placeholders: `{name}`, `{error}`.
    TplContainersRemoveImageFailed,
    /// Notice when the dangling images were not pruned. Placeholder: `{error}`.
    TplContainersPruneImagesFailed,
    /// Notice when a volume was not created. Placeholder: `{error}`.
    TplContainersCreateVolumeFailed,
    /// Notice when a volume was not removed. Placeholders: `{name}`, `{error}`.
    TplContainersRemoveVolumeFailed,
    /// Notice when the unused volumes were not pruned. Placeholder: `{error}`.
    TplContainersPruneVolumesFailed,
    /// Notice when a network was not created. Placeholders: `{name}`, `{error}`.
    TplContainersCreateNetworkFailed,
    /// Notice when a network was not removed. Placeholders: `{name}`, `{error}`.
    TplContainersRemoveNetworkFailed,
    /// Notice when the unused networks were not pruned. Placeholder: `{error}`.
    TplContainersPruneNetworksFailed,
    // Containers mode inspector tabs.
    /// Inspector tab with the key facts of the selected row.
    ContainersTabOverview,
    /// Inspector tab with the logs of the selected container.
    ContainersTabLogs,
    /// Inspector tab with the engine's inspect JSON.
    ContainersTabInspect,
    /// Hint above the logs of a container.
    ContainersLogsHint,
    /// Label of the switch that keeps the logs at the newest line.
    ContainersLogsFollow,
    /// Line in the logs where lines were dropped while they waited for Follow.
    ContainersLogsSkippedOutput,
    /// Count of one log line waiting while Follow is off.
    ContainersLogsOneNewLine,
    /// Count of log lines waiting while Follow is off. Placeholder: `{count}`.
    TplContainersLogsNewLines,
    /// Notice when the logs could not be read. Placeholder: `{error}`.
    TplContainersLogsFailed,
    /// Notice when the inspect JSON could not be loaded. Placeholder: `{error}`.
    TplContainersInspectFailed,
    /// Title of the group of top-level values in the inspect tree.
    ContainersInspectGeneral,
    /// Label of the switch that shows empty values in the inspect tree.
    ContainersInspectShowEmpty,
    /// Label of the switch that shows the inspect JSON instead of the tree.
    ContainersInspectRawJson,
    /// Button that copies the inspect JSON.
    ContainersInspectCopyJson,

    // Run container form (spec Decision 18).
    /// "Run" action on an image and the action button of the Run form.
    ContainersActionRun,
    /// Line under the Run form title.
    ContainersRunSubtitle,
    /// Section title: network, restart and auto remove.
    ContainersRunSectionOptions,
    /// Label of the engine select.
    ContainersFieldEngine,
    /// Placeholder of the optional container name.
    ContainersRunNamePlaceholder,
    /// Switch: pull a missing image first.
    ContainersRunPullMissing,
    /// Placeholder of the command word input.
    ContainersRunCommandPlaceholder,
    /// Hint under the command while the image's command is not known.
    ContainersRunCommandHint,
    /// Hint under the command. Placeholder: `{command}`.
    TplContainersRunCommandHint,
    /// Column heading of the host ports.
    ContainersRunHostPort,
    /// Column heading of the container ports.
    ContainersRunContainerPort,
    /// Button that adds a port row.
    ContainersRunAddPort,
    /// Tooltip of the cross of a port row.
    ContainersRunRemovePort,
    /// Hint under the ports. Placeholder: `{ports}`.
    TplContainersRunImageListens,
    /// Placeholder of an environment value.
    ContainersRunEnvValue,
    /// Button that adds an environment variable.
    ContainersRunAddVariable,
    /// Column heading of the volume sources.
    ContainersRunVolumeSource,
    /// Column heading of the paths in the container.
    ContainersRunVolumeTarget,
    /// Switch of a volume row.
    ContainersRunReadOnly,
    /// Switch of a folder volume row on a computer with SELinux.
    ContainersRunRelabel,
    /// Hint beside the SELinux switch of a volume row.
    ContainersRunRelabelHint,
    /// Button that adds a volume row.
    ContainersRunAddVolume,
    /// Tooltip of the cross of a volume row.
    ContainersRunRemoveVolume,
    /// Label of the network select.
    ContainersFieldNetwork,
    /// Label of the restart policy.
    ContainersFieldRestart,
    /// Restart policy `no`.
    ContainersRestartNo,
    /// Restart policy `on-failure`.
    ContainersRestartOnFailure,
    /// Restart policy `unless-stopped`.
    ContainersRestartUnlessStopped,
    /// Restart policy `always`.
    ContainersRestartAlways,
    /// What the `no` policy does.
    ContainersRestartHintNo,
    /// What the `on-failure` policy does.
    ContainersRestartHintOnFailure,
    /// What the `unless-stopped` policy does.
    ContainersRestartHintUnlessStopped,
    /// What the `always` policy does.
    ContainersRestartHintAlways,
    /// Why the restart choices are disabled while auto remove is on.
    ContainersRestartHintAutoRemove,
    /// Switch: remove the container when it stops.
    ContainersRunAutoRemove,
    /// Error: the image is empty.
    ContainersRunErrImage,
    /// Error: a port is not 1 to 65535.
    ContainersRunErrPort,
    /// Error: a path in the container is not absolute.
    ContainersRunErrContainerPath,
    /// Error: a volume row has no source.
    ContainersRunErrSourceRequired,
    /// Error: a host folder does not exist.
    ContainersRunErrSourceMissing,
    /// Hint: the disk has not answered about a host folder for a while.
    ContainersRunErrSourceNotAnswering,
    /// Error: a source uses `~` and the home folder is not known.
    ContainersRunErrSourceHomeUnknown,
    /// Footer hint of a ready form. Placeholder: `{engine}`.
    TplContainersRunReady,
    /// Footer hint while some fields have errors.
    ContainersRunFixFields,
    /// Footer hint and image status while the image is looked up.
    ContainersRunCheckingImage,
    /// Footer hint while the disk has not answered about a folder source.
    ContainersRunWaitingForFolder,
    /// Label of the action button while the run goes on.
    ContainersRunStarting,
    /// Notice of a failed run. Placeholders: `{name}`, `{error}`.
    TplContainersRunFailed,
    /// Hint at the top of the image panel.
    ContainersRunPanelHint,
    /// Image status: on this computer. Placeholder: `{size}`.
    TplContainersImageHere,
    /// Image status: not on this computer.
    ContainersImageMissing,
    /// Image hint: missing, the pull switch is on.
    ContainersRunImagePulledFirst,
    /// Image hint: missing, the pull switch is off.
    ContainersRunImageNotPulled,
    /// Image panel field: exposed ports.
    ContainersFieldListensOn,
    /// Image panel field: the entrypoint.
    ContainersFieldEntrypoint,
    /// Image panel field: the working directory.
    ContainersFieldWorkdir,
    /// Image panel text while no image is typed.
    ContainersRunNoImage,
    /// Image panel notice of a failed lookup. Placeholder: `{error}`.
    TplContainersImageLookupFailed,
    /// Overview field: the mounts of a container.
    ContainersFieldMounts,
    /// Overview field: the Compose project.
    ContainersFieldCompose,
    /// Overview field: the command of a container.
    ContainersFieldCommand,
    /// Overview field: when the item was made.
    ContainersFieldCreated,
    /// Overview field: where a volume keeps its data.
    ContainersFieldMountpoint,
    /// Overview field: the gateway of a network.
    ContainersFieldGateway,
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
    /// Action that splits the focused pane to the right.
    PaletteSplitRight,
    /// Action that splits the focused pane down.
    PaletteSplitDown,
    /// Closing a session: the palette action and the tooltip of the close
    /// cross on a session row.
    CloseSession,
    /// Action that shows or hides the left panel.
    PaletteToggleLeftPanel,
    /// Action that shows or hides the right panel.
    PaletteToggleRightPanel,
    /// Action that switches to Terminal mode.
    PaletteSwitchToTerminalMode,
    /// Action that switches to Containers mode.
    PaletteSwitchToContainersMode,
    /// Palette entry that switches to Agents mode.
    PaletteSwitchToAgentsMode,

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
    /// Word beside the label of a field that may stay empty.
    FieldOptional,
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
    /// Tooltip of the cross on an environment variable row.
    ButtonRemoveEnv,
    /// Restore settings to the persisted baseline.
    ButtonReset,
    /// Connect with the secret typed in the SSH secret dialog.
    ButtonConnect,
    /// Tooltip of the close cross in the title of a popup, such as Settings.
    ButtonClose,

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
    /// Title of Settings: the popup, the gear tooltip and the palette entry.
    SettingsTitle,
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
    /// Notification when a terminal tab could not start. Placeholder:
    /// `{error}`.
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
    /// Status of a command block whose command ended with a code other
    /// than 0. Placeholder: `{code}`.
    TplBlockExitCode,
    /// Badge after the status of a folded block: how many lines of output
    /// it hides. Placeholder: `{count}`.
    TplBlockLinesHidden,
    /// Sticky header of a block whose start is above the pane: how many
    /// lines of the block are above it. Placeholder: `{count}`.
    TplBlockLinesAbove,

    // Notifications: errors told in a toast and kept in the list.
    /// Notification when the pinned modes could not be saved.
    /// Placeholder: `{error}`.
    TplPinnedModesSaveFailed,
    /// Title of the notifications list, the tooltip of its bell and its
    /// palette entry.
    NotificationsTitle,
    /// Tooltip of the broom button that empties the notifications list.
    NotificationsClear,
    /// Tooltip of the cross that removes one notification from the list.
    NotificationRemove,
    /// Line of the notifications list while it is empty.
    NotificationsEmpty,
    /// Age of a notification that came less than a minute ago.
    NotificationJustNow,
    /// Age of a notification in whole minutes. Placeholder: `{count}`.
    TplNotificationMinutesAgo,
    /// Age of a notification in whole hours. Placeholder: `{count}`.
    TplNotificationHoursAgo,
    /// Notification when a launch could not start.
    /// Placeholders: `{title}`, `{error}`.
    TplLaunchFailed,
    /// Notification when a pane could not be split because its new
    /// terminal did not start. Placeholder: `{error}`.
    TplSplitPaneFailed,
    /// Notification when a split was refused because a pane would get
    /// smaller than the minimum.
    NoRoomForPane,
    /// Notification when the launches could not be saved; the tree keeps
    /// the change and the save tries again. Placeholder: `{error}`.
    TplLaunchesSaveFailed,
    /// Notification when the settings could not be saved; the popup keeps
    /// the edits. Placeholder: `{error}`.
    TplSettingsSaveFailed,
    /// Notification when the settings file could not be read.
    /// Placeholder: `{error}`.
    TplSettingsReadFailed,
    /// Notification when the settings file is broken and the defaults are
    /// used instead. Placeholder: `{error}`.
    TplSettingsFileInvalid,
    /// Notification when a folder pressed in Files could not be opened.
    /// Placeholders: `{name}`, `{error}`.
    TplFolderOpenFailed,
    /// Notification when a link could not be opened in the browser.
    /// Placeholder: `{error}`.
    TplLinkOpenFailed,
    /// Notification at start when the launches file could not be read; the
    /// tree starts empty. Placeholder: `{error}`.
    TplLaunchesReadFailed,
    /// Notification at start when the modes file could not be read; the
    /// default modes are pinned. Placeholder: `{error}`.
    TplPinnedModesReadFailed,
    /// Title of the dialog that asks before a file or a folder in Files is
    /// deleted. Placeholder: `{name}`.
    TplFilesDeleteTitle,
    /// Notification when a file or a folder in Files could not be created,
    /// renamed, moved or deleted. Placeholders: `{name}`, `{error}`.
    TplFilesChangeFailed,
    // Agents mode: profiles, projects, sessions and the status bar.
    /// Title of the section of the profiles in the Agents mode left panel,
    /// "Profiles".
    AgentsSectionProfiles,
    /// Label of the projects in the Agents mode left panel.
    AgentsProjects,
    /// Button that reads the profiles and sessions again.
    AgentsRefresh,
    /// Shown while the profiles are read for the first time.
    AgentsLoading,
    /// Shown in the left panel when no session was found.
    AgentsNoProjects,
    /// State of a profile whose program was not found.
    AgentsStateProgramNotFound,
    /// State of a profile whose config folder does not exist.
    AgentsStateConfigDirNotFound,
    /// Note under a profile whose sessions could not be read. Placeholder:
    /// `{error}`, why each way of reading them failed.
    TplAgentsSessionsFailed,
    /// Word after a project whose folder no longer exists.
    AgentsProjectMissing,
    /// Title of the card shown when no agent was found.
    AgentsNoAgentTitle,
    /// Text of the card shown when no agent was found.
    AgentsNoAgentText,
    /// Title of the card shown while no project is selected.
    AgentsStartTitle,
    /// Text of the card shown while no project is selected.
    AgentsStartText,
    /// Button that continues a session in a terminal tab.
    AgentsContinue,
    /// Title of the column of the session titles in the sessions table.
    AgentsColumnTitle,
    /// Title of the column of the agents in the sessions table.
    AgentsColumnAgent,
    /// Title of the column of the profiles in the sessions table.
    AgentsColumnProfile,
    /// Title of the column of the last change in the sessions table.
    AgentsColumnUpdated,
    /// Placeholder of the search field over the sessions table.
    AgentsSessionsSearchPlaceholder,
    /// Tooltip of the button that reads the sessions again.
    AgentsRefreshSessions,
    /// Tooltip of the button that opens the project folder in a terminal tab.
    AgentsOpenFolderTerminal,
    /// Tooltip of the "⋯" button of the toolbar over the sessions table.
    AgentsMoreActions,
    /// Tooltip of the "⋯" button of a session row.
    AgentsSessionActions,
    /// Menu item that continues a session in a new pane of the terminal tab in front.
    AgentsContinueInSplit,
    /// Menu item that copies the command that continues a session.
    AgentsCopyResumeCommand,
    /// Menu item that copies the id of a session.
    AgentsCopySessionId,
    /// Line under the sessions table that says how to continue a session.
    AgentsSessionsHint,
    /// Shown in the sessions table when no session passes the search.
    AgentsSessionsNothingMatches,
    /// Tooltip of the cross of a tag of the sessions search.
    AgentsTagRemove,
    /// Button that removes every tag of the sessions search.
    AgentsTagsClearAll,
    /// Button that starts a new session in the project folder.
    AgentsNewSession,
    /// Notice of a project whose folder no longer exists.
    AgentsFolderMissing,
    /// Status bar count of one project.
    AgentsStatusOneProject,
    /// Status bar count of projects. Placeholder: `{count}`.
    TplAgentsStatusProjects,
    /// Status bar count of one agent session.
    AgentsStatusOneSession,
    /// Status bar count of agent sessions. Placeholder: `{count}`.
    TplAgentsStatusSessions,
    /// When a session changed less than a minute ago.
    AgentsAgeJustNow,
    /// When a session changed, in minutes. Placeholder: `{count}`.
    TplAgentsAgeMinutes,
    /// When a session changed, in hours. Placeholder: `{count}`.
    TplAgentsAgeHours,
    /// When a session changed, in days. Placeholder: `{count}`.
    TplAgentsAgeDays,
    /// When a session changed, in weeks. Placeholder: `{count}`.
    TplAgentsAgeWeeks,
    /// When a session changed, in months. Placeholder: `{count}`.
    TplAgentsAgeMonths,
    /// When a session changed, in years. Placeholder: `{count}`.
    TplAgentsAgeYears,
    /// Toast when `agents.json` could not be read. Placeholder: `{error}`.
    TplAgentsProfilesReadFailed,
    /// Toast when a config or session folder of a profile is there but
    /// could not be read. Placeholders: `{name}`, the profile; `{error}`,
    /// why.
    TplAgentsFolderUnreadable,
    /// Toast when every source of a profile's usage failed and one gave an
    /// answer OTTY does not expect: the agent probably changed. Placeholders:
    /// `{name}`, the profile; `{error}`, why each source failed.
    TplAgentsLimitsChanged,
    /// Toast when a program started to read a profile's usage stopped
    /// unexpectedly. Placeholders: `{name}`, the profile; `{error}`, which
    /// program and how.
    TplAgentsProgramStopped,
    /// Tooltip of the `+` of the Profiles section, which opens the
    /// dialog that adds a profile.
    AgentsAddProfile,
    /// Label of the agent of a profile.
    AgentsFieldAgent,
    /// Label of the name of a profile.
    AgentsFieldName,
    /// Label of the account a profile is logged in with.
    AgentsFieldAccount,
    /// Label of the plan of the account of a profile.
    AgentsFieldPlan,
    /// Label of the config folder of a profile.
    AgentsFieldConfigDir,
    /// Button of a profile that is not logged in: opens a tab where the agent logs in.
    AgentsLogIn,
    /// Item of the menu of a profile that opens the dialog that edits it.
    AgentsEdit,
    /// Note of a profile that is not logged in.
    AgentsNotLoggedInHint,
    /// Note of a profile whose program was not found.
    AgentsProgramNotFoundHint,
    /// Note of a profile whose config folder was not found.
    AgentsConfigDirNotFoundHint,
    /// Title of the dialog that adds a profile.
    AgentsAddProfileTitle,
    /// Title of the dialog that edits a profile.
    AgentsEditProfileTitle,
    /// Choice of a config folder the user types, "Type a path".
    AgentsFolderTyped,
    /// Choice of a new config folder OTTY creates.
    AgentsFolderCreate,
    /// Hint under the path of a new config folder.
    AgentsCreateFolderHint,
    /// Check of the name of a profile: it is empty.
    AgentsErrorEmptyName,
    /// Check of the name of a profile: another profile has it.
    AgentsErrorNameTaken,
    /// Check of the config folder of a profile: no path.
    AgentsErrorFolderEmpty,
    /// Check of the config folder of a profile: the path is not a full path.
    AgentsErrorFolderNotFull,
    /// Check of the config folder of a profile: another profile of the agent uses it.
    AgentsErrorFolderTaken,
    /// The profiles could not be saved. Placeholder: `{error}`.
    TplAgentsSaveFailed,
    /// Why a change of the profiles is not saved: `agents.json` could not be read.
    AgentsSavedFileUnread,
    /// Title of the dialog that asks before a profile is removed. Placeholder: `{name}`.
    TplAgentsRemoveTitle,
    /// Text of the dialog that asks before a profile is removed. Placeholder: `{path}`.
    TplAgentsRemoveText,
    /// Label of the credits of a profile's account.
    AgentsFieldCredits,
    /// The credits of an account that has no limit on them.
    AgentsCreditsUnlimited,
    /// The credits of an account whose plan tells no number of them.
    AgentsCreditsAvailable,
    /// Note under credits that have no number: the plan tells no balance.
    AgentsCreditsNoBalance,
    /// A limit window that starts again within the minute.
    AgentsResetsNow,
    /// When the limits shown were read, such as "5 min ago". Placeholder: `{time}`.
    TplAgentsLimitsLastRead,
    /// When limits read from a session file were recorded there, such as "30 min ago". Placeholder: `{time}`.
    TplAgentsLimitsRecorded,
    /// The limits were never read; the reasons follow. Placeholder: `{error}`.
    TplAgentsLimitsNotAvailable,
    /// The last read of the limits failed and older ones show. Placeholder: `{error}`.
    TplAgentsLimitsReadFailed,
    /// The limits could not be read because the login is missing or expired.
    AgentsLimitsLogInAgain,
    /// A number of tokens sent, read from the cache and received, such as "12.3K in · 1.2M cached · 4.5K out". Placeholders: `{input}`, `{cached}`, `{output}`.
    TplAgentsTokens,
    /// The tokens could not be counted; the reason follows. Placeholder: `{error}`.
    TplAgentsTokensFailed,
    /// Tooltip of the Refresh button of the Agents section: reads the profiles, their limits and sessions again.
    AgentsRefreshAll,
    /// Tooltip of the search button of the Projects section.
    AgentsSearchProjects,
    /// Tooltip of the chevron of New session in the Projects section: lists the profiles.
    AgentsChooseAgent,
    /// Placeholder of the search field of the projects.
    AgentsSearchPlaceholder,
    /// Tooltip of the cross that clears the search field of the projects.
    AgentsSearchClear,
    /// Note under the empty search field of the projects.
    AgentsSearchHint,
    /// Note under the search field while text is typed, such as "2 of 7 projects match". Placeholders: `{shown}`, `{total}`.
    TplAgentsProjectsMatch,
    /// Note under the empty search field while a filter is on, such as "4 of 7 projects shown". Placeholders: `{shown}`, `{total}`.
    TplAgentsProjectsShown,
    /// Tooltip of the funnel next to the search field of the projects, and the name of its menu.
    AgentsFilterProjects,
    /// Heading of the agents in the funnel menu of the projects.
    AgentsFilterAgent,
    /// Funnel menu item that shows the projects of every agent.
    AgentsFilterAllAgents,
    /// Heading of what to show in the funnel menu of the projects.
    AgentsFilterShow,
    /// Funnel menu item that shows only the projects used in the last 30 days.
    AgentsFilterRecent,
    /// Funnel menu item that shows only the projects whose folder is gone.
    AgentsFilterMissing,
    /// Funnel menu item that sorts the projects by name.
    AgentsFilterSortByName,
    /// Funnel menu item that turns every filter off.
    AgentsFilterClearAll,
    /// Chip of the agent filter of the projects, such as "Codex only". Placeholder: `{name}`.
    TplAgentsChipAgentOnly,
    /// Chip of the filter of the projects used in the last 30 days.
    AgentsChipRecent,
    /// Chip of the filter of the projects whose folder is gone.
    AgentsChipMissing,
    /// Chip of the sort of the projects by name.
    AgentsChipSortedByName,
    /// Tooltip of the cross of a filter chip of the projects.
    AgentsChipRemove,
    /// Short number of a subscription profile: the percent of its most used window, such as "42 %". Placeholder: `{count}`.
    TplAgentsMostUsed,
    /// Short number of a credits profile, such as "3 760 left". Placeholder: `{amount}`.
    TplAgentsCreditsLeft,
    /// Short number of an API key profile, such as "$12.40 today". Placeholder: `{amount}`.
    TplAgentsMoneyToday,
    /// Short number of a credits profile whose plan tells no number: the tokens spent today, such as "1.8M today". Placeholder: `{amount}`.
    TplAgentsTokensToday,
    /// State word of a profile row whose agent needs the user to log in.
    AgentsStateNeedsLogin,
    /// What an API key profile has where a plan would be: the provider's API, such as "Anthropic API". Placeholder: `{provider}`.
    TplAgentsProviderApi,
    /// How much of a usage window is used, such as "42 % used". Placeholder: `{count}`.
    TplAgentsPercentUsed,
    /// Name of the extra usage a subscription bought this month, shown like a usage window.
    AgentsExtraUsage,
    /// An amount of money of a limit, such as "$4.20 of $50". Placeholders: `{amount}`, `{limit}`.
    TplAgentsAmountOf,
    /// How many credits of the pool are used, such as "1 240 of 5 000 used". Placeholders: `{used}`, `{pool}`.
    TplAgentsCreditsUsedOf,
    /// Label of the numbers of today on the profile dashboard, and the last day under the bars per day.
    AgentsToday,
    /// Label of the numbers of the last 7 days on the profile dashboard.
    AgentsLastSevenDays,
    /// Label of the numbers of the last 30 days on the profile dashboard.
    AgentsLastThirtyDays,
    /// Label of the money of the current month on the dashboard of an API key profile.
    AgentsThisMonth,
    /// Title of the tokens block of the profile dashboard.
    AgentsTokensTitle,
    /// A number of tokens written short, such as "2.1M tokens". Placeholder: `{amount}`.
    TplAgentsTokensAmount,
    /// Tooltip of the session files at list prices among the sources of the money: the money is an estimate at the LiteLLM prices fetched at a time. Placeholder: `{time}`.
    TplAgentsMoneyEstimate,
    /// Last row of the Usage block of an API key profile: requests of this month whose model has no price. Placeholder: `{count}`.
    TplAgentsMoneyUnpriced,
    /// Title of the facts of a profile on its dashboard.
    AgentsProfileTitle,
    /// Label of the number of sessions of a profile and the projects they ran in.
    AgentsFieldSessions,
    /// Label of when a profile was last used and in which project.
    AgentsFieldLastUsed,
    /// The sessions of a profile and the projects they ran in, such as "79 in 14 projects". Placeholders: `{sessions}`, `{projects}`.
    TplAgentsSessionsInProjects,
    /// When a profile was last used and in which project, such as "3 min ago, in otty". Placeholders: `{time}`, `{project}`.
    TplAgentsLastUsedIn,
    /// Title of the section of the projects or the models that spent the most tokens in the last 30 days.
    AgentsTopTitle,
    /// Tab of the top section: the projects.
    AgentsTopByProject,
    /// Tab of the top section: the models.
    AgentsTopByModel,
    /// Line in place of the table of a tab of the top section that has nothing to list.
    AgentsTopEmpty,
    /// Column of the top projects table: the project.
    AgentsColumnProject,
    /// Column of the top projects table: the sessions of the project.
    AgentsColumnSessions,
    /// Column of the top tables: the tokens of the project or the model.
    AgentsColumnTokens,
    /// Column of the top models table: the model.
    AgentsColumnModel,
    /// Column of the top models table: the requests of the model.
    AgentsColumnRequests,
    /// Name of the tokens whose model the session files did not name.
    AgentsUnknownModel,
    /// Title of the list of the sources of a profile and their states.
    AgentsSourcesTitle,
    /// State of the source that answered.
    AgentsSourceAnswered,
    /// State of a source after the one that answered: it was not asked.
    AgentsSourceNotNeeded,
    /// State of a source that failed because the login is missing or expired.
    AgentsSourceLoginExpired,
    /// State of a source that can not be used here, such as a missing file.
    AgentsSourceNotAvailable,
    /// State of a source that failed for another reason.
    AgentsSourceFailed,
    /// Line under the typed path of a new profile: the folder must already exist.
    AgentsTypedFolderHint,
    /// Tooltip of the cost report among the sources of the money when it answered: the bill is the money of the whole organization, not only of this profile.
    AgentsMoneyFromBill,
    /// Why the cost report is not available: the profile has no admin key.
    AgentsNoAdminKey,
    /// Name of the row of the money of this month against the user's monthly budget, with its bar.
    AgentsBudgetTitle,
    /// Label of the admin key of an API key profile in the profile dialog: where the key is kept.
    AgentsFieldAdminKey,
    /// Choice of the admin key field: a file that holds the key.
    AgentsAdminKeyFile,
    /// Choice of the admin key field: an environment variable that holds the key.
    AgentsAdminKeyVariable,
    /// Placeholder of the path of the file that holds the admin key.
    AgentsAdminKeyFilePlaceholder,
    /// Placeholder of the name of the variable that holds the admin key.
    AgentsAdminKeyVariablePlaceholder,
    /// Line under the admin key field: what the key is for and that OTTY never saves it.
    AgentsAdminKeyHint,
    /// Label of the monthly budget of an API key profile in the profile dialog, in US dollars.
    AgentsFieldMonthlyBudget,
    /// Placeholder of the monthly budget: an amount in dollars.
    AgentsBudgetPlaceholder,
    /// Error under the monthly budget: the text is not an amount of dollars.
    AgentsErrorBudget,
    /// Error under the path of the admin key file: it is not a full path.
    AgentsErrorKeyPathNotFull,
    /// Error under the name of the admin key variable: it is not a variable name.
    AgentsErrorVariableName,
    /// Title of the billing block of a profile's dashboard: its usage windows, credits or money.
    AgentsUsage,
    /// Line under the Usage title: when the numbers shown were read, such as "2 min ago". Placeholder: `{time}`.
    TplAgentsUpdated,
    /// A reset within a day: how long until it and its clock time. Placeholders: `{duration}`, `{time}`.
    TplAgentsResetsInAt,
    /// A reset within six days: the day of the week and the clock time. Placeholders: `{day}`, `{time}`.
    TplAgentsResetsOnDay,
    /// A reset more than six days ahead: the date and the clock time. Placeholders: `{date}`, `{time}`.
    TplAgentsResetsOnDate,
    /// How long until a reset, in hours and minutes. Placeholders: `{hours}`, `{minutes}`.
    TplAgentsDurationHoursMinutes,
    /// How long until a reset, in whole hours. Placeholder: `{count}`.
    TplAgentsDurationHours,
    /// How long until a reset, in minutes. Placeholder: `{count}`.
    TplAgentsDurationMinutes,
    /// A date of a reset, such as "Nov 23". Placeholders: `{month}`, `{day}`.
    TplAgentsMonthDay,
    /// A clock time in UTC, written when the user's time zone is not known. Placeholder: `{time}`.
    TplAgentsTimeUtc,
    /// Monday, as it is written in the reset line "Resets on {day} at {time}".
    AgentsWeekdayMonday,
    /// Tuesday, as it is written in the reset line "Resets on {day} at {time}".
    AgentsWeekdayTuesday,
    /// Wednesday, as it is written in the reset line "Resets on {day} at {time}".
    AgentsWeekdayWednesday,
    /// Thursday, as it is written in the reset line "Resets on {day} at {time}".
    AgentsWeekdayThursday,
    /// Friday, as it is written in the reset line "Resets on {day} at {time}".
    AgentsWeekdayFriday,
    /// Saturday, as it is written in the reset line "Resets on {day} at {time}".
    AgentsWeekdaySaturday,
    /// Sunday, as it is written in the reset line "Resets on {day} at {time}".
    AgentsWeekdaySunday,
    /// The short name of the month Jan, as it is written in a reset date.
    AgentsMonthJan,
    /// The short name of the month Feb, as it is written in a reset date.
    AgentsMonthFeb,
    /// The short name of the month Mar, as it is written in a reset date.
    AgentsMonthMar,
    /// The short name of the month Apr, as it is written in a reset date.
    AgentsMonthApr,
    /// The short name of the month May, as it is written in a reset date.
    AgentsMonthMay,
    /// The short name of the month Jun, as it is written in a reset date.
    AgentsMonthJun,
    /// The short name of the month Jul, as it is written in a reset date.
    AgentsMonthJul,
    /// The short name of the month Aug, as it is written in a reset date.
    AgentsMonthAug,
    /// The short name of the month Sep, as it is written in a reset date.
    AgentsMonthSep,
    /// The short name of the month Oct, as it is written in a reset date.
    AgentsMonthOct,
    /// The short name of the month Nov, as it is written in a reset date.
    AgentsMonthNov,
    /// The short name of the month Dec, as it is written in a reset date.
    AgentsMonthDec,
    /// Name of the source that reads the agent's session files.
    AgentsSourceSessionFiles,
    /// Name of Claude Code's first source of limits: the usage request to Anthropic's server.
    AgentsSourceAnthropicUsageApi,
    /// Name of Claude Code's second source of limits: its `/usage` panel read in a hidden terminal.
    AgentsSourceClaudeUsageCommand,
    /// Name of Codex's first source of limits: the usage request to ChatGPT's server.
    AgentsSourceChatgptUsageApi,
    /// Name of Codex's second source of limits: `codex app-server`.
    AgentsSourceCodexAppServer,
    /// Name of Codex's third source of limits: the newest session file.
    AgentsSourceLatestCodexSession,
    /// Name of the first source of the money of a Claude Code API key profile: the cost report of the Anthropic Admin API, read with an admin key.
    AgentsSourceAnthropicCostReport,
    /// Name of the source of the money of an API key profile: its tokens at the list prices.
    AgentsSourceSessionFilesAtListPrices,
    /// Name of the first source of the list prices: the LiteLLM price table, fetched at most once a day.
    AgentsSourceLitellmPrices,
    /// Name of the second source of the list prices: the copy of the LiteLLM price table kept from the last good fetch.
    AgentsSourceKeptPrices,
    /// Tooltip of the source of the prices that answered: where the prices come from.
    AgentsPricesFrom,
    /// Column of "Where the numbers come from": which number of the dashboard the row gives, such as Limits or Money.
    AgentsColumnNumber,
    /// Column of "Where the numbers come from": the source of the number.
    AgentsColumnSource,
    /// Column of "Where the numbers come from": what the source did at the latest read.
    AgentsColumnState,
    /// Column of "Where the numbers come from": when the data of the source is from.
    AgentsColumnSourceUpdated,
    /// Number of the dashboard a source gives: the usage windows or the credits of a subscription.
    AgentsNumberLimits,
    /// Number of the dashboard a source gives: the tokens spent and the sessions.
    AgentsNumberTokens,
    /// Number of the dashboard a source gives: the money of an API key profile.
    AgentsNumberMoney,
    /// Number of the dashboard a source gives: the list prices the money is estimated at.
    AgentsNumberPrices,
    /// Tooltip of the name of Claude Code's first source of limits: what the direct usage request does.
    AgentsSourceTipAnthropicUsageApi,
    /// Tooltip of the name of Claude Code's second source of limits: what the hidden `/usage` run does and when.
    AgentsSourceTipClaudeUsageCommand,
    /// Tooltip of the name of Codex's first source of limits: what the direct usage request does.
    AgentsSourceTipChatgptUsageApi,
    /// Tooltip of the name of Codex's second source of limits: `codex app-server`.
    AgentsSourceTipCodexAppServer,
    /// Tooltip of the name of Codex's third source of limits: the record in its session files.
    AgentsSourceTipLatestCodexSession,
    /// Tooltip of the name of the source that reads the agent's session files.
    AgentsSourceTipSessionFiles,
    /// Tooltip of the name of the cost report among the sources of the money.
    AgentsSourceTipAnthropicCostReport,
    /// Tooltip of the name of the money estimate among the sources of the money.
    AgentsSourceTipSessionFilesAtListPrices,
    /// Tooltip of the name of the first source of the list prices: the LiteLLM price table.
    AgentsSourceTipLitellmPrices,
    /// Tooltip of the name of the second source of the list prices: the kept copy.
    AgentsSourceTipKeptPrices,
    /// Muted line under the money rows of an API key profile when no source gave prices: the rows show a dash.
    AgentsNoPrices,
    /// Notification: the LiteLLM price table or its kept copy is broken or not what OTTY expects. Placeholder: `{error}`.
    TplAgentsPricesUnreadable,
    /// Item of the menu of a session that opens its statistics.
    AgentsShowStatistics,
    /// Tooltip of the button of the project title row that opens the statistics of the project.
    AgentsProjectStatistics,
    /// Label of when a session started, in its statistics.
    AgentsStatsStarted,
    /// Label of when a session or a project was last active, in its statistics.
    AgentsStatsLastActivity,
    /// Label of when a project was first active, in its statistics.
    AgentsStatsFirstActivity,
    /// Label of how long a session ran, in its statistics.
    AgentsStatsLength,
    /// Label of how many messages the user wrote in a session.
    AgentsStatsMessages,
    /// Label of how many answers the model gave in a session.
    AgentsStatsAnswers,
    /// Title of the table of the tokens of each model in the statistics.
    AgentsStatsTokensByModel,
    /// Title of the table of the sessions and tokens of each profile in the statistics of a project.
    AgentsStatsByProfile,
    /// Title of the block of the usage windows at the start and the end of a Codex session.
    AgentsStatsLimits,
    /// Last row of the tokens table: every model together.
    AgentsStatsAllModels,
    /// Shown instead of the tokens table when the session files have no tokens.
    AgentsStatsNoTokens,
    /// Shown under the spinner while the statistics are read.
    AgentsStatsReading,
    /// Title of the column of the input tokens.
    AgentsColumnIn,
    /// Title of the column of the input tokens read from the cache.
    AgentsColumnCached,
    /// Title of the column of the output tokens.
    AgentsColumnOut,
    /// Title of the column of all the tokens.
    AgentsColumnTotal,
    /// Title of the column of the money an API key profile spent, and the label of its sum.
    AgentsColumnCost,
    /// The cost of a model the price table has no price for.
    AgentsNoPrice,
    /// The statistics could not be read; the reason follows. Placeholder: `{error}`.
    TplAgentsStatsFailed,
    /// How much of a usage window was used at the start and at the end of a session, such as "12 % → 31 %". Placeholders: `{start}`, `{end}`.
    TplAgentsLimitChange,
    /// A date with its year and clock time, such as "Nov 16, 2026, 09:00". Placeholders: `{date}` (the month and the day), `{year}`, `{time}`.
    TplAgentsDateTime,
    /// Palette entry that opens Agents mode with the New session dialog in the folder of the focused terminal tab.
    PaletteNewAgentSessionHere,
    /// Files context menu item that opens Agents mode with the New session dialog in the folder.
    FilesNewAgentSession,
    /// Label of the folder field of the New session dialog: the folder the agent starts in.
    AgentsFieldFolder,
    /// Label of the profile select of the New session dialog.
    AgentsFieldProfile,
    /// Button of the New session dialog that starts the agent in the folder.
    AgentsStart,
    /// Why the New session dialog can not start: the typed folder does not exist.
    AgentsErrorFolderNotFound,
    /// Why the New session dialog can not start: the typed path is not a full path.
    AgentsErrorFolderNotFullPath,
    /// Under the profile select of the New session dialog: no profile can start a session.
    AgentsNoProfileToStart,
}

impl Key {
    /// Every key in the catalogs, used by tests to verify full coverage.
    #[cfg(test)]
    pub(super) const ALL: [Self; 734] = [
        Self::MenuNewTerminal,
        Self::MenuSplitCurrentPane,
        Self::MenuSplitCurrentPaneDown,
        Self::MenuNewLaunch,
        Self::NewSessionMenu,
        Self::MenuNewFolder,
        Self::LaunchesMenu,
        Self::TerminalTabNotInitialized,
        Self::QuickLaunchEditorNotInitialized,
        Self::TerminalUnavailable,
        Self::ProcessTerminated,
        Self::BlockRunning,
        Self::BlockOneLineHidden,
        Self::BlockCollapse,
        Self::BlockExpand,
        Self::BlockOneLineAbove,
        Self::BlockGoToStart,
        Self::QuickLaunchCreateTabTitle,
        Self::CopySelection,
        Self::Paste,
        Self::CopyContent,
        Self::CopyCommand,
        Self::Collapse,
        Self::Expand,
        Self::RunAgain,
        Self::SplitDown,
        Self::SplitRight,
        Self::ClosePane,
        Self::ThisComputer,
        Self::WindowToggleMaximize,
        Self::WindowMinimize,
        Self::WindowRightPanel,
        Self::WindowRightPanelNotUsed,
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
        Self::SessionStateEditor,
        Self::NoOpenSessions,
        Self::EmptyTitle,
        Self::EmptyDescription,
        Self::EmptyNewTerminal,
        Self::EmptyNewLaunch,
        Self::LaunchesPanelTitle,
        Self::LaunchRunning,
        Self::NoActiveFolder,
        Self::FilesTitle,
        Self::FilesOpen,
        Self::FilesOpenInTerminal,
        Self::FilesNewFile,
        Self::FilesNameEmpty,
        Self::FilesNameInvalid,
        Self::FilesNameTaken,
        Self::FilesDeleteFileText,
        Self::FilesDeleteFolderText,
        Self::StatusPaneOne,
        Self::TplStatusPanes,
        Self::StatusSessionOne,
        Self::TplStatusSessions,
        Self::TplStatusBlockPlace,
        Self::ModeTerminal,
        Self::ModeContainers,
        Self::ModeAgents,
        Self::ModesAllModes,
        Self::ModesPinnedSection,
        Self::TplModesPinnedNote,
        Self::ModesOtherSection,
        Self::ModesPin,
        Self::ModesUnpin,
        Self::ModesUnpinAnotherFirst,
        Self::ModesNoOtherModes,
        Self::ContainersContainers,
        Self::ContainersImages,
        Self::ContainersVolumes,
        Self::ContainersNetworks,
        Self::ContainersRunContainer,
        Self::ContainersPullImage,
        Self::ContainersCreateVolume,
        Self::ContainersCreateNetwork,
        Self::ContainersRefresh,
        Self::ContainersMoreActions,
        Self::ContainersSearchPlaceholder,
        Self::ContainersTagRemove,
        Self::ContainersTagsClearAll,
        Self::TplContainersRunning,
        Self::ContainersListFailedShort,
        Self::ContainersLoading,
        Self::ContainersNoContainers,
        Self::ContainersNoImages,
        Self::ContainersNoVolumes,
        Self::ContainersNoNetworks,
        Self::ContainersNothingMatches,
        Self::TplContainersListFailed,
        Self::ContainersNotInstalledTitle,
        Self::ContainersNotInstalledText,
        Self::ContainersInstallDocker,
        Self::ContainersInstallPodman,
        Self::ContainersCheckAgain,
        Self::ContainersCopy,
        Self::ContainersHowToFix,
        Self::ContainersHowToFixDocker,
        Self::ContainersHowToFixPodman,
        Self::ContainersRunInTerminal,
        Self::ContainersDockerStoppedTitle,
        Self::ContainersDockerStoppedText,
        Self::ContainersDockerStoppedHint,
        Self::ContainersPodmanOffTitle,
        Self::ContainersPodmanOffText,
        Self::ContainersPodmanOffHint,
        Self::ContainersBothOffTitle,
        Self::ContainersBothOffText,
        Self::ContainersBothOffDockerLabel,
        Self::ContainersBothOffPodmanLabel,
        Self::ContainersDockerNoAccessTitle,
        Self::TplContainersDockerNoAccessText,
        Self::ContainersDockerNoAccessLabel,
        Self::ContainersDockerNoAccessHint,
        Self::ContainersPodmanNoAccessTitle,
        Self::TplContainersPodmanNoAccessText,
        Self::ContainersPodmanNoAccessLabel,
        Self::ContainersPodmanNoAccessHint,
        Self::ContainersDockerNoAccessPodmanOffTitle,
        Self::TplContainersDockerNoAccessPodmanOffText,
        Self::ContainersPodmanNoAccessDockerOffTitle,
        Self::TplContainersPodmanNoAccessDockerOffText,
        Self::ContainersPanelNotInstalled,
        Self::ContainersPanelDockerStopped,
        Self::ContainersPanelPodmanOff,
        Self::ContainersPanelBothOff,
        Self::ContainersPanelDockerNoAccess,
        Self::ContainersPanelPodmanNoAccess,
        Self::ContainersPanelDockerNoAccessPodmanOff,
        Self::ContainersPanelPodmanNoAccessDockerOff,
        Self::ContainersNothingToInspect,
        Self::TplContainersLookedFor,
        Self::ContainersLookedForCommands,
        Self::ContainersColumnName,
        Self::ContainersColumnStatus,
        Self::ContainersColumnImage,
        Self::ContainersColumnPorts,
        Self::ContainersColumnAge,
        Self::ContainersColumnRepository,
        Self::ContainersColumnTag,
        Self::ContainersColumnId,
        Self::ContainersColumnSize,
        Self::ContainersColumnUsedBy,
        Self::ContainersColumnDriver,
        Self::ContainersColumnScope,
        Self::ContainersColumnSubnet,
        Self::ContainersStateRunning,
        Self::ContainersStatePaused,
        Self::ContainersStateRestarting,
        Self::ContainersStateCreated,
        Self::ContainersStateExited,
        Self::TplContainersStateExitedCode,
        Self::ContainersStateDead,
        Self::ContainersStateRemoving,
        Self::ContainersStateStopping,
        Self::ContainersStateUnknown,
        Self::ContainersImageDangling,
        Self::ContainersUsedByOneContainer,
        Self::TplContainersUsedByContainers,
        Self::ContainersNotUsed,
        Self::TplContainersUsedBy,
        Self::ContainersBuiltInNetwork,
        Self::ContainersOneContainerConnected,
        Self::TplContainersContainersConnected,
        Self::ContainersAgeUnderMinute,
        Self::TplContainersAgeMinutes,
        Self::TplContainersAgeHours,
        Self::TplContainersAgeDays,
        Self::TplContainersAgeWeeks,
        Self::TplContainersAgeMonths,
        Self::TplContainersAgeYears,
        Self::ContainersStatusMode,
        Self::ContainersStatusSearching,
        Self::ContainersStatusNoEngine,
        Self::ContainersStatusDockerStopped,
        Self::ContainersStatusPodmanOff,
        Self::ContainersStatusBothOff,
        Self::TplContainersStatusNoAccess,
        Self::TplContainersStatusDockerNoAccessPodmanOff,
        Self::TplContainersStatusPodmanNoAccessDockerOff,
        Self::ContainersStatusOneContainer,
        Self::TplContainersStatusContainers,
        Self::ContainersStatusOneImage,
        Self::TplContainersStatusImages,
        Self::ContainersStatusOneVolume,
        Self::TplContainersStatusVolumes,
        Self::ContainersStatusOneNetwork,
        Self::TplContainersStatusNetworks,
        Self::ContainersActionStart,
        Self::ContainersActionStop,
        Self::ContainersActionRestart,
        Self::ContainersActionPause,
        Self::ContainersActionUnpause,
        Self::ContainersActionShell,
        Self::ContainersShellNeedsDocker,
        Self::ContainersShellNeedsPodman,
        Self::TplContainersShellTabTitle,
        Self::ContainersActionRemove,
        Self::ContainersPruneStopped,
        Self::ContainersPrune,
        Self::TplContainersRemoveContainerTitle,
        Self::ContainersRemoveContainerText,
        Self::ContainersStopItFirst,
        Self::ContainersPruneContainersTitle,
        Self::TplContainersPruneContainersText,
        Self::NotificationDismiss,
        Self::TplContainersStartFailed,
        Self::TplContainersStopFailed,
        Self::TplContainersRestartFailed,
        Self::TplContainersPauseFailed,
        Self::TplContainersUnpauseFailed,
        Self::TplContainersRemoveContainerFailed,
        Self::TplContainersPruneContainersFailed,
        Self::ContainersActionPullAgain,
        Self::ContainersPruneDangling,
        Self::ContainersPruneUnused,
        Self::TplContainersRemoveImageTitle,
        Self::ContainersRemoveImageText,
        Self::ContainersForceRemoveImage,
        Self::TplContainersRemoveTagTitle,
        Self::ContainersRemoveTagText,
        Self::ContainersForceUntagsImageText,
        Self::ContainersForceDeletesImageText,
        Self::ContainersForceDeletesContainersText,
        Self::ContainersRemoveImageNeedsForceText,
        Self::ContainersRemoveImageRefusedText,
        Self::ContainersAlsoRemoveNamedVolumes,
        Self::ContainersPruneImagesTitle,
        Self::TplContainersPruneImagesText,
        Self::TplContainersRemoveVolumeTitle,
        Self::ContainersRemoveVolumeText,
        Self::ContainersPruneVolumesTitle,
        Self::TplContainersPruneVolumesText,
        Self::TplContainersPruneAnonymousVolumesText,
        Self::TplContainersRemoveNetworkTitle,
        Self::ContainersRemoveNetworkText,
        Self::ContainersPruneNetworksTitle,
        Self::TplContainersPruneNetworksText,
        Self::TplContainersVolumeInUseHint,
        Self::ContainersBuiltInNetworkHint,
        Self::ContainersNetworkInUseHint,
        Self::ContainersFieldImage,
        Self::ContainersFieldTag,
        Self::ContainersFieldName,
        Self::ContainersFieldDriver,
        Self::ContainersImagePlaceholder,
        Self::ContainersVolumeNamePlaceholder,
        Self::ContainersActionPull,
        Self::ContainersActionCreate,
        Self::TplContainersPulling,
        Self::TplContainersPullAmount,
        Self::TplContainersPullFailed,
        Self::TplContainersRemoveImageFailed,
        Self::TplContainersPruneImagesFailed,
        Self::TplContainersCreateVolumeFailed,
        Self::TplContainersRemoveVolumeFailed,
        Self::TplContainersPruneVolumesFailed,
        Self::TplContainersCreateNetworkFailed,
        Self::TplContainersRemoveNetworkFailed,
        Self::TplContainersPruneNetworksFailed,
        Self::ContainersTabOverview,
        Self::ContainersTabLogs,
        Self::ContainersTabInspect,
        Self::ContainersLogsHint,
        Self::ContainersLogsFollow,
        Self::ContainersLogsSkippedOutput,
        Self::ContainersLogsOneNewLine,
        Self::TplContainersLogsNewLines,
        Self::TplContainersLogsFailed,
        Self::TplContainersInspectFailed,
        Self::ContainersInspectGeneral,
        Self::ContainersInspectShowEmpty,
        Self::ContainersInspectRawJson,
        Self::ContainersInspectCopyJson,
        Self::ContainersActionRun,
        Self::ContainersRunSubtitle,
        Self::ContainersRunSectionOptions,
        Self::ContainersFieldEngine,
        Self::ContainersRunNamePlaceholder,
        Self::ContainersRunPullMissing,
        Self::ContainersRunCommandPlaceholder,
        Self::ContainersRunCommandHint,
        Self::TplContainersRunCommandHint,
        Self::ContainersRunHostPort,
        Self::ContainersRunContainerPort,
        Self::ContainersRunAddPort,
        Self::ContainersRunRemovePort,
        Self::TplContainersRunImageListens,
        Self::ContainersRunEnvValue,
        Self::ContainersRunAddVariable,
        Self::ContainersRunVolumeSource,
        Self::ContainersRunVolumeTarget,
        Self::ContainersRunReadOnly,
        Self::ContainersRunRelabel,
        Self::ContainersRunRelabelHint,
        Self::ContainersRunAddVolume,
        Self::ContainersRunRemoveVolume,
        Self::ContainersFieldNetwork,
        Self::ContainersFieldRestart,
        Self::ContainersRestartNo,
        Self::ContainersRestartOnFailure,
        Self::ContainersRestartUnlessStopped,
        Self::ContainersRestartAlways,
        Self::ContainersRestartHintNo,
        Self::ContainersRestartHintOnFailure,
        Self::ContainersRestartHintUnlessStopped,
        Self::ContainersRestartHintAlways,
        Self::ContainersRestartHintAutoRemove,
        Self::ContainersRunAutoRemove,
        Self::ContainersRunErrImage,
        Self::ContainersRunErrPort,
        Self::ContainersRunErrContainerPath,
        Self::ContainersRunErrSourceRequired,
        Self::ContainersRunErrSourceMissing,
        Self::ContainersRunErrSourceNotAnswering,
        Self::ContainersRunErrSourceHomeUnknown,
        Self::TplContainersRunReady,
        Self::ContainersRunFixFields,
        Self::ContainersRunCheckingImage,
        Self::ContainersRunWaitingForFolder,
        Self::ContainersRunStarting,
        Self::TplContainersRunFailed,
        Self::ContainersRunPanelHint,
        Self::TplContainersImageHere,
        Self::ContainersImageMissing,
        Self::ContainersRunImagePulledFirst,
        Self::ContainersRunImageNotPulled,
        Self::ContainersFieldListensOn,
        Self::ContainersFieldEntrypoint,
        Self::ContainersFieldWorkdir,
        Self::ContainersRunNoImage,
        Self::TplContainersImageLookupFailed,
        Self::ContainersFieldMounts,
        Self::ContainersFieldCompose,
        Self::ContainersFieldCommand,
        Self::ContainersFieldCreated,
        Self::ContainersFieldMountpoint,
        Self::ContainersFieldGateway,
        Self::TitleSearchNoSessions,
        Self::PalettePlaceholder,
        Self::PaletteNoMatches,
        Self::PaletteHintMove,
        Self::PaletteHintOpen,
        Self::PaletteGroupActions,
        Self::PaletteNewTerminal,
        Self::PaletteNewLaunch,
        Self::PaletteNewFolder,
        Self::PaletteSplitRight,
        Self::PaletteSplitDown,
        Self::CloseSession,
        Self::PaletteToggleLeftPanel,
        Self::PaletteToggleRightPanel,
        Self::PaletteSwitchToTerminalMode,
        Self::PaletteSwitchToContainersMode,
        Self::PaletteSwitchToAgentsMode,
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
        Self::FieldOptional,
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
        Self::ButtonRemoveEnv,
        Self::ButtonReset,
        Self::ButtonConnect,
        Self::ButtonClose,
        Self::SecretPromptPassphraseTitle,
        Self::SecretPromptPasswordTitle,
        Self::SecretPromptWrongPassphrase,
        Self::SecretPromptPasswordRejected,
        Self::SecretPromptPassphraseLabel,
        Self::SecretPromptPasswordLabel,
        Self::SettingsTitle,
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
        Self::TplBlockExitCode,
        Self::TplBlockLinesHidden,
        Self::TplBlockLinesAbove,
        Self::TplPinnedModesSaveFailed,
        Self::NotificationsTitle,
        Self::NotificationsClear,
        Self::NotificationRemove,
        Self::NotificationsEmpty,
        Self::NotificationJustNow,
        Self::TplNotificationMinutesAgo,
        Self::TplNotificationHoursAgo,
        Self::TplLaunchFailed,
        Self::TplSplitPaneFailed,
        Self::NoRoomForPane,
        Self::TplLaunchesSaveFailed,
        Self::TplSettingsSaveFailed,
        Self::TplSettingsReadFailed,
        Self::TplSettingsFileInvalid,
        Self::TplFolderOpenFailed,
        Self::TplLinkOpenFailed,
        Self::TplLaunchesReadFailed,
        Self::TplPinnedModesReadFailed,
        Self::TplFilesDeleteTitle,
        Self::TplFilesChangeFailed,
        Self::AgentsSectionProfiles,
        Self::AgentsProjects,
        Self::AgentsRefresh,
        Self::AgentsLoading,
        Self::AgentsNoProjects,
        Self::AgentsStateProgramNotFound,
        Self::AgentsStateConfigDirNotFound,
        Self::TplAgentsSessionsFailed,
        Self::AgentsProjectMissing,
        Self::AgentsNoAgentTitle,
        Self::AgentsNoAgentText,
        Self::AgentsStartTitle,
        Self::AgentsStartText,
        Self::AgentsContinue,
        Self::AgentsColumnTitle,
        Self::AgentsColumnAgent,
        Self::AgentsColumnProfile,
        Self::AgentsColumnUpdated,
        Self::AgentsSessionsSearchPlaceholder,
        Self::AgentsRefreshSessions,
        Self::AgentsOpenFolderTerminal,
        Self::AgentsMoreActions,
        Self::AgentsSessionActions,
        Self::AgentsContinueInSplit,
        Self::AgentsCopyResumeCommand,
        Self::AgentsCopySessionId,
        Self::AgentsSessionsHint,
        Self::AgentsSessionsNothingMatches,
        Self::AgentsTagRemove,
        Self::AgentsTagsClearAll,
        Self::AgentsNewSession,
        Self::AgentsFolderMissing,
        Self::AgentsStatusOneProject,
        Self::TplAgentsStatusProjects,
        Self::AgentsStatusOneSession,
        Self::TplAgentsStatusSessions,
        Self::AgentsAgeJustNow,
        Self::TplAgentsAgeMinutes,
        Self::TplAgentsAgeHours,
        Self::TplAgentsAgeDays,
        Self::TplAgentsAgeWeeks,
        Self::TplAgentsAgeMonths,
        Self::TplAgentsAgeYears,
        Self::TplAgentsProfilesReadFailed,
        Self::TplAgentsFolderUnreadable,
        Self::TplAgentsLimitsChanged,
        Self::TplAgentsProgramStopped,
        Self::AgentsAddProfile,
        Self::AgentsFieldAgent,
        Self::AgentsFieldName,
        Self::AgentsFieldAccount,
        Self::AgentsFieldPlan,
        Self::AgentsFieldConfigDir,
        Self::AgentsLogIn,
        Self::AgentsEdit,
        Self::AgentsNotLoggedInHint,
        Self::AgentsProgramNotFoundHint,
        Self::AgentsConfigDirNotFoundHint,
        Self::AgentsAddProfileTitle,
        Self::AgentsEditProfileTitle,
        Self::AgentsFolderTyped,
        Self::AgentsFolderCreate,
        Self::AgentsCreateFolderHint,
        Self::AgentsErrorEmptyName,
        Self::AgentsErrorNameTaken,
        Self::AgentsErrorFolderEmpty,
        Self::AgentsErrorFolderNotFull,
        Self::AgentsErrorFolderTaken,
        Self::TplAgentsSaveFailed,
        Self::AgentsSavedFileUnread,
        Self::TplAgentsRemoveTitle,
        Self::TplAgentsRemoveText,
        Self::AgentsFieldCredits,
        Self::AgentsCreditsUnlimited,
        Self::AgentsCreditsAvailable,
        Self::AgentsCreditsNoBalance,
        Self::AgentsResetsNow,
        Self::TplAgentsLimitsLastRead,
        Self::TplAgentsLimitsRecorded,
        Self::TplAgentsLimitsNotAvailable,
        Self::TplAgentsLimitsReadFailed,
        Self::AgentsLimitsLogInAgain,
        Self::TplAgentsTokens,
        Self::TplAgentsTokensFailed,
        Self::AgentsRefreshAll,
        Self::AgentsSearchProjects,
        Self::AgentsChooseAgent,
        Self::AgentsSearchPlaceholder,
        Self::AgentsSearchClear,
        Self::AgentsSearchHint,
        Self::TplAgentsProjectsMatch,
        Self::TplAgentsProjectsShown,
        Self::AgentsFilterProjects,
        Self::AgentsFilterAgent,
        Self::AgentsFilterAllAgents,
        Self::AgentsFilterShow,
        Self::AgentsFilterRecent,
        Self::AgentsFilterMissing,
        Self::AgentsFilterSortByName,
        Self::AgentsFilterClearAll,
        Self::TplAgentsChipAgentOnly,
        Self::AgentsChipRecent,
        Self::AgentsChipMissing,
        Self::AgentsChipSortedByName,
        Self::AgentsChipRemove,
        Self::TplAgentsMostUsed,
        Self::TplAgentsCreditsLeft,
        Self::TplAgentsMoneyToday,
        Self::TplAgentsTokensToday,
        Self::AgentsStateNeedsLogin,
        Self::TplAgentsProviderApi,
        Self::TplAgentsPercentUsed,
        Self::AgentsExtraUsage,
        Self::TplAgentsAmountOf,
        Self::TplAgentsCreditsUsedOf,
        Self::AgentsToday,
        Self::AgentsLastSevenDays,
        Self::AgentsLastThirtyDays,
        Self::AgentsThisMonth,
        Self::AgentsTokensTitle,
        Self::TplAgentsTokensAmount,
        Self::TplAgentsMoneyEstimate,
        Self::TplAgentsMoneyUnpriced,
        Self::AgentsProfileTitle,
        Self::AgentsFieldSessions,
        Self::AgentsFieldLastUsed,
        Self::TplAgentsSessionsInProjects,
        Self::TplAgentsLastUsedIn,
        Self::AgentsTopTitle,
        Self::AgentsTopByProject,
        Self::AgentsTopByModel,
        Self::AgentsTopEmpty,
        Self::AgentsColumnProject,
        Self::AgentsColumnSessions,
        Self::AgentsColumnTokens,
        Self::AgentsColumnModel,
        Self::AgentsColumnRequests,
        Self::AgentsUnknownModel,
        Self::AgentsSourcesTitle,
        Self::AgentsSourceAnswered,
        Self::AgentsSourceNotNeeded,
        Self::AgentsSourceLoginExpired,
        Self::AgentsSourceNotAvailable,
        Self::AgentsSourceFailed,
        Self::AgentsTypedFolderHint,
        Self::AgentsMoneyFromBill,
        Self::AgentsNoAdminKey,
        Self::AgentsBudgetTitle,
        Self::AgentsFieldAdminKey,
        Self::AgentsAdminKeyFile,
        Self::AgentsAdminKeyVariable,
        Self::AgentsAdminKeyFilePlaceholder,
        Self::AgentsAdminKeyVariablePlaceholder,
        Self::AgentsAdminKeyHint,
        Self::AgentsFieldMonthlyBudget,
        Self::AgentsBudgetPlaceholder,
        Self::AgentsErrorBudget,
        Self::AgentsErrorKeyPathNotFull,
        Self::AgentsErrorVariableName,
        Self::AgentsUsage,
        Self::TplAgentsUpdated,
        Self::TplAgentsResetsInAt,
        Self::TplAgentsResetsOnDay,
        Self::TplAgentsResetsOnDate,
        Self::TplAgentsDurationHoursMinutes,
        Self::TplAgentsDurationHours,
        Self::TplAgentsDurationMinutes,
        Self::TplAgentsMonthDay,
        Self::TplAgentsTimeUtc,
        Self::AgentsWeekdayMonday,
        Self::AgentsWeekdayTuesday,
        Self::AgentsWeekdayWednesday,
        Self::AgentsWeekdayThursday,
        Self::AgentsWeekdayFriday,
        Self::AgentsWeekdaySaturday,
        Self::AgentsWeekdaySunday,
        Self::AgentsMonthJan,
        Self::AgentsMonthFeb,
        Self::AgentsMonthMar,
        Self::AgentsMonthApr,
        Self::AgentsMonthMay,
        Self::AgentsMonthJun,
        Self::AgentsMonthJul,
        Self::AgentsMonthAug,
        Self::AgentsMonthSep,
        Self::AgentsMonthOct,
        Self::AgentsMonthNov,
        Self::AgentsMonthDec,
        Self::AgentsSourceSessionFiles,
        Self::AgentsSourceAnthropicUsageApi,
        Self::AgentsSourceClaudeUsageCommand,
        Self::AgentsSourceChatgptUsageApi,
        Self::AgentsSourceCodexAppServer,
        Self::AgentsSourceLatestCodexSession,
        Self::AgentsSourceAnthropicCostReport,
        Self::AgentsSourceSessionFilesAtListPrices,
        Self::AgentsSourceLitellmPrices,
        Self::AgentsSourceKeptPrices,
        Self::AgentsPricesFrom,
        Self::AgentsColumnNumber,
        Self::AgentsColumnSource,
        Self::AgentsColumnState,
        Self::AgentsColumnSourceUpdated,
        Self::AgentsNumberLimits,
        Self::AgentsNumberTokens,
        Self::AgentsNumberMoney,
        Self::AgentsNumberPrices,
        Self::AgentsSourceTipAnthropicUsageApi,
        Self::AgentsSourceTipClaudeUsageCommand,
        Self::AgentsSourceTipChatgptUsageApi,
        Self::AgentsSourceTipCodexAppServer,
        Self::AgentsSourceTipLatestCodexSession,
        Self::AgentsSourceTipSessionFiles,
        Self::AgentsSourceTipAnthropicCostReport,
        Self::AgentsSourceTipSessionFilesAtListPrices,
        Self::AgentsSourceTipLitellmPrices,
        Self::AgentsSourceTipKeptPrices,
        Self::AgentsNoPrices,
        Self::TplAgentsPricesUnreadable,
        Self::AgentsShowStatistics,
        Self::AgentsProjectStatistics,
        Self::AgentsStatsStarted,
        Self::AgentsStatsLastActivity,
        Self::AgentsStatsFirstActivity,
        Self::AgentsStatsLength,
        Self::AgentsStatsMessages,
        Self::AgentsStatsAnswers,
        Self::AgentsStatsTokensByModel,
        Self::AgentsStatsByProfile,
        Self::AgentsStatsLimits,
        Self::AgentsStatsAllModels,
        Self::AgentsStatsNoTokens,
        Self::AgentsStatsReading,
        Self::AgentsColumnIn,
        Self::AgentsColumnCached,
        Self::AgentsColumnOut,
        Self::AgentsColumnTotal,
        Self::AgentsColumnCost,
        Self::AgentsNoPrice,
        Self::TplAgentsStatsFailed,
        Self::TplAgentsLimitChange,
        Self::TplAgentsDateTime,
        Self::PaletteNewAgentSessionHere,
        Self::FilesNewAgentSession,
        Self::AgentsFieldFolder,
        Self::AgentsFieldProfile,
        Self::AgentsStart,
        Self::AgentsErrorFolderNotFound,
        Self::AgentsErrorFolderNotFullPath,
        Self::AgentsNoProfileToStart,
    ];
}
