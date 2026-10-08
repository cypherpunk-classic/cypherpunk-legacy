import { test, expect, type Page } from '@playwright/test';

/**
 * Full playthrough: starts a new game and completes every chapter by driving
 * the real UI. Each puzzle is solved through its own `autoplay()`, which works
 * the same controls a player would, so this doubles as a check that every
 * puzzle is actually solvable. The debug bridge (?debug) only triggers
 * interactions and advances dialogue; it never sets a flag to skip a puzzle.
 */

async function unblocked(page: Page): Promise<void> {
  // Wait until no UI layer is blocking AND the world scene has finished its
  // current interaction, so the next fired interaction is not dropped as busy.
  // Require idle to hold across a short gap, to skip the transient idle window
  // before an auto-started cutscene (e.g. a scene's intro) takes over.
  await page.waitForFunction(
    () => {
      const w = window as any;
      if (!w.cpl.idle()) {
        w.__idleSince = 0;
        return false;
      }
      w.__idleSince = w.__idleSince || Date.now();
      return Date.now() - w.__idleSince > 250;
    },
    { timeout: 25000, polling: 80 },
  );
}

/** Fire an interaction without awaiting (it blocks on dialogue/puzzles). */
const fireRaw = (page: Page, id: string): void => void page.evaluate((i) => void (window as any).cpl.interact(i), id);

/** Wait until idle, then fire — so the interaction is never dropped as busy. */
async function fire(page: Page, id: string): Promise<void> {
  await unblocked(page);
  fireRaw(page, id);
}

/** Trigger a dialogue-only interaction and wait for the dialogue to finish. */
async function interact(page: Page, id: string): Promise<void> {
  await fire(page, id);
  await unblocked(page);
}

const autoplay = (page: Page): Promise<void> => page.evaluate(() => (window as any).cpl.autoplay());
const flag = (page: Page, f: string): Promise<boolean> => page.evaluate((x) => (window as any).cpl.state.flag(x), f);

/** Autoplay the currently-open puzzle and press Continue. Does NOT wait for
 * idle afterwards, because the next step may be another puzzle or follow-up
 * dialogue; callers wait for whatever comes next (a puzzle selector, or, for a
 * flag assertion / interaction, the leading `unblocked` inside `fire`). */
async function finishPuzzle(page: Page): Promise<void> {
  await autoplay(page);
  await page.waitForSelector('[data-testid=terminal-continue]', { timeout: 20000 });
  await page.click('[data-testid=terminal-continue]');
}

/** Open a puzzle (fire-and-forget), autoplay it, press Continue. */
async function solve(page: Page, id: string, open: () => Promise<void>): Promise<void> {
  await open();
  await page.waitForSelector(`[data-puzzle="${id}"]`, { timeout: 15000 });
  await finishPuzzle(page);
}

/** A puzzle that opens after some dialogue inside an already-fired interaction. */
async function solveOpened(page: Page, id: string): Promise<void> {
  await page.waitForSelector(`[data-puzzle="${id}"]`, { timeout: 15000 });
  await finishPuzzle(page);
}

test('new game to credits, solving every puzzle through the UI', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(e.message));

  await page.goto('/?debug&seed=e2e-playthrough');
  await page.waitForSelector('[data-testid=title]', { timeout: 20000 });

  // New game
  await page.click('[data-testid=new-game]');
  await page.fill('[data-testid=handle]', 'tester');
  await page.click('[data-testid=begin]');
  await page.waitForTimeout(1500);
  await page.evaluate(() => (window as any).cpl.autoDialog(true));

  // --- Prologue ---
  await interact(page, 'cabin.dresser');
  await fire(page, 'cabin.computer'); // note + boot dialogue, then the seed puzzle
  await solveOpened(page, 'pro.seed');
  await solveOpened(page, 'pro.login'); // opens right after seed
  await page.waitForFunction(() => (window as any).cpl.scene() === 'city', { timeout: 20000 });
  await unblocked(page);
  expect(await flag(page, 'pro.done')).toBe(true);

  // --- Chapter 1 ---
  await fire(page, 'mara.talk');
  await solveOpened(page, 'ch1.otp');
  await solve(page, 'ch1.ctr', () => fire(page, 'wharf.boathouse'));
  await interact(page, 'mara.talk');
  expect(await flag(page, 'ch1.done')).toBe(true);

  // --- Chapter 2 ---
  await fire(page, 'felix.talk');
  await solveOpened(page, 'ch2.dh');
  await solve(page, 'ch2.mitm', () => fire(page, 'exchange.kiosk'));
  await solve(page, 'ch2.logjam', () => fire(page, 'exchange.kiosk'));
  await interact(page, 'felix.talk');
  expect(await flag(page, 'ch2.done')).toBe(true);

  // --- Chapter 3 ---
  await fire(page, 'juno.talk');
  await solveOpened(page, 'ch3.shamir');
  await interact(page, 'commons.dropA');
  await interact(page, 'commons.dropB');
  await solve(page, 'ch3.feldman', () => fire(page, 'juno.talk'));
  await interact(page, 'juno.talk');
  expect(await flag(page, 'ch3.done')).toBe(true);

  // Open the Vault
  await page.evaluate(() => (window as any).cpl.teleport(480, 420));
  await fire(page, 'plaza.vault');
  await page.waitForFunction(() => (window as any).cpl.state.flag('vault.opened'), { timeout: 20000 });
  await unblocked(page);

  // --- Chapter 4 ---
  await solve(page, 'ch4.nonce', () => fire(page, 'ibrahim.talk'));
  await fire(page, 'mint.archive'); // archive opens, then rogue puzzle
  await solveOpened(page, 'ch4.rogue');
  await solve(page, 'ch4.taproot', () => fire(page, 'mint.kiosk'));
  await interact(page, 'ibrahim.talk');
  expect(await flag(page, 'ch4.done')).toBe(true);

  // --- Chapter 5 ---
  await solve(page, 'ch5.sigma', () => fire(page, 'lin.talk'));
  await solve(page, 'ch5.sim', () => fire(page, 'lin.talk'));
  await solve(page, 'ch5.extract', () => fire(page, 'lin.talk'));
  await interact(page, 'lin.talk');
  expect(await flag(page, 'ch5.done')).toBe(true);

  // --- Chapter 6 ---
  await solve(page, 'ch6.gauss', () => fire(page, 'ada.talk'));
  await solve(page, 'ch6.knapsack', () => fire(page, 'ada.talk'));
  await solve(page, 'ch6.pq', () => fire(page, 'ada.talk'));
  await interact(page, 'ada.talk');
  expect(await flag(page, 'end.ready')).toBe(true);

  // --- Finale ---
  await page.evaluate(() => (window as any).cpl.teleport(480, 420));
  await fire(page, 'plaza.vault'); // finale cutscene, then the broadcast puzzle
  await page.waitForSelector('[data-puzzle="end.broadcast"]', { timeout: 20000 });
  await autoplay(page);
  await page.waitForSelector('[data-testid=terminal-continue]', { timeout: 15000 });
  await page.click('[data-testid=terminal-continue]');
  await page.waitForSelector('[data-testid=credits]', { timeout: 20000 });

  expect(await flag(page, 'end.done')).toBe(true);
  expect(errors, `page errors during playthrough:\n${errors.join('\n')}`).toEqual([]);
});
