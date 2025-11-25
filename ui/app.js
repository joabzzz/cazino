// Cazino Client Application

const runtimeConfig = window.CAZINO_CONFIG || {};
const httpOrigin = window.location.origin;
const wsOrigin = `${window.location.protocol === "https:" ? "wss" : "ws"}://${window.location.host}`;

function resolveBaseUrl(value, fallbackPath) {
  if (!value) {
    return `${httpOrigin}${fallbackPath}`;
  }

  if (value.startsWith("http://") || value.startsWith("https://")) {
    return value.replace(/\/$/, "");
  }

  const normalized = value.startsWith("/") ? value : `/${value}`;
  return `${httpOrigin}${normalized}`.replace(/\/$/, "");
}

function resolveWsUrl(value, fallbackPath) {
  if (!value) {
    return `${wsOrigin}${fallbackPath}`;
  }

  if (value.startsWith("ws://") || value.startsWith("wss://")) {
    return value;
  }

  const normalized = value.startsWith("/") ? value : `/${value}`;
  return `${wsOrigin}${normalized}`;
}

const API_BASE = resolveBaseUrl(runtimeConfig.apiBase, "/api");
const WS_URL = resolveWsUrl(runtimeConfig.wsUrl, "/ws");

// ===== State Management =====
const state = {
  market: null,
  user: null,
  inviteCode: null,
  bets: [],
  users: [],
  leaderboard: [],
  ws: null,
  currentBetId: null,
};

// Expose state to window for testing
window.state = state;

// ===== Utility Functions =====
function getDeviceFingerprint() {
  let fingerprint = localStorage.getItem("cazino_device_id");
  if (!fingerprint) {
    fingerprint = crypto.randomUUID();
    localStorage.setItem("cazino_device_id", fingerprint);
  }
  return fingerprint;
}

// ===== Recent Markets Functions =====
async function fetchRecentMarkets() {
  try {
    const deviceId = getDeviceFingerprint();
    const result = await apiCall(`/devices/${deviceId}/markets`);
    return result.markets || [];
  } catch (error) {
    console.error("Failed to fetch recent markets:", error);
    return [];
  }
}

async function renderRecentMarkets() {
  const container = document.getElementById("recent-markets");
  const list = document.getElementById("recent-markets-list");

  // Show loading state
  container.style.display = "block";
  list.innerHTML = '<div class="loading-state">Loading recent markets...</div>';

  const markets = await fetchRecentMarkets();

  if (markets.length === 0) {
    container.style.display = "none";
    return;
  }

  container.style.display = "block";
  list.innerHTML = markets
    .map(
      (item) => `
      <button class="recent-market-item" data-code="${item.market.invite_code}">
        <div class="recent-market-name">${item.market.name}</div>
        <div class="recent-market-info">
          <span class="recent-market-code">${item.market.invite_code}</span>
          <span class="recent-market-user">@${item.user.display_name}</span>
          <span class="recent-market-status">${item.market.status}</span>
        </div>
      </button>
    `,
    )
    .join("");

  // Add click handlers
  list.querySelectorAll(".recent-market-item").forEach((item) => {
    item.addEventListener("click", () => {
      const code = item.dataset.code;
      rejoinMarket(code);
    });
  });
}

async function rejoinMarket(inviteCode) {
  try {
    const result = await apiCall(`/markets/${inviteCode}/join`, {
      method: "POST",
      body: JSON.stringify({
        display_name: "rejoining", // Will be ignored for existing device
        avatar: "👤",
        device_id: getDeviceFingerprint(),
      }),
    });

    state.market = result.market;
    state.user = result.user;
    state.inviteCode = inviteCode;

    connectWebSocket();

    if (state.market.status === "draft") {
      showLobby();
    } else {
      showMarket();
    }
  } catch (error) {
    showError(error.message);
    // Refresh the list in case the market was deleted
    renderRecentMarkets();
  }
}

function showScreen(screenId) {
  document
    .querySelectorAll(".screen")
    .forEach((s) => s.classList.remove("active"));
  document.getElementById(screenId).classList.add("active");
}

