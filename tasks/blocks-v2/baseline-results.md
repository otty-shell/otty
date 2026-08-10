# Blocks v2: результаты baseline фазы 0

Дата измерения: 2026-08-10. Схема отчёта: `BLOCKS_BASELINE version=1`.

## Воспроизведение

Из корня workspace дважды выполнить одну команду:

```bash
cargo test --release -p otty --test blocks_baseline -- --ignored --nocapture
```

Итоговая строка каждого прогона начинается с `BLOCKS_BASELINE version=1` и содержит только
числовые параметры и boolean assertions. Prompt, command и output в отчёт не выводятся.
Строки `BLOCKS_BASELINE_STAGE` разделяют latency построения блоков, длинного output и PTY
queue-сценария, поэтому timeout или замедление можно локализовать.

Сценарий выполняет:

- 10 000 маленьких lifecycle-блоков с retention текущей модели;
- 100 000 строк mutable active output;
- off-screen `ScrollTo` и resize 80 → 200 → 40 columns;
- snapshot build/size и приблизительную block memory с отдельными active/history числами;
- 50 000 строк через настоящий PTY без чтения consumer-ом до завершения child;
- assertions для scroll correctness, replaceable frame depth и lossless queue depth.

PTY queue-сценарий имеет отдельный timeout 30 секунд. Benchmark failure по memory, latency,
queue depth или scroll correctness виден по отдельному полю/assertion.

## Машина

- ОС: Fedora Linux, kernel `7.0.12-101.fc43.x86_64`, `x86_64`.
- CPU: AMD Ryzen 9 7900, 12 cores / 24 threads.
- RAM: 31 942 900 KiB total.
- Rust: `rustc 1.96.1 (31fca3adb 2026-06-26)`, LLVM 22.1.2.
- Cargo: `cargo 1.96.1 (356927216 2026-06-26)`.
- Source HEAD: `8eff7639be53`, branch `blocks-v2`, включая рабочие изменения фазы 0.
- Profile: Cargo `release`, optimized.

## Результаты

Оба прогона содержали одинаковый набор полей.

| Метрика | Прогон 1 | Прогон 2 |
|---|---:|---:|
| requested blocks | 10 000 | 10 000 |
| retained blocks | 1 000 | 1 000 |
| finished / active blocks | 999 / 1 | 999 / 1 |
| finished / active lines | 23 976 / 10 024 | 23 976 / 10 024 |
| columns / viewport lines | 40 / 24 | 40 / 24 |
| long output lines | 100 000 | 100 000 |
| model duration | 1 647 ms | 1 652 ms |
| snapshot build | 14 952 µs | 16 306 µs |
| snapshot estimated bytes | 273 458 | 273 458 |
| total block memory estimate | 59 570 002 bytes | 59 570 002 bytes |
| active memory estimate | 9 969 651 bytes | 9 969 651 bytes |
| finished memory estimate | 49 600 351 bytes | 49 600 351 bytes |
| PTY queue output lines | 50 000 | 50 000 |
| PTY queue duration | 265 ms | 261 ms |
| replaceable frame depth | 1 | 1 |
| replaced frames | 4 847 | 5 096 |
| lossless queue depth / peak | 1 / 1 | 1 / 1 |
| scroll correct | true | true |

`snapshot estimated bytes` учитывает owned frame, cells, block metadata/text allocations,
damage и hyperlink index. `block memory estimate` учитывает block structs, logical grid rows,
cells, metadata и retained text allocations; это детерминированная приблизительная оценка, а
не RSS процесса. Mutable active content и finished history считаются отдельно.

## Проверки качества и coverage

Обязательные workspace-проверки прошли. Общий line coverage той же командой вырос с 51.05%
до 51.16%; region coverage — с 52.27% до 52.41%, function coverage — с 52.01% до 52.12%.
`cargo deny check` завершается успешно и сохраняет два ранее существовавших предупреждения о
yanked transitive crates `core2 0.4.0` и `spin 0.9.8`.

## GUI и shell smoke

- `cargo run -p otty` успешно открыл Wayland window и работал 30 секунд без panic/crash.
- Bash/Zsh и nested Bash выполняются через настоящий PTY в `shell_integration`.
- Test-only legacy fixture стабильно воспроизводит потерю nested integration и два одинаковых
  `cmd-1`; production v2 test подтверждает уникальные shell/block IDs.
- Model regression tests проверяют active growth, head retention/truncation fallback,
  resize matrix и полностью off-screen `ScrollTo`.

Интерактивный визуальный сценарий с wheel/key scroll остаётся описан в
`phase-00-baseline.md`; его результаты должны подтверждаться человеком на целевой desktop,
поскольку Wayland не разрешает тестовому процессу синтезировать глобальный input.

### Результат ручной GUI-проверки

Статус: **ожидается подтверждение**.

- Bash root/nested: anchor при output и resize — не проверено; off-screen `ScrollTo` — не
  проверено.
- Zsh root/nested: anchor при output и resize — не проверено; off-screen `ScrollTo` — не
  проверено.
