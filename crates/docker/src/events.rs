//! Which list an engine event changes, read from the engine's `/events`
//! messages.

use std::collections::HashMap;

use bollard::models::{EventMessage, EventMessageTypeEnum};

/// The object types the `/events` request asks for.
const WATCHED_TYPES: [&str; 4] = ["container", "image", "volume", "network"];
/// Container actions of someone using a container (a shell, `docker cp`,
/// `docker top`): the container list does not change. `exec_create` and
/// `exec_start` carry the command after a colon, so these are prefixes.
const CONTAINER_USE_ACTIONS: [&str; 8] = [
    "exec_",
    "attach",
    "detach",
    "resize",
    "top",
    "archive-path",
    "extract-to-dir",
    "export",
];
/// Network actions that attach or detach a container: what changes is the
/// container's network list, and "used by" of networks follows from it.
const NETWORK_ATTACH_ACTIONS: [&str; 2] = ["connect", "disconnect"];
/// The container action of a new run: a start or a restart.
const CONTAINER_START_ACTION: &str = "start";
/// Volume actions of a container starting or stopping, which already
/// reloads the container list (and with it "used by" of volumes).
const VOLUME_MOUNT_ACTIONS: [&str; 2] = ["mount", "unmount"];

/// One of the four lists of an engine that an event can change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ListKind {
    /// The container list.
    Containers,
    /// The image list.
    Images,
    /// The volume list.
    Volumes,
    /// The network list.
    Networks,
}

impl ListKind {
    /// Every list, in the order the debounce keeps them.
    pub(crate) const ALL: [Self; 4] = [
        Self::Containers,
        Self::Images,
        Self::Volumes,
        Self::Networks,
    ];

    /// The list an event changes; `None` for events that change no list.
    ///
    /// The object type decides, with three exceptions: using a container
    /// changes nothing, a container joining or leaving a network changes
    /// the container list, and a volume mount changes nothing on its own.
    pub(crate) fn from_event(message: &EventMessage) -> Option<Self> {
        let action = message.action.as_deref().unwrap_or_default();

        match message.typ? {
            EventMessageTypeEnum::CONTAINER => {
                let is_use = CONTAINER_USE_ACTIONS
                    .iter()
                    .any(|prefix| action.starts_with(prefix));
                (!is_use).then_some(Self::Containers)
            },
            EventMessageTypeEnum::IMAGE => Some(Self::Images),
            EventMessageTypeEnum::VOLUME => {
                let is_mount = VOLUME_MOUNT_ACTIONS.contains(&action);
                (!is_mount).then_some(Self::Volumes)
            },
            EventMessageTypeEnum::NETWORK => {
                if NETWORK_ATTACH_ACTIONS.contains(&action) {
                    Some(Self::Containers)
                } else {
                    Some(Self::Networks)
                }
            },
            _ => None,
        }
    }
}

/// What one engine event tells the watch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EngineChange {
    /// The list to reload.
    pub(crate) list: ListKind,
    /// The id of the container this event started, for a `start` event.
    pub(crate) started_container: Option<String>,
}

impl EngineChange {
    /// What `message` changes; `None` for events that change no list.
    pub(crate) fn from_event(message: &EventMessage) -> Option<Self> {
        Some(Self {
            list: ListKind::from_event(message)?,
            started_container: started_container(message),
        })
    }
}

/// The `filters` of the `/events` request: only the four object types OTTY
/// lists. Other types (and Podman's `pod` events, which bollard cannot read)
/// never reach OTTY.
pub(crate) fn event_filters() -> HashMap<String, Vec<String>> {
    let types = WATCHED_TYPES.iter().map(|kind| kind.to_string()).collect();

    HashMap::from([(String::from("type"), types)])
}