function closeModal(modalId) {
  document.getElementById(modalId).classList.remove("active");
}

function showModal(modalId) {
  document.getElementById(modalId).classList.add("active");
}

function showError(message) {
  alert(message); // In production, use a nicer notification system
}

function formatBalance(balance) {
  return `$${balance.toLocaleString()}`;
}

// ===== API Functions =====
async function apiCall(endpoint, options = {}) {
  try {
    const response = await fetch(`${API_BASE}${endpoint}`, {
      ...options,
      headers: {
        "Content-Type": "application/json",
        ...options.headers,
      },
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.error || "API request failed");
    }

    // Some endpoints return empty responses
    const text = await response.text();
    if (!text || text.trim() === "") {
      return null;
    }

    return JSON.parse(text);
  } catch (error) {
    console.error("API Error:", error);
    throw error;
  }
}

// ===== WebSocket Functions =====
function connectWebSocket() {
  if (state.ws) {
    state.ws.close();
  }

  if (!state.market || !state.market.id) {
    console.error("Cannot connect WebSocket: no market ID");
    return;
  }

  const wsUrl = `${WS_URL}/${state.market.id}`;
  console.log("Connecting to WebSocket:", wsUrl);
  state.ws = new WebSocket(wsUrl);

  state.ws.onopen = () => {
    console.log("WebSocket connected");
    if (state.market) {
      state.ws.send(
        JSON.stringify({
          type: "subscribe",
          market_id: state.market.id,
        }),
      );
    }
  };

  state.ws.onmessage = (event) => {
    const message = JSON.parse(event.data);
    handleWebSocketMessage(message);
  };

  state.ws.onerror = (error) => {
    console.error("WebSocket error:", error);
  };

  state.ws.onclose = () => {
    console.log("WebSocket disconnected");
    // Reconnect after 3 seconds
    setTimeout(connectWebSocket, 3000);
  };
}

function handleWebSocketMessage(message) {
  console.log("WS Message:", message);

  switch (message.type) {
    case "market_update":
      state.market = message.market;
      updateMarketDisplay();
      break;

    case "user_joined":
      loadUsers();
      break;

    case "market_opened":
      // Market has been opened, transition from lobby to market view
      loadMarket();
      showMarket();
      break;

    case "bet_created":
      loadBets();
      break;

    case "bet_approved":
      loadBets();
      break;

    case "wager_placed":
      loadBets();
      updateUserBalance();
      break;

    case "bet_resolved":
      loadBets();
      updateUserBalance();
      break;

    case "market_status_changed":
      // Reload market data
      loadMarket().then(() => {
        // If market is now open and we're in lobby, transition to market screen
        if (
          state.market.status === "open" &&
          document.getElementById("lobby-screen").classList.contains("active")
        ) {
          showMarket();
        }
      });
      break;
  }
}

// ===== Market Functions =====
async function createMarket() {
  const adminName = document
    .getElementById("admin-name")
    .value.trim()
    .toLowerCase();
  const name = document.getElementById("market-name").value;
  const duration = parseInt(document.getElementById("duration").value);
  const startingBalance = parseInt(
    document.getElementById("starting-balance").value,
  );

  try {
    const result = await apiCall("/markets", {
      method: "POST",
      body: JSON.stringify({
        name,
        admin_name: adminName,
        duration_hours: duration,
        starting_balance: startingBalance,
        device_id: getDeviceFingerprint(),
      }),
    });

    state.market = result.market;
    state.user = result.user;
    state.inviteCode = result.invite_code;

    connectWebSocket();
    showLobby();
  } catch (error) {
    showError(error.message);
  }
}

async function joinMarket() {
  const inviteCode = document.getElementById("invite-code").value.toUpperCase();
  const displayName = document
    .getElementById("display-name")
    .value.trim()
    .toLowerCase();

  try {
    const result = await apiCall(`/markets/${inviteCode}/join`, {
      method: "POST",
      body: JSON.stringify({
        display_name: displayName,
        avatar: "👤",
        device_id: getDeviceFingerprint(),
      }),
    });

    state.market = result.market;
    state.user = result.user;
    state.inviteCode = inviteCode;

    connectWebSocket();

    if (state.market.status === "draft") {
      showLobby();
    } else {
      showMarket();
    }
  } catch (error) {
    showError(error.message);
  }
}

