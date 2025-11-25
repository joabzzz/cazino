import { test, expect, type Browser } from "@playwright/test";

const LOCAL_URL = "http://localhost:3333";

async function launchParticipant(
  browser: Browser,
  url: string,
  inviteCode: string,
  username: string,
) {
  const context = await browser.newContext();
  const page = await context.newPage();
  await page.goto(url);
  await page.getByRole("button", { name: "Join Market" }).click();
  await page.fill("#invite-code", inviteCode);
  await page.fill("#display-name", username);
  await page.getByRole("button", { name: "Join" }).click();
  await page.waitForSelector("#lobby-screen.active");
  await expect(page.locator("#player-list")).toContainText(`@${username}`);
  return { context, page };
}

test("ui + server happy path", async ({ browser, baseURL }) => {
  const url = baseURL ?? LOCAL_URL;

  // Admin creates market
  const adminContext = await browser.newContext();
  const adminPage = await adminContext.newPage();
  await adminPage.goto(url);

  await adminPage.getByRole("button", { name: "Create Market" }).click();
  await adminPage.fill("#admin-name", "admin");
  await adminPage.fill("#market-name", "integration-test");
  await adminPage.fill("#duration", "24");
  await adminPage.fill("#starting-balance", "1000");
  await adminPage.getByRole("button", { name: "Create" }).click();

  await adminPage.waitForSelector("#lobby-screen.active");
  const inviteCode = (
    await adminPage.locator("#lobby-invite-code").textContent()
  )?.trim();
  expect(inviteCode).toBeTruthy();

  // Two participants join so bets can target a third user
  const bob = await launchParticipant(browser, url, inviteCode!, "bob");
  const carol = await launchParticipant(browser, url, inviteCode!, "carol");

  // Admin opens market and creates a bet about Carol
  await adminPage.getByRole("button", { name: "Open Market" }).click();
  await adminPage.waitForSelector("#market-screen.active");

  await adminPage.getByRole("button", { name: "+ Create Bet" }).click();
  await adminPage.waitForSelector("#create-bet-modal.active");
  await adminPage.fill(
    "#bet-description",
    "@carol will brag about fantasy football",
  );
  await adminPage.fill("#opening-wager", "100");
  await adminPage.locator('#create-bet-form button[type="submit"]').click();
  await adminPage.evaluate(() => (window as any).loadBets?.());
  await adminPage.waitForSelector("#bets-list .bet-card");

  // Bob can see / wager on the bet (since it is about Carol)
  await bob.page.waitForSelector("#market-screen.active");
  await bob.page.evaluate(() => (window as any).loadBets?.());
  const betCard = bob.page.locator("#bets-list .bet-card").first();
  await expect(betCard).not.toHaveClass(/hidden/);
  await betCard.getByRole("button", { name: "Place Wager" }).click();
  await bob.page.waitForSelector("#wager-modal.active");
  await bob.page.locator('input[name="side"][value="NO"]').check();
  await bob.page.fill("#wager-amount", "100");
  await bob.page.getByRole("button", { name: "Place Wager" }).click();
  await bob.page.waitForSelector("#wager-modal.active", { state: "detached" });

  await expect(bob.page.locator("#user-balance")).toContainText("Z$900");
  await expect(bob.page.locator("#leaderboard-list")).toContainText("@bob");

  await Promise.all([
    adminContext.close(),
    bob.context.close(),
    carol.context.close(),
  ]);
});
