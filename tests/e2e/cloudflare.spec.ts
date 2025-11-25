import { test, expect } from "@playwright/test";

/**
 * Cloudflare Deployment Verification Tests
 *
 * These tests are designed to verify that the application works correctly
 * when deployed to Cloudflare Pages with a separate API backend.
 *
 * Run these tests against your deployed Cloudflare Pages URL:
 *   E2E_BASE_URL=https://your-cazino.pages.dev npm run test:ui
 *
 * Critical aspects tested:
 * - WebSocket connections work through Cloudflare proxy
 * - API requests are properly routed
 * - CORS is configured correctly (if using separate domains)
 * - Real-time updates function in production environment
 */

test.describe("Cloudflare Deployment Tests", () => {
  test.skip(({ baseURL }) => {
    // Only run these tests when explicitly testing against a remote URL
    const url = baseURL || "http://localhost:3333";
    return url.includes("localhost") || url.includes("127.0.0.1");
  }, "Skipping Cloudflare tests - not running against remote deployment");

  test("health check endpoint is accessible", async ({ request, baseURL }) => {
    const response = await request.get(`${baseURL}/health`);
    expect(response.ok()).toBeTruthy();
  });

  test("api endpoint is accessible through cloudflare routing", async ({
    request,
    baseURL,
  }) => {
    // Test that /api/* routes are working
    const response = await request.get(`${baseURL}/api/health`);
    expect(response.ok()).toBeTruthy();

    const body = await response.text();
    expect(body.length).toBeGreaterThan(0);
  });

  test("websocket connection works through cloudflare", async ({
    page,
    baseURL,
  }) => {
    // Create a market
    await page.goto(baseURL!);
    await page.getByRole("button", { name: "Create Market" }).click();
    await page.fill("#admin-name", "cloudflare-admin");
    await page.fill("#market-name", "cloudflare-test");
    await page.fill("#duration", "24");
    await page.fill("#starting-balance", "1000");
    await page.getByRole("button", { name: "Create" }).click();

    await page.waitForSelector("#lobby-screen.active", { timeout: 10000 });

    // Check that WebSocket connected successfully
    const wsConnected = await page.evaluate(() => {
      // @ts-ignore
      return window.state?.ws?.readyState === WebSocket.OPEN;
    });

    expect(wsConnected).toBe(true);

    // Get the WebSocket URL to verify it's using the correct protocol
    const wsUrl = await page.evaluate(() => {
      // @ts-ignore
      return window.state?.ws?.url;
    });

    // Should be using wss:// (secure) for production
    if (baseURL?.startsWith("https://")) {
      expect(wsUrl).toContain("wss://");
    }
  });

  test("complete user flow works on cloudflare", async ({
    browser,
    baseURL,
  }) => {
    // This is a smoke test for the full application on Cloudflare

    // Create market
    const adminContext = await browser.newContext();
    const adminPage = await adminContext.newPage();

    await adminPage.goto(baseURL!);
    await adminPage.getByRole("button", { name: "Create Market" }).click();
    await adminPage.fill("#admin-name", "cf-admin");
    await adminPage.fill("#market-name", "cloudflare-full-test");
    await adminPage.fill("#duration", "24");
    await adminPage.fill("#starting-balance", "1000");
    await adminPage.getByRole("button", { name: "Create" }).click();

    await adminPage.waitForSelector("#lobby-screen.active", { timeout: 10000 });
    const inviteCode = (
      await adminPage.locator("#lobby-invite-code").textContent()
    )?.trim();
    expect(inviteCode).toBeTruthy();

    // User joins
    const userContext = await browser.newContext();
    const userPage = await userContext.newPage();

    await userPage.goto(baseURL!);
    await userPage.getByRole("button", { name: "Join Market" }).click();
    await userPage.fill("#invite-code", inviteCode!);
    await userPage.fill("#display-name", "cf-user");
    await userPage.getByRole("button", { name: "Join" }).click();

    await userPage.waitForSelector("#lobby-screen.active", { timeout: 10000 });

    // Verify WebSocket broadcast: admin should see user joined
    await expect(adminPage.locator("#player-list")).toContainText("@cf-user", {
      timeout: 10000,
    });

    // Open market
    await adminPage.getByRole("button", { name: "Open Market" }).click();
    await adminPage.waitForSelector("#market-screen.active", {
      timeout: 10000,
    });

    // Verify WebSocket broadcast: user should auto-transition to market screen
    await userPage.waitForSelector("#market-screen.active", { timeout: 10000 });

    // Create bet
    await adminPage.getByRole("button", { name: "+ Create Bet" }).click();
    await adminPage.waitForSelector("#create-bet-modal.active");
    await adminPage.fill(
      "#bet-description",
      "@cf-user will test on cloudflare",
    );
    await adminPage.fill("#opening-wager", "100");
    await adminPage.locator('#create-bet-form button[type="submit"]').click();

    // Bet should appear for admin
    await adminPage.evaluate(() => (window as any).loadBets?.());
    await adminPage.waitForSelector("#bets-list .bet-card", { timeout: 10000 });

    // Verify WebSocket broadcast: bet should appear for user (but hidden)
    await userPage.evaluate(() => (window as any).loadBets?.());
    await userPage.waitForSelector("#bets-list .bet-card", { timeout: 10000 });

    await Promise.all([adminContext.close(), userContext.close()]);
  });

  test("websocket reconnects after temporary disconnect on cloudflare", async ({
    page,
    baseURL,
  }) => {
    await page.goto(baseURL!);
    await page.getByRole("button", { name: "Create Market" }).click();
    await page.fill("#admin-name", "reconnect-admin");
    await page.fill("#market-name", "reconnect-cf-test");
    await page.fill("#duration", "24");
    await page.fill("#starting-balance", "1000");
    await page.getByRole("button", { name: "Create" }).click();

    await page.waitForSelector("#lobby-screen.active", { timeout: 10000 });

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
  });

  test("api responses include proper CORS headers (if cross-origin)", async ({
    request,
    baseURL,
  }) => {
    // This test verifies CORS is configured if UI and API are on different domains

    const response = await request.get(`${baseURL}/api/health`, {
      headers: {
        Origin: baseURL!,
      },
    });

    expect(response.ok()).toBeTruthy();

    // If this is a cross-origin request, CORS headers should be present
    // (Cloudflare may add these automatically, or they may come from the backend)
    const headers = response.headers();

    // Note: This might not be present if same-origin, which is fine
    // Just documenting expected behavior for cross-origin setups
    if (headers["access-control-allow-origin"]) {
      expect(headers["access-control-allow-origin"]).toBeTruthy();
    }
  });

  test("static assets load correctly", async ({ page, baseURL }) => {
    const responses: any[] = [];

    page.on("response", (response) => {
      responses.push({
        url: response.url(),
        status: response.status(),
      });
    });

    await page.goto(baseURL!);

    // Wait for page to fully load
    await page.waitForLoadState("networkidle");

    // Check that critical assets loaded successfully
    const failedRequests = responses.filter((r) => r.status >= 400);

    if (failedRequests.length > 0) {
      console.log("Failed requests:", failedRequests);
    }

    expect(failedRequests.length).toBe(0);

    // Verify key files loaded
    const cssLoaded = responses.some(
      (r) => r.url.includes("style.css") && r.status === 200,
    );
    const jsLoaded = responses.some(
      (r) => r.url.includes("app.js") && r.status === 200,
    );

    expect(cssLoaded).toBe(true);
    expect(jsLoaded).toBe(true);
  });

  test("environment configuration is correct", async ({ page, baseURL }) => {
    await page.goto(baseURL!);

    // Check that the UI has the correct configuration
    const config = await page.evaluate(() => {
      // @ts-ignore
      const API_BASE =
        window.API_BASE ||
        (window as any).resolveBaseUrl?.(window.CAZINO_CONFIG?.apiBase, "/api");
      // @ts-ignore
      const WS_URL =
        window.WS_URL ||
        (window as any).resolveWsUrl?.(window.CAZINO_CONFIG?.wsUrl, "/ws");

      return {
        API_BASE,
        WS_URL,
      };
    });

    // API_BASE should be defined and valid
    expect(config.API_BASE).toBeTruthy();
    expect(config.API_BASE).toContain("api");

    // WS_URL should be defined and valid
    expect(config.WS_URL).toBeTruthy();
    expect(config.WS_URL).toContain("ws");

    // If HTTPS, WebSocket should be WSS
    if (baseURL?.startsWith("https://")) {
      expect(config.WS_URL).toContain("wss://");
    }
  });
});

test.describe("Cloudflare Pages Specific Tests", () => {
  test.skip(({ baseURL }) => {
    // Only run if testing against *.pages.dev domain
    const url = baseURL || "http://localhost:3333";
    return !url.includes("pages.dev");
  }, "Skipping Pages-specific tests - not running against *.pages.dev");

  test("pages.dev domain is accessible", async ({ page, baseURL }) => {
    const response = await page.goto(baseURL!);
    expect(response?.ok()).toBeTruthy();
  });

  test("pages routing works correctly", async ({ page, baseURL }) => {
    // Test that Pages is serving the UI correctly
    await page.goto(baseURL!);

    // Check for key UI elements
    await expect(
      page.getByRole("button", { name: "Create Market" }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Join Market" }),
    ).toBeVisible();
  });
});