async function loadMarket() {
  try {
    const market = await apiCall(`/markets/${state.market.id}`);
    state.market = market;
    updateMarketDisplay();
  } catch (error) {
    console.error("Failed to load market:", error);
  }
}

async function openMarket() {
  try {
    await apiCall(`/markets/${state.market.id}/open/${state.user.id}`, {
      method: "POST",
    });

    await loadMarket();
    showMarket();
  } catch (error) {
    showError(error.message);
  }
}

async function closeMarket() {
  if (
    !confirm("Are you sure you want to close this market? Betting will end.")
  ) {
    return;
  }

  try {
    await apiCall(`/markets/${state.market.id}/close/${state.user.id}`, {
      method: "POST",
    });

    await loadMarket();
    alert("Market closed successfully!");
  } catch (error) {
    showError(error.message);
  }
}

async function deleteMarket() {
  if (
    !confirm(
      "Are you sure you want to DELETE this market? This cannot be undone!",
    )
  ) {
    return;
  }

  if (
    !confirm(
      "This will delete ALL bets, wagers, and user data. Are you REALLY sure?",
    )
  ) {
    return;
  }

  try {
    await apiCall(`/markets/${state.market.id}/delete/${state.user.id}`, {
      method: "POST",
    });

    // Disconnect websocket
    if (state.ws) {
      state.ws.close();
      state.ws = null;
    }

    // Clear state
    state.market = null;
    state.user = null;
    state.inviteCode = null;
    state.bets = [];
    state.users = [];
    state.leaderboard = [];

    alert("Market deleted successfully!");
    showScreen("landing-screen");
  } catch (error) {
    showError(error.message);
  }
}

async function loadUsers() {
  try {
    const result = await apiCall(`/markets/${state.market.id}/leaderboard`);
    state.users = result.users.map((u) => u.user);
    renderPlayerList();
  } catch (error) {
    console.error("Failed to load users:", error);
  }
}

// ===== Bet Functions =====
async function loadBets() {
  try {
    const bets = await apiCall(
      `/markets/${state.market.id}/bets/${state.user.id}`,
    );
    state.bets = bets;
    renderBets();
    renderFeed();
  } catch (error) {
    console.error("Failed to load bets:", error);
  }
}

async function createBet() {
  const description = document.getElementById("bet-description").value;
  const odds = "1:1"; // Default to even odds, actual odds determined by betting pool
  const wager = parseInt(document.getElementById("opening-wager").value);
  const hideFromSubject = document.getElementById("hide-from-subject").checked;

  // Parse @username from description
  const mentionMatch = description.match(/@([a-zA-Z0-9_]+)/);
  if (!mentionMatch) {
    showError("Please include @username in the bet description");
    return;
  }

  const username = mentionMatch[1].toLowerCase();

  // Find user by username
  const subjectUser = state.users.find(
    (u) => u.display_name.toLowerCase() === username,
  );
  if (!subjectUser) {
    showError(`User @${username} not found in this market`);
    return;
  }

  try {
    await apiCall(`/markets/${state.market.id}/bets/${state.user.id}`, {
      method: "POST",
      body: JSON.stringify({
        subject_user_id: subjectUser.id,
        description,
        initial_odds: odds,
        opening_wager: wager,
        hide_from_subject: hideFromSubject,
      }),
    });

    closeModal("create-bet-modal");
    document.getElementById("create-bet-form").reset();
    await loadBets();
    await updateUserBalance();
  } catch (error) {
    showError(error.message);
  }
}

