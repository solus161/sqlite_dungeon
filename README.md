# SQLite Dungeon

An educational, in-browser explorer for the SQLite file format. Drop in a `.db` file and walk through it: 
the 100-byte header, every page, every cell, every varint, and the B-tree paths a query takes.

> Status: early planning. The current code is a working SQLite reader from the
> [Codecrafters "Build your own SQLite"](https://app.codecrafters.io/courses/sqlite/overview) challenge.
> It will become the core of the visualizer.

## Why

Most SQLite visualizers are unmaintained or only show part of the format. This project's goals:

- **Teach.** Every field on screen points back to its exact bytes, and the spec sections behind them are one click away.
- **Show the algorithms.** Watch a `SELECT` walk the table B-tree or an index, one step at a time.
- **Keep it private.** By default the file is parsed in your browser via WebAssembly and never uploaded.
- **Last for years.** No frontend framework and no JS build chain, so there's little to update or break.

---

## Architecture

```
                 ┌───────────────────────────── Browser ──────────────────────────────┐
                 │  web/ UI (vanilla JS + Web Components + SVG)                       │
                 │        │ DataSource interface: header(), page(n), walk(sql), …     │
                 │        ├──────────────────────────────┐                            │
                 │        ▼ postMessage                  ▼ fetch (JSON)               │
                 │  ┌─ Web Worker ───────────────────┐   HttpSource ──────────────┐   │
  your.db ──────▶│  │ wasm → core                    │                           │   │
  (File, read    │  │ PageSource = FileReaderSync    │                           │   │
   page by page) │  │   over File.slice()            │                           │   │
                 │  └────────────────────────────────┘                           │   │
                 └───────────────────────────────────────────────────────────────┼───┘
                                                                                 │
                 ┌──────────────────────────── server (Axum) ────────────────────▼───┐
                 │  static web/ + wasm · /api/samples · /api/lab (rusqlite)          │
                 │  local mode: --db /path/big.db → /api/db/* (native core,          │
  big.db ───────▶│              PageSource = file on server disk)                    │
                 └───────────────────────────────────────────────────────────────────┘
```

One Rust core, two hosts:

| Crate | Role |
|---|---|
| `core` | All the Codecrafters code in one library crate, with no I/O and no panics. **File format:** the DB header, page headers, cell pointer arrays, cells, varints, record headers and serial types, overflow chains, and the freelist. Every parsed field carries a `Span { offset, len }` so the UI can highlight its bytes. Reads bytes one page at a time through a synchronous `PageSource` trait and never needs the whole file in memory. **SQL:** the tokenizer, parser, schema loading (`parse_meta`) and executor. Full scans and index scans report each step to a `Tracer` (`VisitPage`, `CompareKey`, `Descend`, `EmitRow`, …), and the UI replays those events. Schema loading needs the SQL parser, so the format and SQL code stay in one crate. |
| `wasm` | Thin `wasm-bindgen` layer that exposes `core` functions to JS. Runs inside a **Web Worker**. Its `PageSource` calls `FileReaderSync` on `File.slice()`, so reads stay synchronous and the UI thread never blocks. |
| `cli` | The Codecrafters-compatible command line (`.dbinfo`, `.tables`, `.page N`, `SELECT …`). Handy for checking `core` without the UI. |
| `server` | Axum app. It serves the UI as one binary with the assets embedded, provides a library of sample databases, and runs **local mode** (see below). It will later host the **mutation lab**: run `INSERT`/`DELETE` with real SQLite (rusqlite) and get before/after page snapshots that show B-tree splits and merges. |
| `web/` | Plain HTML/CSS/ES modules, native Custom Elements, and SVG for diagrams. No bundler and no npm. Views talk to a `DataSource` interface, not to WASM or HTTP directly. |

### Two ways to open a database

Both modes run the same `core` and return the same serde types. The UI can't tell which one it's using.

| | **Browser mode** (default) | **Local server mode** |
|---|---|---|
| Use when | The file is on the user's machine | The file already lives on the server's machine (a production DB, or a file too big for the browser to handle comfortably) |
| How | Drag-and-drop or the file picker | `server --db /path/big.db` |
| Parsing runs in | WASM in a Web Worker | Native Rust inside Axum |
| How bytes are read | `FileReaderSync` over `File.slice(offset, page_size)`, one page at a time | Positional file reads (later: mmap) |
| UI's `DataSource` | `WorkerSource` (`postMessage`) | `HttpSource` (`fetch` on `/api/db/*`) |
| Upload needed | No. The file never leaves the browser. | No. The file never leaves the server. |

Neither mode uploads the file. Uploading a large DB to a remote server is the slowest option, so the project doesn't support it.

**What file size changes.** Opening the file and inspecting single pages cost the same at 10 MB or 10 GB. Size only matters for work that touches every page:
- **Page map:** classify pages in the background and show progress, cache the results by page number, and draw a downsampled overview for very large page counts.
- **Full-scan replay:** cap the recorded trace (for example, the first N leaf pages) and tell the user it was cut short.
- **Page cache:** a small LRU of parsed pages in the core, so going back to a page doesn't read it again.

## Tech stack

| Area | Choice | Notes |
|---|---|---|
| Language | Rust (edition 2024) | |
| WASM | `wasm-bindgen` + `wasm-bindgen-cli` | Check whether `wasm-pack` is still maintained before depending on it. The rustwasm org was archived in 2025, so calling `wasm-bindgen-cli` directly is the safer bet. |
| Data to JS | `serde` + `serde-wasm-bindgen` | |
| Errors | `thiserror` | Replaces the hand-written `CustomErr` conversions |
| Server | `axum`, `tokio`, `tower-http` (ServeDir, compression), `tracing` | |
| Asset embedding | `rust-embed` | Single-binary deployment |
| Real SQLite (lab only) | `rusqlite` (bundled) | Server side only, never in the WASM build |
| Frontend | HTML, CSS (custom properties, light/dark), JS modules, Custom Elements, SVG | No dependencies to start with |
| Testing | `cargo test`, `wasm-bindgen-test`, `cargo-fuzz` | Optional later: Playwright for UI smoke tests |
| Task runner | `just` (or a Makefile) | `just wasm`, `just serve`, `just test` |

## Project structure (target)

```
sqlite_dungeon/
├── Cargo.toml                 # workspace
├── justfile
├── crates/
│   ├── core/                  # library: all Codecrafters code
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── pager.rs, btree.rs, exceptions.rs, utils.rs   # today
│   │       ├── processor.rs                                   # today: executor (full/index scan)
│   │       ├── parser/{mod,tokenizer,create,select}.rs        # today: SQL parser
│   │       └── later split btree.rs into: header, page, cell, record, varint,
│   │           overflow, freelist, span, source, trace
│   ├── wasm/
│   │   └── src/lib.rs
│   ├── server/
│   │   └── src/{main,routes,samples,local_db,lab}.rs
│   └── cli/                   # Codecrafters CLI (.dbinfo, .tables, .page, SELECT)
├── web/
│   ├── index.html
│   ├── css/
│   ├── js/
│   │   ├── app.js             # routing (hash-based), state
│   │   ├── worker.js          # Web Worker: loads pkg/, owns the File, FileReaderSync reads
│   │   ├── sources/           # DataSource: worker-source.js (browser mode), http-source.js (local mode)
│   │   └── components/        # <db-header>, <page-map>, <page-inspector>, <hex-view>,
│   │                          # <cell-view>, <btree-view>, <walk-player>, <schema-view>
│   └── pkg/                   # generated wasm (gitignored)
├── samples/                   # companies.db and small hand-made teaching DBs
└── docs/                      # format notes, design decisions
```

## Requirements

### Functional

| ID | Feature |
|---|---|
| F1 | Open a `.db` file: local file by file picker or drag-and-drop (browser mode), a file on the server (`--db`, local mode), or a sample DB. |
| F2 | **Header view.** All 100 header bytes, annotated (page size, encoding, schema format, page count, freelist, …). |
| F3 | **Page map.** A grid of every page, colored by type (table/index × interior/leaf, overflow, freelist, ptrmap), with fill-level shown. Clicking a page opens it. |
| F4 | **Page inspector.** The page's layout as bands (header, cell pointer array, unallocated space, cell content, freeblocks) next to a **hex view**. Hovering a field highlights its bytes and the reverse. |
| F5 | **Cell decoder.** Step-by-step varint decoding, the record header, serial type to value, and row id. Links to child pages. |
| F6 | **Overflow chain.** Show how much of the payload stays on the page and follow the chain across overflow pages. |
| F7 | **B-tree view.** An SVG tree for any table or index root. Expands lazily, shows the keys in each node, and links nodes to the page inspector. |
| F8 | **Query walk player.** Type a `SELECT` and get the chosen plan (full scan or index scan) plus a step-by-step replay on the B-tree view: play, pause, step, and scrub. |
| F9 | **Schema view.** The `sqlite_schema` rows, each linked to its root page. |
| F10 | **Sample library** served by the Axum server. |

### Non-functional

- No frontend framework and no JS build step. Any modern browser can load `index.html` with plain ES modules.
- Fully usable as static files, so it can run offline or on GitHub Pages. The server is optional.
- **Never panic on corrupt or hostile input.** Every parse returns `Result`, and the parsers are fuzzed.
- Read-only. The user's file is never modified.
- **Size-independent opening:** opening a DB and inspecting a page cost the same whether the file is 10 MB or 10 GB. The whole file is never loaded into memory. Pages are read and parsed on demand, and long lists are virtualized.
- Work that touches every page (page map, full-scan replay) runs off the UI thread, shows progress, and can be cancelled or capped.
- Keyboard navigable, readable in light and dark themes.

## Roadmap

**Local mode comes first.** Axum serves the UI and a database file from disk, so every feature first works end to end over HTTP, with no WASM involved. Browser mode (WASM in a worker) is added in Phase 6, behind the same `DataSource` interface, and the existing views should work unchanged. The order is meant for learning: Axum comes early and is used in every phase, and by the time WASM arrives the core and UI already work, so the new part is only the WASM build and its JS bindings.

### Phase 0 — Restructure
- [x] Convert to a cargo workspace (`core`, `wasm`, `server`, `cli`) and move all old code into `core`.
- [x] Make `core` a library: rename `src/main.rs` to `src/lib.rs` and declare the modules there. Add `paste.workspace = true` to `core`'s `Cargo.toml`.
- [x] Rename the `core` package to `dungeon_core`, because `core` collides with Rust's built-in `core` crate. The folder stays `crates/core`. Other crates depend on it with `dungeon_core = { path = "../core" }` and import it as `dungeon_core::…`.
- [x] Move the CLI `main.rs` into `crates/cli` and depend on `core` from there.
- [x] Keep the existing tests green. They use `companies.db`, so move it to `samples/`.

### Phase 1 — Core hardening
- [ ] Add a synchronous `PageSource` trait (`page_size()`, `page_count()`, `read_page(n)`) to replace the `std::fs`-bound `Pager`. Implement `MemSource(Vec<u8>)` for tests and small samples, and `FileSource` (positional reads) for native builds.
- [ ] Add a small LRU page cache in front of any `PageSource`.
- [ ] Add `Span` to every parsed field.
- [ ] Remove `expect`/`unwrap`/slice panics from parsing paths and make `parse_varint` bounds-checked.
- [ ] Fix known bugs:
  - [ ] Serial types 6 (`Int64`) and 7 (`Float64`) are 8 bytes long, not 6.
  - [ ] Serial types 8 and 9 should decode to the integers 0 and 1, not `Null`.
  - [ ] Read the interior-page right pointer relative to the page header, so it works on page 1 (offset 100 + 8).
  - [ ] Make the overflow chain advance its pointer, and compute the local payload size using the spec's U/X/M/K formula.
- [ ] Parse the freelist (trunk and leaf pages) and classify every page for the page map.
- [ ] Derive serde types for UI output.
- [ ] Add `cargo-fuzz` targets for page and record parsing.

### Phase 2 — Axum local mode + first screen
- [ ] Create the `server` skeleton:
  - CLI args (`--db`, `--port`, `--bind`)
  - shared `AppState` holding the open DB behind `Arc<Mutex<…>>`, since the core uses `Rc` and isn't `Sync`
  - `tracing` logs and graceful shutdown
- [ ] Bind to `127.0.0.1` by default, because local mode exposes a file on disk. Serving it on other interfaces needs an explicit flag.
- [ ] Open the DB read-only with `FileSource`. Add `GET /api/db/header`, `GET /api/db/page/:n`, and `GET /api/db/schema`. Responses are serde types from `core`.
- [ ] Add one `ApiError` type that implements `IntoResponse`, so core errors become proper HTTP status codes with a JSON body.
- [ ] Run CPU-heavy parsing in `spawn_blocking`.
- [ ] Serve `web/` from disk with `tower-http` `ServeDir` during development. Embedding the assets comes in Phase 7.
- [ ] Set up the `web/` skeleton: hash routing and theme. Define the **async** `DataSource` interface in JS and implement `HttpSource`.
- [ ] Build the header view (F2).
- [ ] Build the page map (F3). Add `GET /api/db/pages?from=&to=` for chunked classification and stream progress with **Server-Sent Events** (`/api/db/pages/stream`). Draw a downsampled overview when there are many pages.
- [ ] Test the handlers with `axum-test`, or with `tower::ServiceExt::oneshot`, against `samples/companies.db`.

### Phase 3 — Page & cell inspection
- [ ] Build `<hex-view>`: render only the visible rows and accept highlight ranges. Add `GET /api/db/page/:n/raw`, returning raw bytes as `application/octet-stream`.
- [ ] Build the page inspector bands (F4) with two-way hover sync.
- [ ] Build the cell decoder (F5) and the overflow chain view (F6). Add `GET /api/db/page/:n/cell/:i`, and `GET /api/db/overflow/:n` to follow the chain.
- [ ] Build the schema view (F9).

### Phase 4 — B-tree diagram
- [ ] Add `GET /api/db/tree/:root?depth=` to return a subtree skeleton (page ids, types, keys) for lazy expansion.
- [ ] Add a tree layout. Start with a simple layered layout and move to a tidy-tree algorithm later.
- [ ] Build `<btree-view>` with lazy expansion and pan/zoom (F7).

### Phase 5 — Query walk
- [ ] Add a `Tracer` trait to `core` and instrument `full_scan`, `index_scan`, and `extract_rows`.
- [ ] Add `POST /api/db/walk` with `{ sql, max_events }`, returning the plan and the event list. Cap the trace and flag it as truncated when it hits the limit.
- [ ] Add `GET /api/db/walk/ws` to stream events over a **WebSocket** as the query runs. The client can cancel by closing the socket.
- [ ] Build `<walk-player>`: replay the events on the B-tree view (F8).

### Phase 6 — WASM browser mode
- [ ] Fill in the `wasm` crate, mirroring the HTTP API: `open(file)`, `header()`, `page(n)`, `classify_pages(from, to)`, `walk(sql, max_events)`.
- [ ] Add `JsFileSource`: a `PageSource` that calls an imported JS function, `read_at(offset, len)`, which uses `FileReaderSync` on `File.slice()`.
- [ ] Add `worker.js` to run the WASM in a Web Worker, plus a small request/response protocol over `postMessage` with request ids, progress messages and cancellation.
- [ ] Implement `WorkerSource`. All existing views should work unchanged, which tests whether the `DataSource` abstraction is sound.
- [ ] Add the file picker and drag-and-drop (F1). Browser mode becomes the default when no `--db` is loaded.
- [ ] Add a `wasm-bindgen-test` suite, plus a check that the WASM build and the server return the same JSON for the same page.
- [ ] Test with a multi-GB generated DB to confirm opening it is instant and memory stays flat.

### Phase 7 — Server polish & distribution
- [ ] Embed `web/` and `pkg/` with `rust-embed`, serve correct MIME types (`application/wasm`) and enable compression, and ship as a single binary.
- [ ] Add `/api/samples` for listing and downloading sample DBs (F10).
- [ ] Stop parsing work when the client disconnects.
- [ ] Use mmap for `FileSource` (`memmap2`).
- [ ] Publish a static-only build (browser mode, no server) to GitHub Pages.

### Phase 8 — Ideas backlog
- [ ] **Mutation lab.** `/api/lab` runs `INSERT`/`DELETE` on a scratch DB with rusqlite and returns before/after page snapshots to animate page splits. Good further Axum practice: per-session scratch DBs, a cleanup task for idle sessions, rate-limiting middleware, and a custom extractor for the session.
- [ ] Ptrmap pages and auto-vacuum.
- [ ] WAL file viewer (frames, checkpoints).
- [ ] Diff two DB files page by page.
- [ ] Guided lessons ("What is a varint?", "Why indexes are fast").
- [ ] Shareable links to a sample DB + page + cell.
- [ ] Support `WITHOUT ROWID` tables and multi-column indexes in the SQL layer.

## Open questions

- ~~**Large files:** whole `ArrayBuffer` or lazy `Blob.slice`?~~ **Decided:** read lazily, one page at a time. In browser mode the WASM runs in a Web Worker and uses `FileReaderSync`, so `PageSource` stays synchronous. Files already on a server use local mode. Files are never uploaded.
- **Page map at extreme sizes:** for millions of pages, should classification be exhaustive (slow, exact) or sampled (fast, approximate)? Possibly sampled first, then refined.
- **Local mode transport:** the server parses natively and returns JSON (current plan). The alternative is to serve raw page bytes and let the browser's WASM parse them, which keeps a single parser but needs synchronous fetches from the worker.
- **Tree layout:** write it by hand, or vendor one small, stable library such as `d3-hierarchy`?
- **CLI:** keep the Codecrafters-compatible binary (`your_program.sh`) or retire it?
- **Deployment:** static hosting only, or a hosted Axum instance for the lab?

## Development (planned)

```sh
just test        # cargo test --workspace
just wasm        # cargo build -p wasm --target wasm32-unknown-unknown --release
                 #   && wasm-bindgen --target web --out-dir web/pkg ...
just serve       # cargo run -p server  → http://localhost:3000  (browser mode + samples)
just serve-db samples/companies.db
                 # cargo run -p server -- --db samples/companies.db  (local mode)
just cli samples/companies.db ".tables"
                 # cargo run -p cli -- samples/companies.db ".tables"
```

## References

- SQLite file format: <https://www.sqlite.org/fileformat2.html>
- SQLite B-tree module notes: <https://www.sqlite.org/btreemodule.html>
- Codecrafters SQLite challenge: <https://app.codecrafters.io/courses/sqlite/overview>
