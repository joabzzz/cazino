import { test, expect, type Page } from '@playwright/test';

const LOCAL_URL = 'http://localhost:3333';

/**
 * WebSocket-focused E2E tests
 *
 * These tests explicitly verify that WebSocket real-time updates are working correctly.
 * This is critical for Cloudflare deployment where WebSocket routing can be tricky.
 */

interface WebSocketMessage {
  type: string;
  [key: string]: any;
}

/**
 * Helper to capture WebSocket messages on a page
 */
async function setupWebSocketListener(page: Page): Promise<() => Promise<WebSocketMessage[]>> {
  await page.addInitScript(() => {
    // @ts-ignore - injecting into page context
    window.__wsMessages = [];

    const originalWebSocket = window.WebSocket;
    // @ts-ignore
    window.WebSocket = class extends originalWebSocket {
      constructor(url: string | URL, protocols?: string | string[]) {
        super(url, protocols);

        this.addEventListener('message', (event) => {
          try {
            const data = JSON.parse(event.data);
            // @ts-ignore
            window.__wsMessages.push(data);
          } catch (e) {
            // Not JSON, ignore
          }
        });
      }
    };
  });

  return async () => {
    return await page.evaluate(() => {
      // @ts-ignore
      return window.__wsMessages || [];
    });
  };
}

/**
 * Helper to wait for a specific WebSocket message type
 */
async function waitForWebSocketMessage(
  page: Page,
  messageType: string,
  timeout = 5000
): Promise<WebSocketMessage> {
  const startTime = Date.now();

  while (Date.now() - startTime < timeout) {
    const messages = await page.evaluate(() => {
      // @ts-ignore
      return window.__wsMessages || [];
    });

    const message = messages.find((msg: WebSocketMessage) => msg.type === messageType);
    if (message) {
      return message;
    }

    await page.waitForTimeout(100);
  }

  throw new Error(`WebSocket message '${messageType}' not received within ${timeout}ms`);
}

test('websocket connection established on market join', async ({ browser, baseURL }) => {
  const url = baseURL ?? LOCAL_URL;
  const context = await browser.newContext();
  const page = await context.newPage();

  const getMessages = await setupWebSocketListener(page);

  await page.goto(url);
  await page.getByRole('button', { name: 'Create Market' }).click();
  await page.fill('#admin-name', 'ws-test-admin');
  await page.fill('#market-name', 'ws-test-market');
  await page.fill('#duration', '24');
  await page.fill('#starting-balance', '1000');
  await page.getByRole('button', { name: 'Create' }).click();

  // Wait for lobby screen to be active
  await page.waitForSelector('#lobby-screen.active');

  // Check that WebSocket connected
  const wsConnected = await page.evaluate(() => {
    // @ts-ignore
    return window.state?.ws?.readyState === WebSocket.OPEN;
  });

  expect(wsConnected).toBe(true);

  await context.close();
});

test('websocket broadcasts user_joined event', async ({ browser, baseURL }) => {
  const url = baseURL ?? LOCAL_URL;

  // Create market with admin
  const adminContext = await browser.newContext();
  const adminPage = await adminContext.newPage();
  const getAdminMessages = await setupWebSocketListener(adminPage);

  await adminPage.goto(url);
  await adminPage.getByRole('button', { name: 'Create Market' }).click();
  await adminPage.fill('#admin-name', 'admin');
  await adminPage.fill('#market-name', 'broadcast-test');
  await adminPage.fill('#duration', '24');
  await adminPage.fill('#starting-balance', '1000');
  await adminPage.getByRole('button', { name: 'Create' }).click();

  await adminPage.waitForSelector('#lobby-screen.active');
  const inviteCode = (await adminPage.locator('#lobby-invite-code').textContent())?.trim();
  expect(inviteCode).toBeTruthy();

  // Clear any messages so far
  await adminPage.evaluate(() => {
    // @ts-ignore
    window.__wsMessages = [];
  });

  // Second user joins
  const userContext = await browser.newContext();
  const userPage = await userContext.newPage();

  await userPage.goto(url);
  await userPage.getByRole('button', { name: 'Join Market' }).click();
  await userPage.fill('#invite-code', inviteCode!);
  await userPage.fill('#display-name', 'bob');
  await userPage.getByRole('button', { name: 'Join' }).click();

  // Admin should receive user_joined WebSocket message
  const userJoinedMsg = await waitForWebSocketMessage(adminPage, 'user_joined', 10000);
  expect(userJoinedMsg).toBeTruthy();
  expect(userJoinedMsg.type).toBe('user_joined');

  // Verify bob appears in admin's player list (proves UI updated from WS message)
  await expect(adminPage.locator('#player-list')).toContainText('@bob');

  await Promise.all([adminContext.close(), userContext.close()]);
});