async function placeWager() {
  const side = document.querySelector('input[name="side"]:checked').value;
  const amount = parseInt(document.getElementById("wager-amount").value);

  try {
    await apiCall(`/bets/${state.currentBetId}/wager/${state.user.id}`, {
      method: "POST",
      body: JSON.stringify({
        side,
        amount,
      }),
    });

    closeModal("wager-modal");
    document.getElementById("wager-form").reset();
    await loadBets();
    await updateUserBalance();
  } catch (error) {
    showError(error.message);
  }
}

async function resolveBet(betId, outcome) {
  try {
    await apiCall(`/bets/${betId}/resolve/${state.user.id}`, {
      method: "POST",
      body: JSON.stringify({
        outcome,
      }),
    });

    await loadBets();
    await updateUserBalance();
  } catch (error) {
    showError(error.message);
  }
}

// ===== Leaderboard Functions =====
async function loadLeaderboard() {
  try {
    const result = await apiCall(`/markets/${state.market.id}/leaderboard`);
    state.leaderboard = result.users;
    renderLeaderboard();
  } catch (error) {
    console.error("Failed to load leaderboard:", error);
  }
}

// ===== Reveal Functions =====
async function loadReveal() {
  try {
    const result = await apiCall(`/users/${state.user.id}/reveal`);
    renderReveal(result.bets);
  } catch (error) {
    console.error("Failed to load reveal:", error);
  }
}

// ===== Update Functions =====
async function updateUserBalance() {
  try {
    const result = await apiCall(`/markets/${state.market.id}/leaderboard`);
    const currentUser = result.users.find((u) => u.user.id === state.user.id);
    if (currentUser) {
      state.user.balance = currentUser.user.balance;
      document.getElementById("user-balance").textContent = formatBalance(
        state.user.balance,
      );
    }
  } catch (error) {
    console.error("Failed to update balance:", error);
  }
}

// ===== Render Functions =====
function showLobby() {
  document.getElementById("lobby-market-name").textContent = state.market.name;
  document.getElementById("lobby-status").textContent = state.market.status;
  document.getElementById("lobby-invite-code").textContent = state.inviteCode;

  if (state.user.is_admin) {
    document.getElementById("admin-controls").style.display = "block";
  }

  loadUsers();
  showScreen("lobby-screen");
}

function showMarket() {
  document.getElementById("market-name-display").textContent =
    state.market.name;
  document.getElementById("market-status").textContent = state.market.status;
  document.getElementById("user-balance").textContent = formatBalance(
    state.user.balance,
  );
  document.getElementById("market-invite-code").textContent = state.inviteCode;

  if (state.user.is_admin) {
    document
      .querySelectorAll(".admin-section")
      .forEach((el) => (el.style.display = "block"));
    // Show Admin tab for admins
    document.getElementById("admin-tab-btn").style.display = "block";
  } else {
    document.getElementById("admin-tab-btn").style.display = "none";
  }

  loadBets();
  loadLeaderboard();
  loadUsers();
  showScreen("market-screen");
}

function updateMarketDisplay() {
  if (document.getElementById("lobby-screen").classList.contains("active")) {
    document.getElementById("lobby-status").textContent = state.market.status;
    loadUsers();
  } else if (
    document.getElementById("market-screen").classList.contains("active")
  ) {
    document.getElementById("market-status").textContent = state.market.status;
  }
}

function renderPlayerList() {
  const list = document.getElementById("player-list");
  const count = document.getElementById("player-count");

  count.textContent = state.users.length;

  list.innerHTML = state.users
    .map(
      (user) => `
        <li>
            <span class="player-name">@${user.display_name}</span>
            ${user.is_admin ? '<span class="player-badge">Admin</span>' : ""}
        </li>
    `,
    )
    .join("");
}

