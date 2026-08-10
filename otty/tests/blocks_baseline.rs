#![cfg(unix)]

use std::error::Error;
use std::io;
use std::time::{Duration, Instant};

use otty_libterm::escape::{Action, EscapeActor, EscapeParser, Parser};
use otty_libterm::surface::{
    BlockAlignment, BlockId, BlockSurface, Dimensions, Scroll, SurfaceActor,
    SurfaceConfig, SurfaceModel, TerminalSessionId,
};
use otty_libterm::{
    ChannelConfig, Driver, RuntimeHooks, TerminalBuilder, TerminalSize, pty,
};

const REQUESTED_BLOCKS: usize = 10_000;
const LONG_OUTPUT_LINES: usize = 100_000;
const QUEUE_OUTPUT_LINES: usize = 50_000;
const BUILD_COLUMNS: usize = 8;
const BUILD_LINES: usize = 2;
const VIEWPORT_COLUMNS: usize = 80;
const VIEWPORT_LINES: usize = 24;

struct BaselineDimensions {
    columns: usize,
    lines: usize,
}

impl BaselineDimensions {
    fn new(columns: usize, lines: usize) -> Self {
        Self { columns, lines }
    }
}

impl Dimensions for BaselineDimensions {
    fn total_lines(&self) -> usize {
        self.lines
    }

    fn screen_lines(&self) -> usize {
        self.lines
    }

    fn columns(&self) -> usize {
        self.columns
    }
}

struct BaselineActor {
    surface: BlockSurface,
}

impl BaselineActor {
    fn new() -> Self {
        let dimensions = BaselineDimensions::new(BUILD_COLUMNS, BUILD_LINES);
        let mut surface =
            BlockSurface::new(SurfaceConfig::default(), &dimensions);
        surface.register_terminal_session(TerminalSessionId::new("baseline"));

        Self { surface }
    }
}

impl EscapeActor for BaselineActor {
    fn handle(&mut self, action: Action) {
        match action {
            Action::Print(character) => self.surface.print(character),
            Action::CarriageReturn => self.surface.carriage_return(),
            Action::LineFeed => self.surface.line_feed(),
            Action::ProtocolEvent(event) => {
                self.surface.handle_protocol_event(event)
            },
            _ => {},
        }
    }
}

struct QueueBaseline {
    duration: Duration,
    unread_frame_depth: usize,
    replaced_frames: u64,
    lossless_queue_depth: usize,
    max_lossless_queue_depth: usize,
}

struct BaselineDeadline {
    deadline: Instant,
}

impl<D: Driver + ?Sized> RuntimeHooks<D> for BaselineDeadline {
    fn before_poll(&mut self, _driver: &mut D) -> otty_libterm::Result<()> {
        if Instant::now() >= self.deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "baseline PTY queue scenario timed out",
            )
            .into());
        }

        Ok(())
    }
}