test('websocket broadcasts market_opened event', async ({ browser, baseURL }) => {
  const url = baseURL ?? LOCAL_URL;

  // Create market
  const adminContext = await browser.newContext();
  const adminPage = await adminContext.newPage();
  const getAdminMessages = await setupWebSocketListener(adminPage);

  await adminPage.goto(url);
  await adminPage.getByRole('button', { name: 'Create Market' }).click();
  await adminPage.fill('#admin-name', 'admin');
  await adminPage.fill('#market-name', 'open-test');
  await adminPage.fill('#duration', '24');
  await adminPage.fill('#starting-balance', '1000');
  await adminPage.getByRole('button', { name: 'Create' }).click();

  await adminPage.waitForSelector('#lobby-screen.active');
  const inviteCode = (await adminPage.locator('#lobby-invite-code').textContent())?.trim();

  // User joins and waits in lobby
  const userContext = await browser.newContext();
  const userPage = await userContext.newPage();
  const getUserMessages = await setupWebSocketListener(userPage);

  await userPage.goto(url);
  await userPage.getByRole('button', { name: 'Join Market' }).click();
  await userPage.fill('#invite-code', inviteCode!);
  await userPage.fill('#display-name', 'carol');
  await userPage.getByRole('button', { name: 'Join' }).click();

  await userPage.waitForSelector('#lobby-screen.active');

  // Clear messages
  await userPage.evaluate(() => {
    // @ts-ignore
    window.__wsMessages = [];
  });

  // Admin opens market
  await adminPage.getByRole('button', { name: 'Open Market' }).click();
  await adminPage.waitForSelector('#market-screen.active');

  // User should receive market_opened WebSocket message and auto-transition to market screen
  const marketOpenedMsg = await waitForWebSocketMessage(userPage, 'market_opened', 10000);
  expect(marketOpenedMsg).toBeTruthy();
  expect(marketOpenedMsg.type).toBe('market_opened');

  // User should automatically transition to market screen
  await userPage.waitForSelector('#market-screen.active', { timeout: 5000 });

  await Promise.all([adminContext.close(), userContext.close()]);
});

test('websocket broadcasts bet_created event', async ({ browser, baseURL }) => {
  const url = baseURL ?? LOCAL_URL;

  // Setup market with admin
  const adminContext = await browser.newContext();
  const adminPage = await adminContext.newPage();

  await adminPage.goto(url);
  await adminPage.getByRole('button', { name: 'Create Market' }).click();
  await adminPage.fill('#admin-name', 'admin');
  await adminPage.fill('#market-name', 'bet-test');
  await adminPage.fill('#duration', '24');
  await adminPage.fill('#starting-balance', '1000');
  await adminPage.getByRole('button', { name: 'Create' }).click();

  await adminPage.waitForSelector('#lobby-screen.active');
  const inviteCode = (await adminPage.locator('#lobby-invite-code').textContent())?.trim();

  // User joins
  const userContext = await browser.newContext();
  const userPage = await userContext.newPage();
  const getUserMessages = await setupWebSocketListener(userPage);

  await userPage.goto(url);
  await userPage.getByRole('button', { name: 'Join Market' }).click();
  await userPage.fill('#invite-code', inviteCode!);
  await userPage.fill('#display-name', 'dave');
  await userPage.getByRole('button', { name: 'Join' }).click();

  // Open market
  await adminPage.getByRole('button', { name: 'Open Market' }).click();
  await adminPage.waitForSelector('#market-screen.active');
  await userPage.waitForSelector('#market-screen.active');

  // Clear user's messages
  await userPage.evaluate(() => {
    // @ts-ignore
    window.__wsMessages = [];
  });

  // Admin creates bet about dave
  await adminPage.getByRole('button', { name: '+ Create Bet' }).click();
  await adminPage.waitForSelector('#create-bet-modal.active');
  await adminPage.fill('#bet-description', '@dave will mention cryptocurrency');
  await adminPage.fill('#opening-wager', '100');
  await adminPage.locator('#create-bet-form button[type="submit"]').click();

  // User should receive bet_created WebSocket message
  const betCreatedMsg = await waitForWebSocketMessage(userPage, 'bet_created', 10000);
  expect(betCreatedMsg).toBeTruthy();
  expect(betCreatedMsg.type).toBe('bet_created');

  // User should see the bet appear in their list (but hidden since it's about them)
  await userPage.evaluate(() => (window as any).loadBets?.());
  await userPage.waitForSelector('#bets-list .bet-card', { timeout: 5000 });

  await Promise.all([adminContext.close(), userContext.close()]);
});