function renderBets() {
  const activeBets = state.bets.filter((bet) => bet.status === "active");
  const list = document.getElementById("bets-list");

  if (activeBets.length === 0) {
    list.innerHTML =
      '<div class="empty-state">No active bets yet. Create one to get started!</div>';
    return;
  }

  list.innerHTML = activeBets
    .map(
      (bet) => `
        <div class="bet-card ${bet.is_hidden ? "hidden" : ""}"">
            <div class="bet-header">
                <div>
                    <div class="bet-description">${bet.description || "[Hidden - about you]"}</div>
                    ${bet.subject_user_id ? `<div class="bet-subject">About ${getUserName(bet.subject_user_id)}</div>` : ""}
                </div>
                <span class="bet-status-badge ${bet.status}">${bet.status}</span>
            </div>

            ${
              !bet.is_hidden
                ? `
                <div class="bet-pools">
                    <div class="pool-info">
                        <div class="pool-label">YES</div>
                        <div class="pool-value">${bet.yes_pool}</div>
                        <div class="pool-prob">${(calculateProbability(bet.yes_pool, bet.no_pool) * 100).toFixed(1)}%</div>
                    </div>
                    <div class="pool-info">
                        <div class="pool-label">NO</div>
                        <div class="pool-value">${bet.no_pool}</div>
                        <div class="pool-prob">${((1 - calculateProbability(bet.yes_pool, bet.no_pool)) * 100).toFixed(1)}%</div>
                    </div>
                </div>

                <div class="bet-actions">
                    <button class="btn btn-small" onclick="openWagerModal('${bet.id}')">Place Wager</button>
                    <button class="btn btn-small" onclick="openBetDetailView('${bet.id}')">View Details</button>
                </div>
            `
                : ""
            }
        </div>
    `,
    )
    .join("");
}

function renderLeaderboard() {
  const list = document.getElementById("leaderboard-list");

  if (state.leaderboard.length === 0) {
    list.innerHTML = '<div class="empty-state">No players yet</div>';
    return;
  }

  // Sort by total value (balance + profit) descending
  const sortedLeaderboard = [...state.leaderboard].sort((a, b) => {
    const totalA = a.user.balance + a.profit;
    const totalB = b.user.balance + b.profit;
    return totalB - totalA;
  });

  list.innerHTML = sortedLeaderboard
    .map((item, index) => {
      const totalValue = item.user.balance + item.profit;
      return `
        <div class="leaderboard-item">
            <div class="leaderboard-rank">#${index + 1}</div>
            <div class="leaderboard-name">@${item.user.display_name}</div>
            <div class="leaderboard-values">
                <div class="leaderboard-total">${formatBalance(totalValue)}</div>
                <div class="leaderboard-breakdown">
                    <span class="leaderboard-cash" title="Cash on hand">${formatBalance(item.user.balance)}</span>
                    <span class="leaderboard-profit ${item.profit >= 0 ? "positive" : "negative"}" title="Profit/Loss from bets">
                        ${item.profit >= 0 ? "+" : ""}${formatBalance(item.profit)}
                    </span>
                </div>
            </div>
        </div>
    `;
    })
    .join("");
}

function renderReveal(bets) {
  const list = document.getElementById("reveal-list");

  if (bets.length === 0) {
    list.innerHTML = '<div class="empty-state">No bets about you yet</div>';
    return;
  }

  list.innerHTML = bets
    .map(
      (bet) => `
        <div class="bet-card">
            <div class="bet-header">
                <div>
                    <div class="bet-description">${bet.description}</div>
                    <div class="bet-subject">Created by ${getUserName(bet.created_by)}</div>
                </div>
                <span class="bet-status-badge ${bet.status}">${bet.status}</span>
            </div>

            <div class="bet-pools">
                <div class="pool-info">
                    <div class="pool-label">YES</div>
                    <div class="pool-value">${bet.yes_pool}</div>
                    <div class="pool-prob">${(calculateProbability(bet.yes_pool, bet.no_pool) * 100).toFixed(1)}%</div>
                </div>
                <div class="pool-info">
                    <div class="pool-label">NO</div>
                    <div class="pool-value">${bet.no_pool}</div>
                    <div class="pool-prob">${((1 - calculateProbability(bet.yes_pool, bet.no_pool)) * 100).toFixed(1)}%</div>
                </div>
            </div>
        </div>
    `,
    )
    .join("");
}