#[test]
#[ignore = "explicit phase-00 baseline; run with --ignored --nocapture"]
fn blocks_phase_00_baseline() -> Result<(), Box<dyn Error>> {
    let started = Instant::now();
    let mut parser = Parser::<otty_libterm::escape::vte::Parser>::new();
    let mut actor = BaselineActor::new();
    emit_protocol_event(
        &mut parser,
        &mut actor,
        r#"{"v":2,"event":"shell_hello","terminal_session_id":"baseline","shell_instance_id":"root","seq":1,"payload":{"shell":"bash"}}"#,
    );

    let mut sequence = 2;
    for block in 1..=REQUESTED_BLOCKS {
        let block_id = format!("baseline:root:{block}");
        emit_protocol_event(
            &mut parser,
            &mut actor,
            &format!(
                r#"{{"v":2,"event":"prompt_prepare","terminal_session_id":"baseline","shell_instance_id":"root","seq":{sequence},"block_id":"{block_id}","payload":{{}}}}"#,
            ),
        );
        sequence += 1;
        actor.surface.print('x');
        actor.surface.carriage_return();
        actor.surface.line_feed();
    }
    eprintln!(
        "BLOCKS_BASELINE_STAGE name=blocks duration_ms={}",
        started.elapsed().as_millis(),
    );
    actor
        .surface
        .resize(BaselineDimensions::new(VIEWPORT_COLUMNS, VIEWPORT_LINES));

    let active_id = format!("baseline:root:{}", REQUESTED_BLOCKS + 1);
    emit_protocol_event(
        &mut parser,
        &mut actor,
        &format!(
            r#"{{"v":2,"event":"prompt_prepare","terminal_session_id":"baseline","shell_instance_id":"root","seq":{sequence},"block_id":"{active_id}","payload":{{}}}}"#,
        ),
    );
    sequence += 1;
    emit_protocol_event(
        &mut parser,
        &mut actor,
        &format!(
            r#"{{"v":2,"event":"command_start","terminal_session_id":"baseline","shell_instance_id":"root","seq":{sequence},"block_id":"{active_id}","payload":{{}}}}"#,
        ),
    );

    let anchor_id =
        BlockId::new(format!("baseline:root:{}", REQUESTED_BLOCKS - 100,));
    let found_offscreen = actor
        .surface
        .scroll_to_block(&anchor_id, BlockAlignment::Start);
    for _ in 0..LONG_OUTPUT_LINES {
        actor.surface.print('y');
        actor.surface.carriage_return();
        actor.surface.line_feed();
    }
    eprintln!(
        "BLOCKS_BASELINE_STAGE name=long_output duration_ms={}",
        started.elapsed().as_millis(),
    );

    let mut scroll_correct = found_offscreen;
    for columns in [200, 40] {
        actor
            .surface
            .resize(BaselineDimensions::new(columns, VIEWPORT_LINES));
        scroll_correct &= visible_block(&mut actor.surface, &anchor_id);
    }

    actor.surface.scroll_display(Scroll::Bottom);
    let snapshot_started = Instant::now();
    let snapshot = actor.surface.snapshot_owned();
    let snapshot_build = snapshot_started.elapsed();
    let snapshot_bytes = snapshot.estimated_bytes();
    let memory = actor.surface.memory_metrics();
    let model_duration = started.elapsed();
    let queue = run_slow_consumer()?;
    eprintln!(
        "BLOCKS_BASELINE_STAGE name=queue duration_ms={}",
        queue.duration.as_millis(),
    );
    let view = snapshot.view();

    assert!(
        scroll_correct,
        "viewport anchor or off-screen ScrollTo failed"
    );
    assert!(queue.unread_frame_depth <= 1);
    assert!(queue.replaced_frames > 0);
    assert!(queue.max_lossless_queue_depth <= 1);

    println!(
        "BLOCKS_BASELINE version=1 requested_blocks={REQUESTED_BLOCKS} retained_blocks={} finished_blocks={} active_blocks={} finished_lines={} active_lines={} columns={} viewport_lines={} long_output_lines={LONG_OUTPUT_LINES} model_duration_ms={} snapshot_bytes={snapshot_bytes} snapshot_build_us={} block_memory_bytes={} active_memory_bytes={} finished_memory_bytes={} queue_output_lines={QUEUE_OUTPUT_LINES} queue_duration_ms={} replaceable_frame_depth={} replaced_frames={} lossless_queue_depth={} max_lossless_queue_depth={} scroll_correct={scroll_correct}",
        memory.finished_block_count() + memory.active_block_count(),
        memory.finished_block_count(),
        memory.active_block_count(),
        memory.finished_lines(),
        memory.active_lines(),
        view.size.columns,
        view.size.screen_lines,
        model_duration.as_millis(),
        snapshot_build.as_micros(),
        memory.total_bytes(),
        memory.active_bytes(),
        memory.finished_bytes(),
        queue.duration.as_millis(),
        queue.unread_frame_depth,
        queue.replaced_frames,
        queue.lossless_queue_depth,
        queue.max_lossless_queue_depth,
    );

    Ok(())
}

fn emit_protocol_event(
    parser: &mut Parser<otty_libterm::escape::vte::Parser>,
    actor: &mut BaselineActor,
    json: &str,
) {
    let encoded = json
        .bytes()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let frame = format!("\x1bPotty-dcs;event-v2;h;{encoded}\x1b\\");

    parser.advance(frame.as_bytes(), actor);
}

fn visible_block(surface: &mut BlockSurface, block_id: &BlockId) -> bool {
    surface
        .snapshot_owned()
        .view()
        .blocks()
        .iter()
        .any(|block| block.meta.id == block_id.as_str() && block.line_count > 0)
}

fn run_slow_consumer() -> Result<QueueBaseline, Box<dyn Error>> {
    let output_script = format!(
        "i=0; while [ $i -lt {QUEUE_OUTPUT_LINES} ]; do printf 'q\\n'; i=$((i + 1)); done",
    );
    let session = pty::local("/bin/sh")
        .with_arg("-c")
        .with_arg(&output_script)
        .set_controling_tty_enable();
    let terminal_size = TerminalSize {
        cols: VIEWPORT_COLUMNS as u16,
        rows: VIEWPORT_LINES as u16,
        ..TerminalSize::default()
    };
    let (mut runtime, mut engine, _handle, events) =
        TerminalBuilder::from(session)
            .with_size(terminal_size)
            .with_channel_config(ChannelConfig::bounded(8))
            .build_with_runtime()?;

    let started = Instant::now();
    runtime.run(
        &mut engine,
        BaselineDeadline {
            deadline: started + Duration::from_secs(30),
        },
    )?;

    Ok(QueueBaseline {
        duration: started.elapsed(),
        unread_frame_depth: events.unread_frame_depth(),
        replaced_frames: events.replaced_frame_count(),
        lossless_queue_depth: events.lossless_queue_depth(),
        max_lossless_queue_depth: events.max_lossless_queue_depth(),
    })
}
