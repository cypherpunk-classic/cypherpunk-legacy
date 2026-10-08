import * as Phaser from 'phaser';
import { generateTextures } from '../world/textures';

/** Paint every texture in code, then hand over to the title screen. */
export class BootScene extends Phaser.Scene {
  constructor() {
    super({ key: 'boot' });
  }

  create(): void {
    generateTextures(this);
    this.game.events.emit('cpl-booted');
  }
}