function renderFeed() {
  const list = document.getElementById("feed-list");

  // Generate feed events from bets
  const feedEvents = [];

  state.bets.forEach((bet) => {
    // Add bet creation event
    const isBetAboutMe = bet.subject_user_id === state.user.id;
    const betDescription =
      isBetAboutMe && !bet.is_hidden
        ? bet.description
        : isBetAboutMe
          ? "[Hidden bet about you]"
          : bet.description;

    feedEvents.push({
      type: "bet_created",
      timestamp: new Date(bet.created_at),
      creator: getUserName(bet.created_by),
      description: betDescription,
      amount: bet.yes_pool + bet.no_pool,
      bet: bet,
    });

    // Add resolution event if resolved
    if (bet.status === "resolved_yes" || bet.status === "resolved_no") {
      feedEvents.push({
        type: "bet_resolved",
        timestamp: new Date(bet.resolved_at),
        bet: bet,
        outcome: bet.status === "resolved_yes" ? "YES" : "NO",
        description: betDescription,
      });
    }
  });

  // Sort by timestamp (newest first)
  feedEvents.sort((a, b) => b.timestamp - a.timestamp);

  if (feedEvents.length === 0) {
    list.innerHTML = '<div class="empty-state">No activity yet</div>';
    renderTicker([]);
    return;
  }

  list.innerHTML = feedEvents
    .map((event) => {
      if (event.type === "bet_created") {
        return `
        <div class="feed-item">
          <div class="feed-item-header">
            ${highlightMentionsAndAmounts(event.creator + " placed " + formatBalance(event.amount) + " on " + (event.description && event.description.includes("[Hidden") ? '<span class="feed-masked">[Hidden bet]</span>' : event.description || ""))}
          </div>
          <div class="feed-item-details">${formatTimestamp(event.timestamp)}</div>
        </div>
      `;
      } else if (event.type === "bet_resolved") {
        return `
        <div class="feed-item">
          <div class="feed-item-header">
            ${highlightMentionsAndAmounts(getUserName(event.bet.created_by) + " validated " + event.description + " - " + event.outcome)}
          </div>
          <div class="feed-item-details">${formatTimestamp(event.timestamp)}</div>
          ${renderWinnings(event.bet)}
        </div>
      `;
      }
    })
    .join("");

  // Update ticker
  renderTicker(feedEvents.slice(0, 5));
}

function highlightMentionsAndAmounts(text) {
  // Highlight @mentions
  text = text.replace(
    /@([a-zA-Z0-9_]+)/g,
    '<span class="user-mention">@$1</span>',
  );
  // Highlight amounts
  text = text.replace(/\$[\d,]+/g, '<span class="amount">$&</span>');
  return text;
}

function renderTicker(feedEvents) {
  const ticker = document.getElementById("ticker-content");

  if (feedEvents.length === 0) {
    ticker.innerHTML = "No recent activity";
    return;
  }

  const tickerItems = feedEvents
    .map((event) => {
      if (event.type === "bet_created") {
        return highlightMentionsAndAmounts(
          event.creator +
            " placed " +
            formatBalance(event.amount) +
            " on " +
            (event.description.includes("[Hidden")
              ? "[Hidden bet]"
              : event.description),
        );
      } else if (event.type === "bet_resolved") {
        return highlightMentionsAndAmounts(
          getUserName(event.bet.created_by) +
            " validated " +
            event.description +
            " - " +
            event.outcome,
        );
      }
    })
    .join(" • ");

  ticker.innerHTML = tickerItems + " • " + tickerItems; // Duplicate for seamless loop
}

function renderWinnings(bet) {
  // This would need actual wager data from the backend
  // For now, show a placeholder
  return `
    <div class="feed-item-winnings">
      <div class="winner">
        <span>Payouts calculated</span>
      </div>
    </div>
  `;
}

function formatTimestamp(date) {
  const now = new Date();
  const diff = now - date;
  const minutes = Math.floor(diff / 60000);
  const hours = Math.floor(diff / 3600000);

  if (minutes < 1) return "Just now";
  if (minutes < 60) return `${minutes}m ago`;
  if (hours < 24) return `${hours}h ago`;
  return date.toLocaleDateString();
}

