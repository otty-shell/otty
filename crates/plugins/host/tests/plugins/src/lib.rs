//! The test plugins of `otty-plugin-host`. Each build turns on one
//! feature and gives one component; `scripts/build-test-plugins.sh`
//! builds them all and copies them to `tests/fixtures/`.

#[cfg(feature = "minimal")]
mod minimal {
    use crate::minimal::exports::otty::agent::agent::Guest;

    wit_bindgen::generate!({ path: "wit", world: "minimal" });

    struct Plugin;

    impl Guest for Plugin {
        fn version() -> String {
            eprintln!("the minimal plugin was asked its version");

            "1.0.0".to_string()
        }
    }

    export!(Plugin);
}

#[cfg(feature = "spin")]
mod spin {
    use crate::spin::exports::otty::agent::agent::Guest as AgentGuest;
    use crate::spin::exports::otty::test::misbehave::Guest as MisbehaveGuest;

    wit_bindgen::generate!({ path: "wit", world: "spin" });

    struct Plugin;

    impl AgentGuest for Plugin {
        fn version() -> String {
            "1.0.0".to_string()
        }
    }

    impl MisbehaveGuest for Plugin {
        fn spin() -> u64 {
            let mut turns: u64 = 0;
            loop {
                turns = std::hint::black_box(turns.wrapping_add(1));
                if std::hint::black_box(false) {
                    return turns;
                }
            }
        }

        fn hold(_mib: u32) -> u64 {
            0
        }
    }

    export!(Plugin);
}

#[cfg(feature = "grab-memory")]
mod grab_memory {
    use crate::grab_memory::exports::otty::agent::agent::Guest as AgentGuest;
    use crate::grab_memory::exports::otty::test::misbehave::Guest as MisbehaveGuest;

    wit_bindgen::generate!({ path: "wit", world: "grab-memory" });

    struct Plugin;

    impl AgentGuest for Plugin {
        fn version() -> String {
            "1.0.0".to_string()
        }
    }

    impl MisbehaveGuest for Plugin {
        fn spin() -> u64 {
            0
        }

        fn hold(mib: u32) -> u64 {
            let bytes = mib as usize * 1024 * 1024;
            let mut held = vec![0_u8; bytes];
            for page in held.chunks_mut(4096) {
                page[0] = 1;
            }

            std::hint::black_box(&held).len() as u64
        }
    }

    export!(Plugin);
}

#[cfg(feature = "read-files")]
mod read_files {
    use crate::read_files::exports::otty::agent::agent::Guest;

    wit_bindgen::generate!({ path: "wit", world: "read-files" });

    struct Plugin;

    impl Guest for Plugin {
        fn version() -> String {
            std::fs::read_to_string("/etc/hostname").unwrap_or_default()
        }
    }

    export!(Plugin);
}
