# Snake Zoo

A community collection of Battlesnake opponents you can run locally with Docker.

Clone this repo, and you get a diverse field of snakes ready to battle -- no hunting down individual repos or figuring out how to build each one.

## Quick Start

A CLI tool (`sz`) for building and running snakes from this manifest is coming soon. For now, this repo defines the snake manifest format and includes seed entries.

## Manifest Format

Each snake is defined in a single TOML file under `snakes/`. The filename must match the snake's `slug` (e.g., `snakes/my-snake.toml` for a snake with `slug = "my-snake"`).

### Fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `name` | string | yes | -- | Display name of the snake |
| `slug` | string | yes | -- | URL-safe identifier; must match the filename |
| `repo` | string | yes | -- | Git repository URL containing the snake's source code |
| `dockerfile` | string | no | `"./Dockerfile"` | Path to the Dockerfile within the repo |
| `entrypoint` | string | no | `""` | URL path slug for multi-snake images (e.g., `my-snake` serves at `/my-snake/move`) |
| `port` | integer | no | `8000` | Port the snake listens on inside the container |
| `description` | string | no | `""` | Short description of the snake's strategy |

### `[meta]` Table (optional)

| Field | Type | Description |
|-------|------|-------------|
| `author` | string | Snake author's name or handle |
| `strategy` | string | Short label for the snake's strategy |
| `color` | string | Hex color code (e.g., `"#AA66CC"`) |
| `head` | string | Battlesnake head customization ID |
| `tail` | string | Battlesnake tail customization ID |

### `[env]` Table (optional)

Key-value pairs passed as environment variables to the container at runtime.

```toml
[env]
RUST_LOG = "info"
```

### Example

```toml
name = "Amphibious Arthur"
slug = "amphibious-arthur"
repo = "https://github.com/coreyja/battlesnake-rs"
dockerfile = "./Dockerfile"
entrypoint = "amphibious-arthur"
port = 8000
description = "Simulates opponent sprawl and scores positions recursively."

[meta]
author = "coreyja"
strategy = "Recursive Simulation"
color = "#AA66CC"
head = "trans-rights-scarf"
tail = "swirl"
```

## Adding a Snake

1. Create `snakes/<your-slug>.toml`
2. Fill in the required fields: `name`, `slug`, `repo`
3. Ensure the repo has a Dockerfile (or set `dockerfile` to the correct path)
4. For multi-snake images, set `entrypoint` to the URL path slug the snake serves on
5. Submit a PR

## Included Snakes

### battlesnake-rs (coreyja)

| Name | Slug | Strategy |
|------|------|----------|
| Amphibious Arthur | `amphibious-arthur` | Recursive Simulation |
| Bombastic Bob | `bombastic-bob` | Random Reasonable |
| Constant Carter | `constant-carter` | Always Right |
| Devious Devin | `devious-devin` | Paranoid Minimax |
| Eremetic Eric | `eremetic-eric` | Tail Chaser |
| Famished Frank | `famished-frank` | Grow & Corner |
| Gigantic George | `gigantic-george` | Hamiltonian Path |
| Hovering Hobbs | `hovering-hobbs` | Minimax + Flood Fill |
| Jump Flooding | `jump-flooding` | Area Control |
| Improbable Irene | `improbable-irene` | Monte Carlo Tree Search |

### Community Snakes

| Name | Slug | Language | Strategy | Author |
|------|------|----------|----------|--------|
| Snork | `snork` | Rust | Minimax + Flood Fill | wrenger |
| Untimely Neglected Wearable | `untimely-neglected-wearable` | Python | Rule-based + Flood Fill | altersaddle |
| Nessegrev Expert | `nessegrev-expert` | Java | Minimax + Payoff Matrix | Nettogrof |
| Nessegrev Flood | `nessegrev-flood` | Java | Flood Fill | Nettogrof |
| Nessegrev Right | `nessegrev-right` | Java | Right Turn Only | Nettogrof |
| The Very Hungry Caterpillar | `the-very-hungry-caterpillar` | C++ | Rule-based Pathfinding | TheApX |
| POOOOOOOOOOOOOG | `battlesnake-minimax` | JavaScript | Minimax + Alpha-Beta Pruning | calvinl4 |
| Robosnake | `robosnake` | Lua | Alpha-Beta Pruning | smallsco |
| Snek | `snek` | Ruby | Heuristic Scoring | jhawthorn |
