use std::collections::VecDeque;
use std::io;

use criterion::{
    BatchSize, Criterion, black_box, criterion_group, criterion_main,
};
use otty_libterm::{
    TerminalEngine, TerminalOptions, TerminalSize, escape,
    surface::{Surface, SurfaceConfig},
};
use otty_pty::Session;

type DefaultParser = escape::Parser<escape::vte::Parser>;

fn bench_engine_on_readable(c: &mut Criterion) {
    let payload = b"hello world\x1b[31m colored\x1b[0m\n".repeat(200);

    c.bench_function("engine_on_readable_frame_throughput", |b| {
        b.iter_batched(
            || {
                let session = BenchSession::with_reads(vec![payload.clone()]);
                let parser = DefaultParser::default();
                let surface = Surface::new(
                    SurfaceConfig::default(),
                    &TerminalSize::default(),
                );

                TerminalEngine::new(
                    session,
                    parser,
                    surface,
                    TerminalOptions::default(),
                )
                .expect("construct engine")
            },
            |(mut engine, _handle, events)| {
                engine.on_readable().expect("readable");
                while events.try_recv().is_ok() {}
                black_box(engine.has_pending_output());
            },
            BatchSize::SmallInput,
        );
    });
}

#[derive(Default)]
struct BenchSession {
    reads: VecDeque<Vec<u8>>,
}

impl BenchSession {
    fn with_reads(reads: Vec<Vec<u8>>) -> Self {
        Self {
            reads: reads.into(),
        }
    }
}

impl Session for BenchSession {
    fn read(
        &mut self,
        buf: &mut [u8],
    ) -> Result<usize, otty_pty::SessionError> {
        if let Some(mut chunk) = self.reads.pop_front() {
            let len = chunk.len().min(buf.len());
            buf[..len].copy_from_slice(&chunk[..len]);
            if len < chunk.len() {
                chunk.drain(0..len);
                self.reads.push_front(chunk);
            }
            return Ok(len);
        }
        Err(io::Error::from(io::ErrorKind::WouldBlock).into())
    }

    fn write(&mut self, input: &[u8]) -> Result<usize, otty_pty::SessionError> {
        Ok(input.len())
    }

    fn resize(
        &mut self,
        _size: otty_pty::PtySize,
    ) -> Result<(), otty_pty::SessionError> {
        Ok(())
    }

    fn close(&mut self) -> Result<i32, otty_pty::SessionError> {
        Ok(0)
    }

    fn try_get_child_exit_status(
        &mut self,
    ) -> Result<Option<std::process::ExitStatus>, otty_pty::SessionError> {
        Ok(None)
    }
}

criterion_group!(engine, bench_engine_on_readable);
criterion_main!(engine);
