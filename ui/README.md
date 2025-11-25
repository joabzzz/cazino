# Cazino UI

Minimalist monochrome web interface for Cazino.

## Design

- **Font**: Inter (loaded from Google Fonts)
- **Color Scheme**: Pure monochrome (black, white, grays)
- **Style**: Crisp, minimalist, professional
- **Layout**: Mobile-responsive, single-page application

## Structure

- `index.html` - Main HTML structure with all screens and modals
- `style.css` - Complete styling with Inter font
- `app.js` - Client-side application with WebSocket and API integration

## Features

- **Landing Screen**: Create or join market
- **Lobby Screen**: Wait for players, show invite code (draft state)
- **Market Screen**: Active betting interface with tabs
  - Bets tab: View and place wagers on active bets
  - Leaderboard tab: See player rankings and profits
  - Reveal tab: View bets about you
- **Real-time Updates**: WebSocket integration for live odds
- **Hidden Bet Mechanic**: Bets about you appear as "[Hidden - about you]"
- **Admin Controls**: Create bets, approve bets, resolve bets, manage market

## Usage

The UI is served automatically by the Rust server. Just start the server and visit `http://localhost:3000`

```bash
cargo run -- serve
```

Then open your browser to `http://localhost:3000`

## Development

The UI uses vanilla JavaScript (no build step required). To make changes:

1. Edit HTML/CSS/JS files in the `ui/` directory
2. Refresh your browser to see changes
3. No compilation or bundling needed

## Configuration & Deployment

- `config.js` exposes a `window.CAZINO_CONFIG` object. In development it stays empty so the UI talks to the origin that served it (usually `cargo run -- serve`).
- `scripts/build_ui.sh` copies the contents of `ui/` into `dist/` and rewrites `config.js` with the values from `PUBLIC_API_BASE_URL` and `PUBLIC_WS_URL`. Use this script as the build command for Cloudflare Pages or any static host so secrets remain out of Git.

## API Integration

All API calls are made to `http://localhost:3000/api/*`
WebSocket connection to `ws://localhost:3000/ws`

See `app.js` for complete API integration details.
