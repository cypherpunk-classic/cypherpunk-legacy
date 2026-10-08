import * as Phaser from 'phaser';
import { audio } from '../core/audio';

/**
 * Behind the title menu: Cryptopolis at night, drifting slowly. Built from
 * the same textures as the game itself.
 */
export class TitleScene extends Phaser.Scene {
  private layerFar!: Phaser.GameObjects.Container;
  private layerNear!: Phaser.GameObjects.Container;

  constructor() {
    super({ key: 'title' });
  }

  create(): void {
    const cam = this.cameras.main;
    cam.setBackgroundColor('#070914');
    const fit = (): void => {
      const z = Math.max(2, Math.floor(Math.min(this.scale.width / 320, this.scale.height / 200)));
      cam.setZoom(z);
      cam.centerOn(0, -40);
    };
    fit();
    this.scale.on('resize', fit);
    this.events.once('shutdown', () => this.scale.off('resize', fit));

    // stars
    const g = this.add.graphics();
    const rnd = new Phaser.Math.RandomDataGenerator(['cpl']);
    for (let i = 0; i < 140; i++) {
      g.fillStyle(rnd.pick([0x73eff7, 0xdfe8ff, 0xc792ff, 0xffffff]), rnd.realInRange(0.2, 0.9));
      g.fillRect(rnd.between(-500, 500), rnd.between(-260, 40), 1, 1);
    }
    // moon
    const moon = this.add.image(120, -170, 'glow-white').setBlendMode(Phaser.BlendModes.ADD).setAlpha(0.5).setScale(1.4);
    this.tweens.add({ targets: moon, alpha: 0.35, duration: 4000, yoyo: true, repeat: -1 });

    this.layerFar = this.add.container(0, 0);
    this.layerNear = this.add.container(0, 0);
    const far = ['bld-tower', 'bld-block_a', 'bld-observatory', 'bld-block_b', 'bld-mint', 'bld-tower', 'bld-masks', 'bld-block_a', 'bld-exchange'];
    let x = -560;
    for (const k of far) {
      const s = this.add.sprite(x, 30, k).setOrigin(0, 1).setTint(0x6070a0).setAlpha(0.9);
      s.play(`${k}-anim`);
      this.layerFar.add(s);
      x += s.width - 6;
    }
    x = -520;
    const near = ['bld-radioshack', 'bld-boathouse', 'bld-block_b', 'bld-pavilion', 'bld-exchange', 'bld-radioshack', 'bld-block_a', 'bld-boathouse'];
    for (const k of near) {
      const s = this.add.sprite(x, 70, k).setOrigin(0, 1);
      s.play(`${k}-anim`);
      this.layerNear.add(s);
      x += s.width + 10;
    }
    const signs = ['sign-wharf', 'sign-exchange', 'sign-mint', 'sign-masks', 'sign-observatory'];
    signs.forEach((k, i) => {
      const s = this.add.sprite(-420 + i * 190, 6, k).setOrigin(0.5, 1);
      s.play(`${k}-anim`);
      this.layerNear.add(s);
      const glow = this.add.image(-420 + i * 190, 0, 'glow-cyan').setBlendMode(Phaser.BlendModes.ADD).setAlpha(0.25);
      this.layerNear.add(glow);
    });
    // street
    const street = this.add.rectangle(-700, 70, 1400, 80, 0x111421).setOrigin(0, 0);
    this.layerNear.add(street);
    for (let i = -680; i < 700; i += 40) this.layerNear.add(this.add.rectangle(i, 100, 18, 2, 0xc9b458).setOrigin(0, 0));
    // drones
    for (let i = 0; i < 3; i++) {
      const d = this.add.sprite(-300 + i * 260, -90 - i * 20, 'obj-drone').play('obj-drone-anim');
      this.tweens.add({ targets: d, x: d.x + 160, duration: 9000 + i * 2000, yoyo: true, repeat: -1, ease: 'Sine.easeInOut' });
      this.tweens.add({ targets: d, y: d.y + 6, duration: 1200, yoyo: true, repeat: -1, ease: 'Sine.easeInOut' });
    }
    // rain
    this.add.particles(0, 0, 'raindrop', {
      x: { min: -600, max: 600 },
      y: -280,
      lifespan: 1800,
      speedY: { min: 220, max: 300 },
      speedX: { min: -30, max: -10 },
      quantity: 2,
      frequency: 30,
      alpha: { start: 0.6, end: 0.2 },
    });
    this.add.particles(0, 0, 'dot', {
      x: { min: -500, max: 500 },
      y: { min: -100, max: 80 },
      lifespan: 6000,
      speedY: { min: -8, max: -2 },
      scale: 0.5,
      alpha: { start: 0.7, end: 0 },
      tint: [0x73eff7, 0xff3e9a, 0xc792ff],
      frequency: 160,
      blendMode: Phaser.BlendModes.ADD,
    });
    audio.music('title');
  }

  override update(t: number): void {
    this.layerFar.x = Math.sin(t / 9000) * 40;
    this.layerNear.x = Math.sin(t / 9000) * 70;
  }
}