test('websocket broadcasts wager_placed event', async ({ browser, baseURL }) => {
  const url = baseURL ?? LOCAL_URL;

  // Setup market with admin
  const adminContext = await browser.newContext();
  const adminPage = await adminContext.newPage();
  const getAdminMessages = await setupWebSocketListener(adminPage);

  await adminPage.goto(url);
  await adminPage.getByRole('button', { name: 'Create Market' }).click();
  await adminPage.fill('#admin-name', 'admin');
  await adminPage.fill('#market-name', 'wager-test');
  await adminPage.fill('#duration', '24');
  await adminPage.fill('#starting-balance', '1000');
  await adminPage.getByRole('button', { name: 'Create' }).click();

  await adminPage.waitForSelector('#lobby-screen.active');
  const inviteCode = (await adminPage.locator('#lobby-invite-code').textContent())?.trim();

  // Two users join
  const user1Context = await browser.newContext();
  const user1Page = await user1Context.newPage();

  await user1Page.goto(url);
  await user1Page.getByRole('button', { name: 'Join Market' }).click();
  await user1Page.fill('#invite-code', inviteCode!);
  await user1Page.fill('#display-name', 'eve');
  await user1Page.getByRole('button', { name: 'Join' }).click();

  const user2Context = await browser.newContext();
  const user2Page = await user2Context.newPage();

  await user2Page.goto(url);
  await user2Page.getByRole('button', { name: 'Join Market' }).click();
  await user2Page.fill('#invite-code', inviteCode!);
  await user2Page.fill('#display-name', 'frank');
  await user2Page.getByRole('button', { name: 'Join' }).click();

  // Open market
  await adminPage.getByRole('button', { name: 'Open Market' }).click();
  await adminPage.waitForSelector('#market-screen.active');
  await user1Page.waitForSelector('#market-screen.active');
  await user2Page.waitForSelector('#market-screen.active');

  // Create bet about frank
  await adminPage.getByRole('button', { name: '+ Create Bet' }).click();
  await adminPage.waitForSelector('#create-bet-modal.active');
  await adminPage.fill('#bet-description', '@frank will arrive late');
  await adminPage.fill('#opening-wager', '100');
  await adminPage.locator('#create-bet-form button[type="submit"]').click();

  // Wait for bet to appear
  await adminPage.evaluate(() => (window as any).loadBets?.());
  await user1Page.evaluate(() => (window as any).loadBets?.());
  await user1Page.waitForSelector('#bets-list .bet-card');

  // Clear admin's messages
  await adminPage.evaluate(() => {
    // @ts-ignore
    window.__wsMessages = [];
  });

  // User1 places wager
  const betCard = user1Page.locator('#bets-list .bet-card').first();
  await betCard.getByRole('button', { name: 'Place Wager' }).click();
  await user1Page.waitForSelector('#wager-modal.active');
  await user1Page.locator('input[name="side"][value="YES"]').check();
  await user1Page.fill('#wager-amount', '50');
  await user1Page.getByRole('button', { name: 'Place Wager' }).click();

  // Admin should receive wager_placed WebSocket message
  const wagerPlacedMsg = await waitForWebSocketMessage(adminPage, 'wager_placed', 10000);
  expect(wagerPlacedMsg).toBeTruthy();
  expect(wagerPlacedMsg.type).toBe('wager_placed');

  // Admin should see updated pools
  await adminPage.evaluate(() => (window as any).loadBets?.());
  await adminPage.waitForTimeout(500);

  await Promise.all([adminContext.close(), user1Context.close(), user2Context.close()]);
});

