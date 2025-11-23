# UI Fixes Applied

## 1. Admin Name Input

**Issue:** Admin user was hardcoded as "Admin" with no way to set a custom name.

**Fix:**
- Added "Your Name" input field to the create market form (HTML)
- Updated `CreateMarketRequest` struct to include `admin_name` field (Rust)
- Updated API route to use `req.admin_name` instead of hardcoded "Admin" (Rust)
- Updated `createMarket()` function to send admin name in API call (JavaScript)

**Files changed:**
- `ui/index.html` - Added admin-name input field
- `src/api/models.rs` - Added admin_name to CreateMarketRequest
- `src/api/routes.rs` - Use admin_name from request
- `ui/app.js` - Send admin_name in create market API call

## 2. Real-Time User Join Updates

**Issue:** When new users joined a market, the player list didn't update in real-time for existing users. Only refreshing the page would show new players.

**Fix:**
- Added new WebSocket message type `user_joined` with user details (Rust)
- Added broadcast when user joins market via API (Rust)
- Added WebSocket handler in UI to reload user list when receiving `user_joined` event (JavaScript)

**Files changed:**
- `src/api/models.rs` - Added `UserJoined` variant to `WsMessage` enum
- `src/api/routes.rs` - Broadcast user_joined event in join_market route
- `ui/app.js` - Handle user_joined message in handleWebSocketMessage()

## How to Test

1. **Start the server:**
   ```bash
   cargo run -- serve
   ```

2. **Test admin name:**
   - Open `http://localhost:3000`
   - Click "Create Market"
   - Enter your name (e.g., "Alice")
   - Create market
   - Verify your name appears in the player list (not "Admin")

3. **Test real-time user joins:**
   - Keep the admin window open (showing lobby/market)
   - Open a new incognito/private window
   - Navigate to `http://localhost:3000`
   - Click "Join Market"
   - Enter the invite code from admin window
   - Enter a name (e.g., "Bob")
   - Join market
   - **Verify:** Bob immediately appears in admin's player list without refreshing

## WebSocket Events Now Broadcasting

- `market_update` - Market status changes
- `user_joined` - New user joins (NEW!)
- `bet_created` - New bet created
- `bet_approved` - Bet approved by admin
- `wager_placed` - Wager placed on a bet
- `bet_resolved` - Bet resolved with outcome
- `market_status_changed` - Market status changed

All events trigger automatic UI updates via WebSocket for a seamless real-time experience.