/// The id of the container a `start` event started. Docker and Podman both
/// send one `start` per run, also for a restart.
fn started_container(message: &EventMessage) -> Option<String> {
    let is_container = message.typ == Some(EventMessageTypeEnum::CONTAINER);
    if !is_container
        || message.action.as_deref() != Some(CONTAINER_START_ACTION)
    {
        return None;
    }

    message.actor.as_ref()?.id.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOCKER_EVENTS: &str =
        include_str!("../tests/fixtures/docker-events.jsonl");

    fn parse(json: &str) -> EventMessage {
        serde_json::from_str(json).expect("an /events message")
    }

    fn event(kind: &str, action: &str) -> EventMessage {
        parse(&format!(r#"{{"Type":"{kind}","Action":"{action}"}}"#))
    }

    #[test]
    fn captured_docker_events_change_their_lists() {
        let changes: Vec<(String, Option<ListKind>)> = DOCKER_EVENTS
            .lines()
            .map(parse)
            .map(|message| {
                let action = message.action.clone().unwrap_or_default();
                (action, ListKind::from_event(&message))
            })
            .collect();

        let expected = [
            ("pull", Some(ListKind::Images)),
            ("create", Some(ListKind::Containers)),
            ("attach", None),
            ("connect", Some(ListKind::Containers)),
            ("start", Some(ListKind::Containers)),
            ("disconnect", Some(ListKind::Containers)),
            ("die", Some(ListKind::Containers)),
            ("destroy", Some(ListKind::Containers)),
            ("create", Some(ListKind::Volumes)),
            ("create", Some(ListKind::Networks)),
            ("destroy", Some(ListKind::Volumes)),
            ("destroy", Some(ListKind::Networks)),
            ("untag", Some(ListKind::Images)),
            ("delete", Some(ListKind::Images)),
        ];
        let expected: Vec<(String, Option<ListKind>)> = expected
            .iter()
            .map(|(action, list)| (String::from(*action), *list))
            .collect();
        assert_eq!(changes, expected);
    }

    #[test]
    fn using_a_container_from_a_terminal_changes_no_list() {
        let actions = [
            "exec_create: sh",
            "exec_start: sh",
            "exec_die",
            "exec_detach",
            "attach",
            "detach",
            "resize",
            "top",
            "archive-path",
            "extract-to-dir",
            "export",
        ];

        for action in actions {
            let message = event("container", action);

            assert_eq!(ListKind::from_event(&message), None, "{action}");
        }
    }

    #[test]
    fn a_health_change_changes_the_container_list() {
        let message = event("container", "health_status: healthy");

        assert_eq!(ListKind::from_event(&message), Some(ListKind::Containers));
    }

    #[test]
    fn a_volume_mount_changes_no_list_on_its_own() {
        for action in ["mount", "unmount"] {
            let message = event("volume", action);

            assert_eq!(ListKind::from_event(&message), None, "{action}");
        }
    }

    #[test]
    fn other_object_types_change_no_list() {
        for kind in ["daemon", "plugin", "builder", "service", "node", ""] {
            let message = event(kind, "reload");

            assert_eq!(ListKind::from_event(&message), None, "{kind}");
        }
        assert_eq!(ListKind::from_event(&parse("{}")), None);
    }

    #[test]
    fn an_event_without_action_still_changes_its_list() {
        let message = parse(r#"{"Type":"image"}"#);

        assert_eq!(ListKind::from_event(&message), Some(ListKind::Images));
    }

    #[test]
    fn a_container_start_names_the_container() {
        let docker = parse(
            r#"{"Type":"container","Action":"start","Actor":{"ID":"c178aa0f"},"time":1793727983}"#,
        );

        let change = EngineChange::from_event(&docker).expect("a change");

        assert_eq!(change.list, ListKind::Containers);
        assert_eq!(change.started_container.as_deref(), Some("c178aa0f"));
    }

    /// `docker restart` sends kill, die, stop, start, restart and `podman
    /// restart` sends restart, die, init, start: exactly one start, so one
    /// restart clears the logs once.
    #[test]
    fn only_the_start_of_a_restart_names_the_container() {
        let actions = [
            "kill", "die", "stop", "restart", "init", "cleanup", "create",
            "pause", "unpause",
        ];

        for action in actions {
            let message = parse(&format!(
                r#"{{"Type":"container","Action":"{action}","Actor":{{"ID":"c1"}}}}"#
            ));

            let change = EngineChange::from_event(&message).expect("a change");

            assert_eq!(change.started_container, None, "{action}");
        }
    }

    #[test]
    fn a_start_of_something_else_names_no_container() {
        let message =
            parse(r#"{"Type":"network","Action":"start","Actor":{"ID":"n1"}}"#);

        let change = EngineChange::from_event(&message).expect("a change");

        assert_eq!(change.started_container, None);
    }

    #[test]
    fn a_start_without_an_id_names_no_container() {
        let change = EngineChange::from_event(&event("container", "start"))
            .expect("a change");

        assert_eq!(change.started_container, None);
    }

    #[test]
    fn events_that_change_no_list_are_no_change() {
        assert_eq!(EngineChange::from_event(&event("volume", "mount")), None);
    }

    #[test]
    fn only_the_four_listed_types_are_asked_for() {
        let filters = event_filters();

        assert_eq!(filters.len(), 1);
        assert_eq!(
            filters.get("type").map(Vec::as_slice),
            Some(
                [
                    String::from("container"),
                    String::from("image"),
                    String::from("volume"),
                    String::from("network"),
                ]
                .as_slice()
            )
        );
    }
}