// ===== Helper Functions =====
function getUserName(userId) {
  const user = state.users.find((u) => u.id === userId);
  return user ? `@${user.display_name}` : "@unknown";
}

function calculateProbability(yesPool, noPool) {
  const total = yesPool + noPool;
  if (total === 0) return 0.5;
  return yesPool / total;
}

function openBetDetailView(betId) {
  state.currentBetId = betId;
  const bet = state.bets.find((b) => b.id === betId);

  document.getElementById("bet-detail-title").textContent = bet.description;

  // Hide description element if no resolution criteria
  const descElement = document.getElementById("bet-detail-description");
  if (bet.resolution_criteria && bet.resolution_criteria.trim()) {
    descElement.textContent = bet.resolution_criteria;
    descElement.style.display = "block";
  } else {
    descElement.style.display = "none";
  }

  const probability = calculateProbability(bet.yes_pool, bet.no_pool);
  document.getElementById("bet-detail-prob").textContent =
    `${(probability * 100).toFixed(1)}%`;

  document.getElementById("bet-detail-yes-pool").textContent = formatBalance(
    bet.yes_pool,
  );
  document.getElementById("bet-detail-no-pool").textContent = formatBalance(
    bet.no_pool,
  );

  // Show admin actions if user is admin
  if (state.user.is_admin) {
    document.getElementById("bet-detail-admin-actions").style.display = "flex";
  }

  // Draw the odds graph
  drawOddsGraph(bet.yes_pool, bet.no_pool);

  showModal("bet-detail-modal");
}

function drawOddsGraph(yesPool, noPool) {
  const canvas = document.getElementById("bet-detail-chart");
  const ctx = canvas.getContext("2d");

  // Clear canvas
  ctx.clearRect(0, 0, canvas.width, canvas.height);

  const total = yesPool + noPool;
  const yesPercent = total === 0 ? 0.5 : yesPool / total;
  const noPercent = 1 - yesPercent;

  // Draw horizontal bar chart
  const width = canvas.width;
  const height = canvas.height;
  const barHeight = 60;
  const yPosition = (height - barHeight) / 2;

  // Draw YES bar (left side)
  ctx.fillStyle = "#000000";
  const yesBarWidth = width * yesPercent;
  ctx.fillRect(0, yPosition, yesBarWidth, barHeight);

  // Draw NO bar (right side)
  ctx.fillStyle = "#6a6a6a";
  const noBarWidth = width * noPercent;
  ctx.fillRect(yesBarWidth, yPosition, noBarWidth, barHeight);

  // Draw labels on bars
  ctx.fillStyle = "#ffffff";
  ctx.font = "14px Inter";
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";

  // YES label and percentage
  if (yesPercent > 0.15) {
    ctx.fillText("YES", yesBarWidth / 2, yPosition + barHeight / 2 - 10);
    ctx.font = "16px Inter";
    ctx.fillText(
      `${(yesPercent * 100).toFixed(1)}%`,
      yesBarWidth / 2,
      yPosition + barHeight / 2 + 10,
    );
  } else if (yesPercent > 0) {
    // If bar too small, draw outside
    ctx.fillStyle = "#000000";
    ctx.textAlign = "left";
    ctx.fillText(
      `YES ${(yesPercent * 100).toFixed(1)}%`,
      yesBarWidth + 10,
      yPosition + barHeight / 2,
    );
  }

  // NO label and percentage
  ctx.fillStyle = "#ffffff";
  ctx.font = "14px Inter";
  ctx.textAlign = "center";
  if (noPercent > 0.15) {
    ctx.fillText(
      "NO",
      yesBarWidth + noBarWidth / 2,
      yPosition + barHeight / 2 - 10,
    );
    ctx.font = "16px Inter";
    ctx.fillText(
      `${(noPercent * 100).toFixed(1)}%`,
      yesBarWidth + noBarWidth / 2,
      yPosition + barHeight / 2 + 10,
    );
  } else if (noPercent > 0) {
    // If bar too small, draw outside
    ctx.fillStyle = "#6a6a6a";
    ctx.textAlign = "right";
    ctx.fillText(
      `NO ${(noPercent * 100).toFixed(1)}%`,
      yesBarWidth - 10,
      yPosition + barHeight / 2,
    );
  }
}

