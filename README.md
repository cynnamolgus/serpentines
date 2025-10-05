# Serpentines

Serpentines is an engine for desktop customization. It brings playful, GPU‑accelerated effects to your desktop — starting with customizable cursor trails — and will grow into a platform for desktop buddies, animations, and User-defined overlays. It’s native, fast, and lightweight, powered by Rust.

Status: early preview. The immediate focus is a polished cursor‑trails experience. Next, we’ll add desktop buddies with configurable behaviors and state machines, plus shareable presets and overlay widgets.

## Crates
- `serpentines-core/`: engine logic (particles, presets, serialization)
- `serpentines-ui/`: eframe-driven main window UI logic
- `serpentines-platform/`: platform abstraction traits
- `serpentines-win/`: Windows implementations (overlay, input, tray)
- `serpentines-app/`: application entry point

## Build
```
cargo build
```

## Run
```
cargo run -p serpentines-app
```

## Roadmap
- **Cursor Trails 1.0**: Smooth, low‑latency trails with presets (color/shape/decay), per‑monitor support, and quick toggles.
- **Preset Ecosystem**: Import/export shareable trail packs (human‑readable format + optional assets).
- **Control Panel**: Native settings window for live tweaking and managing presets.
- **System Tray**: Lightweight tray with enable/disable, mode switching, and links to settings.
- **Desktop Buddies**: Animated characters with customizable state machines and interactions (click/drag, follow, idle behaviors).
- **Customizable Overlays**: Widgets and visual stickers anchored to the desktop with themes.
- **Cross‑Platform**: macOS/Linux support following the Windows-first foundation.
