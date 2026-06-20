# Orb - Developer Toolkit Orchestrator

Orb is a mock Terminal UI (TUI) application built in Rust using the [Ratatui](https://ratatui.rs/) framework and `crossterm`. It visualizes an AI-driven developer toolkit orchestrator, allowing users to discover command-line tools available on their system and combine them into automated workflows.

## Features

- **Marketplace**: Automatically discovers executables in your system's `PATH` and displays them in an interactive grid. Features a search bar and detailed view for selected packages.
- **Create Flow**: A prompt-based interface where users can describe a task. The system mocks an AI response, randomly selecting tools to generate an orchestrated pipeline.
- **Flow Diagram**: Visualizes the generated sequence of tools as a node-based pipeline (e.g., `[ tool1 ] ━━━▶ [ tool2 ] ━━━▶ [ tool3 ]`).
- **Flow Execution**: Simulates the step-by-step execution of the workflow, providing live visual feedback on the active task and overall status.

## Technologies Used

- **Rust** (Edition 2024)
- **Ratatui** for the terminal UI rendering
- **Crossterm** for cross-platform terminal input and event handling
- **Rand** for mocked AI flow generation

## Running the Project

```bash
cargo run
```

## Controls

- `1`, `2`, `3`: Navigate between Marketplace, Create Flow, and Active Flow screens.
- `h`, `j`, `k`, `l` / Arrow Keys: Navigate the Marketplace grid.
- `Space`: Select tools for manual flow creation.
- `c`: Create a manual flow from selected tools.
- `/`: Enter search mode.
- `Esc`: Cancel or go back.
- `Enter`: Confirm input, generate flow, or start execution.
- `q` / `F10`: Quit the application.