function openWagerModalFromDetail() {
  closeModal("bet-detail-modal");
  openWagerModal(state.currentBetId);
}

function resolveBetFromDetail(outcome) {
  closeModal("bet-detail-modal");
  resolveBet(state.currentBetId, outcome);
}

function openWagerModal(betId) {
  state.currentBetId = betId;
  const bet = state.bets.find((b) => b.id === betId);

  document.getElementById("wager-bet-title").textContent = bet.description;
  document.getElementById("wager-bet-description").textContent =
    bet.resolution_criteria;
  document.getElementById("wager-yes-pool").textContent = formatBalance(
    bet.yes_pool,
  );
  document.getElementById("wager-no-pool").textContent = formatBalance(
    bet.no_pool,
  );
  document.getElementById("wager-yes-prob").textContent =
    `${(calculateProbability(bet.yes_pool, bet.no_pool) * 100).toFixed(1)}%`;
  document.getElementById("wager-no-prob").textContent =
    `${((1 - calculateProbability(bet.yes_pool, bet.no_pool)) * 100).toFixed(1)}%`;

  showModal("wager-modal");
}

// ===== Event Handlers =====
document.getElementById("create-market-btn").addEventListener("click", () => {
  showScreen("create-market-screen");
});

document.getElementById("join-market-btn").addEventListener("click", () => {
  renderRecentMarkets();
  showScreen("join-market-screen");
});

document
  .getElementById("create-market-form")
  .addEventListener("submit", (e) => {
    e.preventDefault();
    createMarket();
  });

document.getElementById("join-market-form").addEventListener("submit", (e) => {
  e.preventDefault();
  joinMarket();
});

document.getElementById("copy-invite-btn").addEventListener("click", () => {
  navigator.clipboard.writeText(state.inviteCode);
  alert("Invite code copied to clipboard!");
});

document
  .getElementById("copy-market-invite-btn")
  .addEventListener("click", () => {
    navigator.clipboard.writeText(state.inviteCode);
    alert("Invite code copied to clipboard!");
  });

document.getElementById("open-market-btn").addEventListener("click", () => {
  openMarket();
});

document
  .getElementById("delete-market-lobby-btn")
  .addEventListener("click", () => {
    deleteMarket();
  });

document.getElementById("close-market-btn").addEventListener("click", () => {
  closeMarket();
});

document.getElementById("delete-market-btn").addEventListener("click", () => {
  deleteMarket();
});

document.getElementById("create-bet-btn").addEventListener("click", () => {
  showModal("create-bet-modal");
});

document.getElementById("create-bet-form").addEventListener("submit", (e) => {
  e.preventDefault();
  createBet();
});

document.getElementById("wager-form").addEventListener("submit", (e) => {
  e.preventDefault();
  placeWager();
});

// Tab switching
document.querySelectorAll(".tab").forEach((tab) => {
  tab.addEventListener("click", () => {
    const tabName = tab.dataset.tab;

    // Update tab buttons
    document
      .querySelectorAll(".tab")
      .forEach((t) => t.classList.remove("active"));
    tab.classList.add("active");

    // Update tab content
    document
      .querySelectorAll(".tab-content")
      .forEach((c) => c.classList.remove("active"));
    document.getElementById(`${tabName}-tab`).classList.add("active");

    // Load data if needed
    if (tabName === "leaderboard") {
      loadLeaderboard();
    } else if (tabName === "reveal") {
      loadReveal();
    }
  });
});

// Close modals when clicking outside
window.addEventListener("click", (e) => {
  if (e.target.classList.contains("modal")) {
    e.target.classList.remove("active");
  }
});

console.log("Cazino client loaded");