test('websocket reconnects after disconnect', async ({ browser, baseURL }) => {
  const url = baseURL ?? LOCAL_URL;
  const context = await browser.newContext();
  const page = await context.newPage();

  await page.goto(url);
  await page.getByRole('button', { name: 'Create Market' }).click();
  await page.fill('#admin-name', 'reconnect-admin');
  await page.fill('#market-name', 'reconnect-test');
  await page.fill('#duration', '24');
  await page.fill('#starting-balance', '1000');
  await page.getByRole('button', { name: 'Create' }).click();

  await page.waitForSelector('#lobby-screen.active');

  // Verify initial connection
  let wsState = await page.evaluate(() => {
    // @ts-ignore
    return window.state?.ws?.readyState;
  });
  expect(wsState).toBe(1); // OPEN

  // Force close WebSocket
  await page.evaluate(() => {
    // @ts-ignore
    if (window.state?.ws) {
      // @ts-ignore
      window.state.ws.close();
    }
  });

  // Wait for automatic reconnection (app has 3 second reconnect delay)
  await page.waitForTimeout(4000);

  // Verify reconnected
  wsState = await page.evaluate(() => {
    // @ts-ignore
    return window.state?.ws?.readyState;
  });
  expect(wsState).toBe(1); // OPEN

  await context.close();
});

test('multiple clients receive same websocket broadcast', async ({ browser, baseURL }) => {
  const url = baseURL ?? LOCAL_URL;

  // Create market
  const adminContext = await browser.newContext();
  const adminPage = await adminContext.newPage();
  const getAdminMessages = await setupWebSocketListener(adminPage);

  await adminPage.goto(url);
  await adminPage.getByRole('button', { name: 'Create Market' }).click();
  await adminPage.fill('#admin-name', 'admin');
  await adminPage.fill('#market-name', 'multi-client-test');
  await adminPage.fill('#duration', '24');
  await adminPage.fill('#starting-balance', '1000');
  await adminPage.getByRole('button', { name: 'Create' }).click();

  await adminPage.waitForSelector('#lobby-screen.active');
  const inviteCode = (await adminPage.locator('#lobby-invite-code').textContent())?.trim();

  // Create 3 participant clients
  const clients = [];
  for (let i = 0; i < 3; i++) {
    const ctx = await browser.newContext();
    const pg = await ctx.newPage();
    const getMsgs = await setupWebSocketListener(pg);

    await pg.goto(url);
    await pg.getByRole('button', { name: 'Join Market' }).click();
    await pg.fill('#invite-code', inviteCode!);
    await pg.fill('#display-name', `user${i}`);
    await pg.getByRole('button', { name: 'Join' }).click();
    await pg.waitForSelector('#lobby-screen.active');

    clients.push({ context: ctx, page: pg, getMessages: getMsgs });
  }

  // Clear all messages
  for (const client of clients) {
    await client.page.evaluate(() => {
      // @ts-ignore
      window.__wsMessages = [];
    });
  }

  // Admin opens market - all clients should receive market_opened
  await adminPage.getByRole('button', { name: 'Open Market' }).click();
  await adminPage.waitForSelector('#market-screen.active');

  // Verify all clients received the broadcast
  for (let i = 0; i < clients.length; i++) {
    const msg = await waitForWebSocketMessage(clients[i].page, 'market_opened', 10000);
    expect(msg).toBeTruthy();
    expect(msg.type).toBe('market_opened');

    // All should transition to market screen
    await clients[i].page.waitForSelector('#market-screen.active');
  }

  // Cleanup
  await adminContext.close();
  for (const client of clients) {
    await client.context.close();
  }
});
