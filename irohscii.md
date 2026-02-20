# irohscii — Comprehensive Study Guide

> **ASCII art drawing tool with real-time P2P collaboration via [iroh](https://iroh.computer) and [automerge](https://automerge.org) CRDTs.**

---

## Table of Contents

1. [Project Overview](#1-project-overview)
2. [Architecture & Crate Structure](#2-architecture--crate-structure)
3. [Key Technologies Deep-Dive](#3-key-technologies-deep-dive)
   - [Iroh (P2P Networking)](#iroh-p2p-networking)
   - [Automerge (CRDTs)](#automerge-crdts)
   - [How They Work Together](#how-iroh-and-automerge-work-together)
4. [The Document Model](#4-the-document-model)
5. [Sync Architecture](#5-sync-architecture)
6. [Presence System](#6-presence-system)
7. [Session Management](#7-session-management)
8. [Undo/Redo System](#8-undoredo-system)
9. [Rendering & UI](#9-rendering--ui)
10. [Shape System & Geometry](#10-shape-system--geometry)
11. [Data Flow Walkthrough](#11-data-flow-walkthrough)
12. [Key Design Decisions](#12-key-design-decisions)
13. [Likely Interview Questions](#13-likely-interview-questions)

---

## 1. Project Overview

**irohscii** is a terminal-based (TUI) collaborative ASCII art editor — think Google Docs meets asciiflow, but fully peer-to-peer with no server. Users draw shapes (rectangles, arrows, diamonds, text, etc.) in a terminal using mouse and keyboard, and all edits sync in real-time across peers via P2P networking.

### Core Principles

| Principle | Implementation |
|-----------|---------------|
| **Local-first** | Automerge document is THE source of truth locally; works offline |
| **P2P / no server** | Iroh provides direct peer connections via QUIC |
| **Conflict-free** | CRDTs ensure all edits merge automatically, no conflicts ever |
| **Real-time** | Presence (cursors, activity) syncs at 20 Hz; document syncs at 5 Hz |

### How It Runs

```
# Start a new session (generates shareable ticket)
nix run github:brittonr/irohscii

# Join someone's session
nix run github:brittonr/irohscii -- --join <TICKET>

# Offline mode (no sync)
nix run github:brittonr/irohscii -- --offline
```

---

## 2. Architecture & Crate Structure

The project is a Rust workspace with **5 library crates** + 1 binary crate:

```
irohscii (workspace root — binary crate)
├── crates/
│   ├── irohscii-geometry    # Pure geometry: line drawing, shape rendering, viewport
│   ├── irohscii-core        # Document model, shapes, layers (Automerge-backed)
│   ├── irohscii-sync        # P2P sync: Iroh networking + Automerge sync + presence
│   ├── irohscii-export      # ASCII and SVG export
│   └── irohscii-session     # Session management, undo history
└── src/                     # Binary: TUI app, tools, modes, UI rendering
```

### Dependency Graph (bottom-up)

```
irohscii-geometry          (zero dependencies — pure math)
       ↑
irohscii-core              (depends on geometry + automerge)
       ↑
irohscii-sync              (depends on core + iroh + aspen-automerge)
irohscii-export            (depends on core + geometry)
irohscii-session           (depends on core + automerge)
       ↑
irohscii (binary)          (depends on ALL crates + ratatui + crossterm)
```

### Why This Matters

- **Separation of concerns**: Geometry is pure math (testable with proptest). Core is the data model. Sync is networking. UI is rendering.
- **The `irohscii-geometry` crate has zero external dependencies** — it's pure algorithms, making it highly testable and reusable.
- **The `irohscii-core` crate owns the Automerge document** — every shape mutation goes through it, ensuring CRDT consistency.

---

## 3. Key Technologies Deep-Dive

### Iroh (P2P Networking)

**What it is**: Iroh is a P2P networking library built on **QUIC** (via Quinn). It provides:

- **Cryptographic identity**: Every peer gets a unique Ed25519 keypair. The public key = the peer's identity.
- **Endpoint discovery**: Peers find each other via DNS discovery (n0.computer's relay infrastructure) and Pkarr (DHT publishing).
- **NAT traversal**: Uses relay servers for hole punching; falls back to relaying if direct connection fails.
- **ALPN-based protocol routing**: Multiple protocols multiplex over the same QUIC connection, distinguished by ALPN (Application-Layer Protocol Negotiation) strings.

**How irohscii uses it**:

```rust
// Create an iroh endpoint with discovery
let endpoint = Endpoint::builder()
    .discovery(DnsDiscovery::n0_dns())     // Find peers via DNS
    .discovery(PkarrPublisher::n0_dns())   // Publish our address to DNS
    .bind().await?;
```

**Key concept — EndpointAddr**: An `EndpointAddr` bundles the peer's public key + relay URL + direct addresses. This is serialized into a **ticket string** (base32-encoded, prefixed with `irohscii1`) that users share to connect.

```rust
// Encoding: EndpointAddr → "irohscii1ABCD..."
pub fn encode_ticket(addr: &EndpointAddr) -> String {
    let bytes = postcard::to_stdvec(addr).unwrap();
    format!("irohscii1{}", data_encoding::BASE32_NOPAD.encode(&bytes))
}
```

**Key concept — ALPN Protocol Routing**: Iroh uses a `Router` to accept incoming connections and dispatch them by ALPN:

```rust
let router = iroh::protocol::Router::builder(endpoint.clone())
    .accept(AUTOMERGE_SYNC_ALPN, sync_handler)      // Document sync
    .accept(PresenceProtocol::ALPN, presence_handler) // Cursor sync
    .spawn();
```

This means **document sync and presence sync are separate protocols** running over the same peer connection infrastructure, but on different QUIC streams.

### Automerge (CRDTs)

**What it is**: Automerge is a **Conflict-free Replicated Data Type (CRDT)** library. It provides a JSON-like document model where:

- Multiple users can edit simultaneously
- Edits **never conflict** — they merge deterministically
- No central server needed for coordination
- Works offline; syncs when connectivity resumes

**How CRDTs work (conceptually)**:

```
User A adds rectangle at (10, 20)  →  {shapes: {uuid1: {kind: "Rectangle", ...}}}
User B adds arrow at (5, 5)        →  {shapes: {uuid2: {kind: "Arrow", ...}}}

After merge: {shapes: {uuid1: ..., uuid2: ...}}  ← Both shapes exist, no conflict!
```

CRDTs guarantee **strong eventual consistency**: if two peers have seen the same set of changes, they'll have identical document state — regardless of the order changes were applied.

**How irohscii uses Automerge**:

The document is structured as a nested map:

```
ROOT
├── "id"          → DocumentId (UUID string)
├── "shapes"      → Map { shape_uuid → Map { kind, start_x, start_y, ... } }
├── "shape_order" → List [ shape_uuid, shape_uuid, ... ]  (render order, bottom→top)
├── "groups"      → Map { group_uuid → Map { members: List, parent? } }
├── "layers"      → Map { layer_uuid → Map { name, visible, locked } }
├── "layer_order" → List [ layer_uuid, ... ]
├── "undo_stack"  → List [ serialized_snapshot_bytes, ... ]
└── "redo_stack"  → List [ serialized_snapshot_bytes, ... ]
```

Every mutation goes through Automerge **transactions**:

```rust
let mut tx = self.doc.transaction();
tx.put(&shape_obj, "start_x", 10i64)?;
tx.put(&shape_obj, "start_y", 20i64)?;
tx.commit();
```

**Why UUIDs for shape IDs**: In a CRDT, you can't use auto-incrementing IDs because two peers might generate the same ID simultaneously. UUIDs (v4) provide globally unique identifiers without coordination.

### How Iroh and Automerge Work Together

The integration follows this pattern:

```
┌─────────────────┐                    ┌─────────────────┐
│   Local Peer     │                    │  Remote Peer     │
│                  │                    │                  │
│  Automerge Doc  ←──── iroh QUIC ────→ Automerge Doc    │
│  (source of      │   (transport)      │  (source of      │
│   truth locally) │                    │   truth locally) │
└─────────────────┘                    └─────────────────┘
```

1. User edits locally → mutation applied to local Automerge doc
2. Local doc serialized → sent to sync thread
3. Sync thread pushes changes via `sync_with_peer()` (Automerge's sync protocol) over iroh QUIC connection
4. Remote peer receives changes → merges into their Automerge doc
5. Remote peer's UI rebuilds from merged doc

**The Automerge sync protocol** is message-based: peers exchange "sync messages" that contain only the **changes the other peer doesn't have** (not the full document). This is efficient — after initial sync, only deltas are exchanged.

**The `aspen-automerge` crate** provides the sync handler that implements this protocol as an iroh `ProtocolHandler`, using the ALPN `b"aspen-automerge/sync/1"`.

---

## 4. The Document Model

**File**: `crates/irohscii-core/src/document.rs`

The `Document` struct wraps an `Automerge` instance:

```rust
pub struct Document {
    doc: Automerge,              // THE source of truth
    storage_path: Option<PathBuf>, // Where to persist on disk
    dirty: bool,                 // Unsaved changes flag
}
```

### Shape Types (`ShapeKind`)

All 17 shape types in a single enum:

| Shape | Key Fields |
|-------|-----------|
| `Line` | start, end, style, connections, label, color |
| `Arrow` | start, end, style, connections, label, color |
| `Rectangle` | start, end, label, color |
| `DoubleBox` | start, end, label, color |
| `Diamond` | center, half_width, half_height, label, color |
| `Ellipse` | center, radius_x, radius_y, label, color |
| `Triangle` | p1, p2, p3, label, color |
| `Freehand` | points (Vec), char, label, color |
| `Text` | pos, content, color |
| `Parallelogram` | start, end, label, color |
| `Hexagon` | center, radius_x, radius_y, label, color |
| `Trapezoid` | start, end, label, color |
| `RoundedRect` | start, end, label, color |
| `Cylinder` | start, end, label, color |
| `Cloud` | start, end, label, color |
| `Star` | center, outer_radius, inner_radius, label, color |

### Layers

Layers provide organization:

- Each shape belongs to exactly one layer
- Layers can be **visible/hidden** and **locked/unlocked**
- Layer order determines rendering order
- Locked layers prevent editing (but not viewing)
- Deleting a layer moves its shapes to the default layer

### Groups

Groups bundle multiple shapes:

- Selecting one shape in a group selects all
- Moving one moves all
- Supports nested groups (parent chain)
- Stored in Automerge as a separate `groups` map

### Connections

Lines/arrows can **snap to shapes** and maintain connections:

- Each line endpoint stores an optional `connection` (shape UUID as u64)
- When a connected shape moves, connected lines update their endpoints
- Snap points are the midpoints of shape edges (top, bottom, left, right)

---

## 5. Sync Architecture

**File**: `crates/irohscii-sync/src/sync/mod.rs`

The sync system runs on a **dedicated thread** with its own tokio runtime, communicating with the main thread via channels.

### Thread Architecture

```
┌─────────────────────────────────────────┐
│              Main Thread                 │
│  (crossterm events → App mutations →     │
│   render via ratatui)                    │
│                                          │
│  SyncCommand ──→ [command_tx] ──→        │
│                                   ↓      │
│  ←── [event_rx] ←── SyncEvent           │
└─────────────────────────────────────────┘
                    ↕
┌─────────────────────────────────────────┐
│           Sync Thread (tokio)            │
│                                          │
│  ┌── iroh Endpoint ──────────────────┐  │
│  │  ├── Router                        │  │
│  │  │   ├── ALPN: automerge/sync/1   │  │
│  │  │   │   (AutomergeSyncHandler)    │  │
│  │  │   └── ALPN: irohscii/presence/1│  │
│  │  │       (PresenceProtocol)        │  │
│  │  ├── Persistent Peer Connection    │  │
│  │  └── Persistent Cluster Connection │  │
│  └────────────────────────────────────┘  │
│                                          │
│  LocalDocumentStore (in-memory)          │
│    ├── docs: HashMap<key, Vec<u8>>       │
│    └── change_tx → notifies on sync      │
└─────────────────────────────────────────┘
```

### Communication Protocol

**Commands (Main → Sync)**:

| Command | Purpose |
|---------|---------|
| `SyncDoc { doc }` | Push local document changes to peers |
| `BroadcastPresence(presence)` | Send cursor/activity to peers |
| `ConnectCluster { ticket }` | Connect to an aspen cluster node |
| `Shutdown` | Clean shutdown |

**Events (Sync → Main)**:

| Event | Purpose |
|-------|---------|
| `Ready { endpoint_id, local_peer_id }` | Connection established |
| `RemoteChanges { doc }` | Received changes from peers |
| `PresenceUpdate(presence)` | Remote cursor moved |
| `PresenceRemoved { peer_id }` | Peer disconnected |
| `Error(msg)` | Something went wrong |

### Sync Timing

| Interval | What | Why |
|----------|------|-----|
| 200ms | Periodic sync with peer | Pull remote changes regularly |
| 2s | Periodic sync with cluster | Persistence is less urgent |
| 50ms | Presence broadcast | Smooth cursor tracking (20 Hz) |
| 16ms | Event loop poll | ~60 FPS rendering |

### The LocalDocumentStore

This is the bridge between the sync protocol and the application:

```rust
pub struct LocalDocumentStore {
    docs: RwLock<HashMap<String, Vec<u8>>>,  // doc bytes in memory
    metas: RwLock<HashMap<String, DocumentMetadata>>,
    change_tx: mpsc::Sender<String>,          // notify app of changes
}
```

- **`update_from_app()`**: App pushes local changes. Does NOT notify (avoids feedback loop).
- **`save()` (from sync)**: Sync protocol saves merged doc. DOES notify app if content changed.
- Change detection: only sends notifications when bytes actually differ (avoids spurious events from no-op syncs).

### Ticket System

Tickets encode peer connection info:

```
irohscii1ABCDEF...  →  base32(postcard(EndpointAddr))
                        Contains: PublicKey + relay URL + direct addresses

amsync1ABCDEF...    →  Aspen sync ticket (address + capability token)
                        Used for authenticated cluster connections
```

### Connection Model

- **Peer sync**: One persistent QUIC connection reused for all syncs. `sync_with_peer()` opens a new bi-directional stream each time (QUIC multiplexing).
- **Cluster sync**: Separate persistent connection to an aspen cluster node for durable storage.
- **Presence**: Separate QUIC connection with different ALPN, running a continuous bidirectional loop.

---

## 6. Presence System

**File**: `crates/irohscii-sync/src/presence.rs`, `crates/irohscii-sync/src/sync/presence_protocol.rs`

Presence is the system for showing **where other users' cursors are** and **what they're doing** in real-time.

### Why Separate from Document Sync?

Presence data is:

- **Ephemeral** (not persisted — lost on disconnect)
- **High-frequency** (20 updates/second per peer)
- **Small** (just cursor position + activity enum)

Document sync is:

- **Persistent** (saved to disk and CRDT history)
- **Lower-frequency** (5 syncs/second)
- **Larger** (full document state)

Mixing them would be wasteful — you'd be adding throwaway cursor positions to the permanent CRDT history.

### PeerPresence Data

```rust
pub struct PeerPresence {
    pub peer_id: PeerId,              // 32-byte Ed25519 public key
    pub cursor_pos: Position,          // Where their cursor is
    pub activity: CursorActivity,      // What they're doing
    pub color_index: u8,               // Deterministic color assignment
    pub timestamp_ms: u64,             // For ordering/staleness
    pub active_layer_id: Option<LayerId>, // Which layer they're on
    pub drag_start_ms: Option<u64>,    // For soft lock priority
}
```

### CursorActivity States

| Activity | What's Shown | Extra Data |
|----------|-------------|-----------|
| `Idle` | Colored cursor block | — |
| `Drawing` | Ghost preview of shape being drawn | tool, start, current position |
| `Selected` | Colored corners around shape | shape_id |
| `Dragging` | Dashed ghost rectangle at new position | shape_id, delta |
| `Resizing` | Dashed ghost rectangle at new bounds | shape_id, preview_bounds |
| `Typing` | Blinking cursor indicator | position |

### Soft Lock Mechanism

When two peers try to drag the same shape simultaneously:

- Each drag records `drag_start_ms` (when the drag began)
- The peer who started **earlier** gets priority (first-dragger-wins)
- The other peer sees the first peer's ghost and can choose to let go
- This is a **soft lock** — no hard prevention, just visual feedback

### Presence Protocol Wire Format

```
[4 bytes: length LE u32][msgpack-encoded PresenceMessage]
```

Messages are serialized with MessagePack (rmp-serde) for compactness.

### Staleness

Peers that haven't sent presence in 5 seconds are pruned from the active list.

---

## 7. Session Management

**File**: `crates/irohscii-session/src/session.rs`

Sessions provide **named workspaces** for organizing multiple drawings.

### Storage Layout

```
~/.local/share/irohscii/
├── sessions/
│   ├── my-diagram/
│   │   ├── document.automerge    # The actual drawing data
│   │   └── meta.json             # Session metadata
│   └── architecture-sketch/
│       ├── document.automerge
│       └── meta.json
└── session_registry.json          # Index of all sessions
```

### SessionMeta

```rust
pub struct SessionMeta {
    pub id: SessionId,           // URL-safe slug ("my-diagram")
    pub name: String,            // Human name ("My Diagram")
    pub description: Option<String>,
    pub created_at: u64,
    pub last_accessed: u64,
    pub ticket: Option<TicketInfo>,  // Last sync ticket
    pub collaborators: Vec<Collaborator>,
    pub tags: Vec<String>,
    pub pinned: bool,
}
```

### Session Lifecycle

1. **Create**: `SessionManager::create_session("My Diagram")` → creates directory, empty doc, meta.json
2. **Open**: `SessionManager::open_session(&id)` → loads automerge doc + meta from disk
3. **Save**: `SessionManager::save_session(&id, &doc, &meta)` → writes doc bytes + meta JSON
4. **Switch**: Save current → load new → reset UI state
5. **Delete**: Remove directory + update registry

---

## 8. Undo/Redo System

**File**: `crates/irohscii-session/src/undo.rs` + `crates/irohscii-core/src/document.rs`

### The Challenge

Automerge **doesn't support rollback**. It's an append-only CRDT — every change is permanent in the history. You can't "undo" a CRDT operation because other peers may have already built on top of it.

### The Solution: Snapshot-Based Undo

irohscii uses **serialized document snapshots** for undo:

```
Before mutation:  serialize(current_shapes) → push to undo_stack
After undo:       pop from undo_stack → deserialize → replace current shapes
                  serialize(current_shapes) → push to redo_stack
```

### Two Undo Implementations

1. **Global (CRDT-synced)**: Stored in the Automerge document itself (`undo_stack`/`redo_stack` lists). Syncs across peers. Used by the Document struct. Max 50 entries.

2. **Local (per-session)**: `UndoManager` with either memory or disk-backed storage. Supports unlimited history via disk snapshots. NOT synced.

### Disk-Backed Storage

```
~/.local/share/irohscii/undo/{session_id}/
├── 00000000.snap
├── 00000001.snap
├── 00000002.snap
└── ...
```

- Memory cache of 20 most recent snapshots for performance
- No limit on total snapshots (disk-backed)
- Cleanup on session close

---

## 9. Rendering & UI

**File**: `src/ui.rs`

The UI uses **ratatui** (Rust TUI framework) with **crossterm** as the terminal backend.

### Layout

```
┌──────────────────────────────────────────────────────┐
│                    Canvas Area                        │ ← Shapes render here
│  (infinite, pannable, zoomable)                      │
│                                                       │
│  ┌─ Layer Panel ─┐  ┌─ Participants ─┐              │ ← Optional side panels
│  │ ● Layer 1     │  │ ● You  [L1]    │              │
│  │   Layer 2     │  │ █ Peer-ab (Idle)│              │
│  └───────────────┘  └────────────────┘              │
├──────────────────────────────────────────────────────┤
│ SEL  [unsaved]  [3]  [SYNC 1]               Default │ ← Status bar (Helix-style)
├──────────────────────────────────────────────────────┤
│ s:select f:freehand t:text l:line r:rect ?:help     │ ← Help bar
└──────────────────────────────────────────────────────┘
```

### Render Order (painter's algorithm, bottom to top)

1. Grid dots (if enabled)
2. Snap guide lines
3. All shapes (via `ShapeView` cache, respecting z-order and layer visibility)
4. Freehand preview (while drawing)
5. Snap point diamonds (when line/arrow tool active)
6. Shape preview (while drawing line/rect/etc.)
7. Active snap indicator
8. Marquee selection box
9. Selection bounding boxes + resize handles
10. Remote cursors + activity ghosts (always on top)

### Mode System

The app uses a **modal UI** inspired by Vim/Helix:

```rust
pub enum Mode {
    Normal,                    // Default — keyboard shortcuts active
    TextInput(TextInputState), // Typing text at a position
    LabelInput(LabelInputState), // Editing a shape's label
    PathInput(PathInputState), // File open/save dialog
    RecentFiles(RecentFilesState),
    SelectionPopup(SelectionPopupState), // Tool/color/brush picker
    ConfirmDialog(ConfirmDialogState),
    HelpScreen(HelpScreenState),
    SessionBrowser(SessionBrowserState),
    SessionCreate(SessionCreateState),
    KeyboardShapeCreate(KeyboardShapeState),
    LayerRename(LayerRenameState),
}
```

Each mode implements `handle_key()` returning a `ModeTransition`:

```rust
pub enum ModeTransition {
    Stay,                    // Stay in current mode
    Normal,                  // Return to Normal mode
    To(Box<Mode>),          // Switch to a different mode
    Action(ModeAction),     // Trigger a side effect
}
```

### ShapeView (Render Cache)

The `ShapeView` is a **read-only cache** rebuilt from the Automerge document after every mutation. It avoids repeatedly deserializing shapes from Automerge during rendering:

```rust
pub struct ShapeView {
    shapes: Vec<CachedShape>,          // All shapes, pre-deserialized
    visible_shapes: Vec<usize>,         // Indices of visible shapes
    spatial_index: HashMap<(i32, i32), Vec<usize>>, // For hit-testing
}
```

---

## 10. Shape System & Geometry

**File**: `crates/irohscii-geometry/src/lib.rs`

### ASCII Art Rendering

Each shape type has a corresponding function that returns `Vec<(Position, char)>`:

```rust
// Rectangle → box-drawing characters
fn rect_points(start: Position, end: Position) -> Vec<(Position, char)>
// Returns: ┌──────┐
//          │      │
//          └──────┘

// Line → directional characters
fn line_points_styled(start, end, style) -> Vec<(Position, char)>
// Straight:    ─────  or  │  or  / \
// Orthogonal:  ──┐       ──┐
//                │         └──

// Ellipse → Bresenham's algorithm adapted for ASCII
fn ellipse_points(center, rx, ry) -> Vec<(Position, char)>
```

### Line Styles

```rust
pub enum LineStyle {
    Straight,        // Direct line: ─ │ / \
    OrthogonalHV,    // Horizontal first, then vertical: ──┐
    OrthogonalVH,    // Vertical first, then horizontal:   │
    OrthogonalAuto,  // Auto-routed to avoid obstacles     └──
}
```

### Viewport (Infinite Canvas)

The canvas is **infinite** — positions can be negative. The `Viewport` handles pan/zoom:

```rust
pub struct Viewport {
    pub offset_x: i32,   // Pan offset
    pub offset_y: i32,
    pub width: u16,       // Terminal size
    pub height: u16,
    pub zoom: f32,        // 0.25 to 4.0
}
```

- `screen_to_canvas()`: Convert mouse clicks to canvas coordinates
- `canvas_to_screen()`: Convert shape positions to terminal cells
- Zoom: at 2.0x, each canvas cell takes 2 terminal cells

### Snap Points

Shapes expose **snap points** (midpoints of edges) that lines/arrows can connect to:

```rust
pub struct SnapPoint {
    pub pos: Position,
    pub shape_id: ShapeId,
}
```

When drawing a line near a shape edge, it "snaps" to the nearest snap point (within a threshold of 3 characters). This creates visual connections that update when shapes move.

---

## 11. Data Flow Walkthrough

### Drawing a Rectangle (Single User)

```
1. Mouse down       → app.start_shape(pos)         [ShapeState created]
2. Mouse move       → app.update_shape(pos)         [Preview renders in yellow]
3. Mouse up         → app.commit_shape()
   a. save_undo_state()                              [Snapshot pushed to undo_stack]
   b. doc.add_shape(ShapeKind::Rectangle{...})       [Automerge transaction]
   c. shape_view.rebuild(&doc)                        [Cache rebuilt]
   d. doc.mark_dirty()                                [Triggers autosave]
4. Sync thread      → SyncDoc sent to peer           [Automerge bytes pushed]
```

### Receiving a Remote Edit

```
1. Sync protocol    → Remote peer sends sync message
2. LocalDocumentStore::save()                         [Merged doc saved, change_tx fires]
3. SyncEvent::RemoteChanges { doc }                   [Event sent to main thread]
4. app.merge_remote(&mut doc)
   a. self.doc.merge(other)                           [Automerge merge — conflict-free!]
   b. self.shape_view.rebuild(&doc)                   [Cache rebuilt with new shapes]
5. Next frame       → UI renders with merged state
```

### Presence Update

```
1. Mouse moves          → cursor_pos updated
2. Every 50ms           → app.build_presence(cursor_pos)
3. SyncCommand::BroadcastPresence(presence)
4. presence_protocol.broadcast(presence)              [To all peer connections]
5. Remote peer receives → PresenceMessage::Update
6. SyncEvent::PresenceUpdate(presence)                [To main thread]
7. presence_manager.update_peer(presence)
8. Next frame           → render_remote_cursor()      [Colored cursor + activity ghost]
```

---

## 12. Key Design Decisions

### 1. "Automerge Document IS the Source of Truth"

Every shape exists in the Automerge document. The `ShapeView` cache is rebuilt from it. This means:

- Sync is trivial: just merge Automerge documents
- No separate "model" to keep in sync with the document
- Trade-off: rebuilding the cache on every edit (mitigated by efficient Automerge reads)

### 2. Separate Thread for Sync

The sync thread runs its own tokio runtime. Benefits:

- Main thread (UI) is never blocked by network I/O
- Sync can happen concurrently with rendering
- Clean separation via channels (no shared mutable state between threads)

### 3. Presence as Ephemeral, Separate Protocol

Cursor positions don't go into the CRDT because:

- They'd pollute the permanent history
- They're too high-frequency for document sync
- They're inherently ephemeral (meaningless after disconnect)

### 4. Snapshot-Based Undo (not CRDT rollback)

Automerge has no `undo()` operation. The solution:

- Serialize all shapes before each mutation
- Store serialized bytes on undo/redo stacks
- Undo = restore previous serialized state
- This IS stored in the CRDT, so undo/redo syncs across peers!

### 5. UUID-Based Shape Identity

Every shape gets a UUID v4 instead of sequential IDs because:

- Two peers creating shapes simultaneously won't have ID collisions
- UUIDs work across CRDT merges
- No coordination needed for ID generation

### 6. Persistent QUIC Connections

Instead of connecting/disconnecting for each sync, connections are kept alive:

- QUIC supports multiplexed streams on a single connection
- Each sync opens a new bidirectional stream (cheap)
- Reduces connection establishment overhead
- Handles NAT keepalive naturally

---

## 13. Likely Interview Questions

### Conceptual Questions

**Q: How do CRDTs ensure no conflicts?**
> Automerge uses operation-based CRDTs. Each edit generates an operation with a globally unique ID (actor ID + sequence number). Concurrent edits to different keys naturally merge. Concurrent edits to the same key use last-writer-wins with deterministic tiebreaking. The key insight: by using UUIDs for shape IDs, two users creating shapes simultaneously create entries at different keys, so they never conflict.

**Q: What happens if two users edit the same shape simultaneously?**
> Each property (start_x, start_y, etc.) is independently conflict-resolved via last-writer-wins. So if User A moves the left edge and User B changes the color, both changes merge cleanly. If both move the left edge, the last write (by deterministic ordering) wins. The soft-lock system (via presence) discourages this by showing who's already interacting with a shape.

**Q: Why use iroh instead of WebSockets/WebRTC?**
> Iroh provides: (1) true P2P with no server needed for data, (2) cryptographic peer identity for free, (3) QUIC transport with built-in encryption and multiplexing, (4) NAT traversal via relay infrastructure, (5) ALPN-based protocol routing. It's purpose-built for local-first P2P applications.

**Q: How does NAT traversal work?**
> Iroh uses n0's relay servers as STUN/TURN-like infrastructure. On startup, the endpoint publishes its address via DNS (Pkarr). When connecting, it tries direct connection first, falls back to relaying through the relay server. The EndpointAddr includes both the relay URL and any known direct addresses.

**Q: Why is presence separate from document sync?**
> Three reasons: (1) Frequency — cursors update 20x/sec, documents 5x/sec. (2) Ephemerality — cursor positions are meaningless after disconnect, but CRDT changes are permanent. (3) Efficiency — adding 20 cursor updates/second to the CRDT would bloat the document history.

### Architecture Questions

**Q: Explain the thread model.**
> Main thread handles UI (crossterm events + ratatui rendering) in a synchronous event loop. Sync thread runs a tokio async runtime for iroh networking. They communicate via `std::sync::mpsc` channels — `SyncCommand` (main→sync) and `SyncEvent` (sync→main). This avoids blocking the UI on network I/O.

**Q: How does the Automerge sync protocol work?**
> It's a message-based protocol. Each peer tracks which changes it has and which the other peer has (via "sync states"). When syncing, a peer sends a message containing the changes the other peer is missing. The other peer applies them and responds similarly. After a full exchange, both peers are in sync. This is bandwidth-efficient — only deltas are exchanged.

**Q: What's the ShapeView and why does it exist?**
> ShapeView is a read-only cache of deserialized shapes, rebuilt from the Automerge document after every mutation. Without it, rendering would require deserializing shapes from Automerge's internal format 60 times per second. The cache provides O(1) shape lookup by ID and spatial indexing for hit-testing.

**Q: How does session management work with sync?**
> Sessions are local-only organization. When you switch sessions, the current document is saved to disk, the new document is loaded, and the sync ticket changes. The sync infrastructure (iroh endpoint) stays alive — only the document being synced changes. Peers on the old session won't see the new session's changes.

### Code-Level Questions

**Q: Walk through what happens when a user draws a shape.**
>
> 1. Mouse down → `start_shape()` creates `ShapeState` with start position, checks for snap points. 2. Mouse move → `update_shape()` updates preview position, checks snap. 3. Mouse up → `commit_shape()` pushes undo checkpoint, creates `ShapeKind` via `doc.add_shape()` (Automerge transaction), rebuilds `ShapeView` cache, marks dirty. 4. Main loop detects dirty → sends `SyncCommand::SyncDoc` → sync thread pushes to peer.

**Q: How does the undo stack work across peers?**
> The undo/redo stacks are stored IN the Automerge document as lists of serialized shape snapshots. When User A undoes, the undo stack modification syncs to User B via normal CRDT merge. This means undo is "global" — undoing on one peer affects all peers. This is a deliberate design choice for collaborative editing.

**Q: What's the `LocalDocumentStore` and why does `update_from_app` not notify?**
> It's an in-memory implementation of the `DocumentStore` trait (from aspen-automerge). When the app pushes changes (`update_from_app`), it doesn't notify because that would create a feedback loop: app pushes → store notifies → app receives its own changes → pushes again. Only changes from the sync protocol (`save()`) trigger notifications.

### Rust-Specific Questions

**Q: Why use `std::sync::mpsc` instead of `tokio::sync::mpsc`?**
> The main thread is synchronous (crossterm event loop), not async. `std::sync::mpsc` works naturally in synchronous code. The sync thread uses `tokio::sync::mpsc` internally where async is needed, but the interface between threads uses `std::sync::mpsc` since the main thread isn't in an async context.

**Q: How is the mode system designed to avoid borrow checker issues?**
> The mode is taken out of `App` with `std::mem::take()` before calling `mode.handle_key(app, key)`. This avoids having `&mut mode` and `&mut app` simultaneously (since mode lives inside app). After the call, the mode (or its replacement) is put back.

**Q: What serialization formats are used and why?**
>
> - **Automerge binary**: For document storage/sync (compact, CRDT-native)
> - **MessagePack (rmp-serde)**: For presence messages (compact binary, fast)
> - **Postcard**: For ticket encoding (compact, no-std compatible)
> - **JSON (serde_json)**: For session metadata (human-readable config files)
> - **Base32**: For ticket strings (URL-safe, case-insensitive)
