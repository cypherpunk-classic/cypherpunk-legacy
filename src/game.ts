import * as Phaser from 'phaser';
import { audio } from './core/audio';
import { state, type Facing, type SceneId } from './core/state';
import { BootScene } from './scenes/BootScene';
import { TitleScene } from './scenes/TitleScene';
import { WorldScene, setWorldHooks } from './world/WorldScene';
import { cabinMap } from './world/maps/cabin';
import { cityMap } from './world/maps/city';
import { ui } from './ui/root';
import { hud } from './ui/hud';
import { openMenu, type MenuTab } from './ui/menu';
import { showTitle } from './ui/title';
import { toast } from './ui/toast';
import { codexById } from './story/codex';

/**
 * The application shell: owns the Phaser game, switches between the title
 * screen and the world scenes, and routes menu actions.
 */

class App {
  game!: Phaser.Game;
  world: WorldScene | null = null;
  private booted!: Promise<void>;

  start(parent: HTMLElement): void {
    this.game = new Phaser.Game({
      type: Phaser.AUTO,
      parent,
      backgroundColor: '#07080e',
      pixelArt: true,
      roundPixels: true,
      antialias: false,
      scale: { mode: Phaser.Scale.RESIZE, width: window.innerWidth, height: window.innerHeight },
      physics: { default: 'arcade', arcade: { debug: false } },
      audio: { noAudio: true },
      banner: false,
      fps: { target: 60 },
      scene: [new BootScene(), new TitleScene(), new WorldScene(cabinMap), new WorldScene(cityMap)],
    });
    this.booted = new Promise((resolve) => this.game.events.once('cpl-booted', () => resolve()));
    hud.onMenu = () => this.menu('keyring');
    hud.onCodex = () => this.menu('codex');
    state.on('codex', ({ id }) => {
      const e = codexById(id);
      if (e) toast('CODEX', e.title, 'codex');
      audio.sfx('pickup');
    });
    state.on('settings', (s) => this.applySettings(s));
    this.applySettings(state.settings);
  }

  private applySettings(s: typeof state.settings): void {
    audio.setVolumes(s.music, s.sfx);
    document.body.classList.toggle('crt-on', s.crt);
    document.body.classList.toggle('reduced-motion', s.reducedMotion);
  }

  async ready(): Promise<void> {
    await this.booted;
  }

  title(): void {
    this.stopWorlds();
    if (!this.game.scene.isActive('title')) this.game.scene.start('title');
    showTitle({
      onContinue: () => void this.continueGame(),
      onNewGame: (handle, look) => void this.newGame(handle, look),
      onSettings: () => void openMenu('settings', { quitToTitle: () => this.title() }),
    });
  }

  private stopWorlds(): void {
    for (const k of ['cabin', 'city']) if (this.game.scene.isActive(k) || this.game.scene.isPaused(k)) this.game.scene.stop(k);
    this.world = null;
    ui.showHud(false);
    hud.setHint(null);
  }

  async newGame(handle: string, look: number): Promise<void> {
    state.newGame(handle, look);
    await this.goto('cabin', 'start');
  }

  async continueGame(): Promise<void> {
    if (!state.load()) return this.title();
    await this.goto(state.data.scene, undefined, state.data.pos);
  }

  /** Fade to black, swap scenes, fade back in. */
  async goto(id: SceneId, spawn?: string, pos?: { x: number; y: number; facing: Facing } | null): Promise<void> {
    await ui.fadeOut(350);
    ui.screen.replaceChildren();
    if (this.game.scene.isActive('title')) this.game.scene.stop('title');
    for (const k of ['cabin', 'city']) if (k !== id && this.game.scene.isActive(k)) this.game.scene.stop(k);
    const scene = this.game.scene.getScene(id) as unknown as WorldScene;
    const started = new Promise<void>((resolve) => scene.events.once('create', () => resolve()));
    if (this.game.scene.isActive(id)) this.game.scene.stop(id);
    this.game.scene.start(id, { spawn, pos: pos ?? null });
    await started;
    this.world = scene;
    ui.showHud(true);
    hud.refresh();
    state.data.scene = id;
    state.save();
    await ui.fadeIn(450);
  }

  menu(tab: MenuTab, focus?: string): void {
    if (ui.blocked || !this.world) return;
    void openMenu(tab, { quitToTitle: () => this.title() }, focus);
  }
}

export const app = new App();
export { setWorldHooks };
