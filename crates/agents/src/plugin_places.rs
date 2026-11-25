//! The files of the places an agent part names, such as where its
//! sessions or its answers are (spec 2026-11-19-agent-plugins, Decision
//! 6): the host lists them, and asks the part to read each one that
//! changed as the kind of its place.

use std::collections::HashMap;
use std::path::PathBuf;

use otty_plugin_host::CalledPart;
use otty_plugin_host::agents::agent::{Place, PlaceItem};

/// The files of `places` that `part` may read, by path, each as the part
/// is asked to read it: with its size, its change time and the kind of
/// its place. A place whose glob the host refused gives no files.
pub(crate) fn place_items(
    part: &CalledPart,
    places: &[Place],
) -> HashMap<PathBuf, PlaceItem> {
    let mut items = HashMap::new();

    for place in places {
        let found = part.plugin().list_files(part.part(), &place.glob);
        let files = match found {
            Ok(files) => files,
            Err(error) => {
                log::warn!(
                    "the place {} of the part {} was not listed: {error:?}",
                    place.glob,
                    part.part()
                );
                Vec::new()
            },
        };

        for file in files {
            let path = PathBuf::from(&file.path);
            items.insert(
                path,
                PlaceItem {
                    file,
                    kind: place.kind,
                },
            );
        }
    }

    items
}
