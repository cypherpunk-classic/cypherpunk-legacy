import { checksumReport, isWord, masterFingerprint, seedFromMnemonic, WORDLIST, checksumCandidates } from '../../crypto/bip39';
import { toHex } from '../../crypto/bytes';
import { sha256 } from '@noble/hashes/sha2.js';
import { h } from '../../ui/dom';
import { bitsView, btn, card, kv, lede, note, pause, typeValue } from '../../ui/widgets';
import type { Puzzle, PuzzleHost } from '../../ui/terminal';
import { grandpa } from '../../story/world';

/**
 * Prologue, part 1. One word of Grandpa's mnemonic is illegible. The BIP-39
 * checksum rules out ~15/16 of the wordlist; the master fingerprint on the
 * sticky note pins down the one true word.
 */
export class SeedRecovery implements Puzzle {
  id = 'pro.seed';
  title = 'SEED RESTORE';
  subtitle = 'BIP-39 · 12 words · 128-bit entropy';
  accent = '#5dff9a';
  private host!: PuzzleHost;
  private input!: HTMLInputElement;
  private bitsBox!: HTMLElement;
  private checkBox!: HTMLElement;
  private funnelBox!: HTMLElement;
  private sweepBtn!: HTMLButtonElement;
  private tries = 0;
  private busy = false;

  mount(host: PuzzleHost): void {
    this.host = host;
    const g = grandpa().mnemonic;
    const target = g.fingerprint;

    const slots = h(
      'div',
      { style: 'display:grid;grid-template-columns:repeat(auto-fill,minmax(120px,1fr));gap:6px' },
      ...g.words.map((w, i) => {
        if (i !== g.missingIndex)
          return h('div', { class: 'pill', style: 'cursor:default;display:flex;gap:8px' }, h('span', { style: 'color:var(--faint)' }, String(i + 1).padStart(2, '0')), w);
        const list = h('datalist', { id: 'bip39-candidates' }, ...WORDLIST.filter((x) => x.startsWith(g.firstLetter)).map((x) => h('option', { value: x })));
        this.input = h('input', {
          type: 'text',
          list: 'bip39-candidates',
          placeholder: `${g.firstLetter}▒▒▒▒`,
          autocomplete: 'off',
          spellcheck: 'false',
          'data-testid': 'seed-word',
          style: 'width:100%;padding:3px 8px;border-color:var(--amber);font-size:14px',
        }) as HTMLInputElement;
        return h('div', { style: 'display:flex;gap:6px;align-items:center' }, h('span', { style: 'color:var(--amber);font-size:13px' }, String(i + 1).padStart(2, '0')), this.input, list);
      }),
    );

    this.bitsBox = h('div');
    this.checkBox = h('div', { class: 'note' });
    this.funnelBox = h('div', { class: 'funnel' });
    this.sweepBtn = btn('Run the recovery sweep', () => void this.sweep(), '', 'seed-sweep');

    this.input.addEventListener('input', () => this.preview());
    this.input.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') void this.tryWord();
    });

    host.body.append(
      h(
        'div',
        { class: 'stack' },
        lede(
          `Grandpa's letter lists twelve words. Word <b>${g.missingIndex + 1}</b> sits under a coffee ring: only its first letter, <b>“${g.firstLetter}”</b>, survives. The sticky note on the monitor gives the wallet's <span class="term">master fingerprint</span>. Find the word whose seed produces that fingerprint.`,
        ),
        card('The letter', slots),
        h(
          'div',
          { class: 'grid2' },
          card(
            'What the words encode',
            note('Each word is an 11-bit index into the 2048-word list. 12 × 11 = 132 bits: 128 bits of entropy, then a 4-bit checksum, the first 4 bits of SHA-256(entropy).'),
            h('div', { style: 'margin-top:8px' }, this.bitsBox),
            h('div', { style: 'margin-top:8px' }, this.checkBox),
          ),
          card(
            'Sticky note',
            h('div', { class: 'hex amber', style: 'font-size:12px' }, grandpa().descriptor),
            h('div', { style: 'margin-top:10px' }, kv([
              ['master fingerprint', h('span', { class: 'hex big amber' }, target)],
              ['meaning', 'first 4 bytes of HASH160(master public key)'],
            ])),
            h('div', { class: 'row', style: 'margin-top:12px' }, btn('Try this word', () => void this.tryWord(), 'primary', 'seed-try')),
          ),
        ),
        card(
          'Recovery sweep',
          note('Too many words start with that letter? Let the machine do what wallet-recovery tools do: test the whole wordlist against the checksum, keep the ones that fit the visible letter, then derive a seed for each survivor and compare fingerprints.'),
          h('div', { style: 'margin:10px 0' }, this.sweepBtn),
          this.funnelBox,
        ),
      ),
    );
    host.status('Type a candidate word, or just press “Run the recovery sweep” to let the machine find it.', 'info');
    this.preview();
  }

  private words(candidate: string): string[] {
    const g = grandpa().mnemonic;
    const w = g.words.slice();
    w[g.missingIndex] = candidate;
    return w;
  }

  private preview(): void {
    const g = grandpa().mnemonic;
    const cand = this.input.value.trim().toLowerCase();
    const missing = g.missingIndex;
    if (!isWord(cand)) {
      const placeholder = this.words('abandon');
      const rep = checksumReport(placeholder);
      this.bitsBox.replaceChildren(bitsView(rep.bits, { group: 11, marks: (i) => (i >= 128 ? 'cs' : i >= missing * 11 && i < missing * 11 + 11 ? 'bad' : '') }));
      this.checkBox.innerHTML = cand ? `<span style="color:var(--red)">“${cand}” is not in the BIP-39 wordlist.</span>` : 'The red bits are unknown until you choose a word.';
      return;
    }
    const rep = checksumReport(this.words(cand));
    this.bitsBox.replaceChildren(bitsView(rep.bits, { group: 11, marks: (i) => (i >= 128 ? (rep.ok ? 'cs' : 'cs bad') : '') }));
    const digest = toHex(sha256(rep.entropy));
    this.checkBox.innerHTML = `SHA-256(entropy) = <span class="hex">${digest.slice(0, 8)}…</span> → first 4 bits <b>${rep.expectedBits}</b>; the words say <b>${rep.checksumBits}</b>. ${
      rep.ok ? '<span style="color:var(--green)">Checksum OK.</span>' : '<span style="color:var(--red)">Checksum fails.</span>'
    }`;
  }

  private async tryWord(): Promise<void> {
    if (this.busy || this.host.solved()) return;
    const g = grandpa().mnemonic;
    const cand = this.input.value.trim().toLowerCase();
    this.tries++;
    this.sweepBtn.disabled = false;
    if (!isWord(cand)) {
      this.host.status('That is not a BIP-39 word.', 'bad');
      this.host.sfx('error');
      return;
    }
    if (!cand.startsWith(g.firstLetter)) {
      this.host.status(`The surviving letter is “${g.firstLetter}”; “${cand}” cannot be it.`, 'bad');
      this.host.sfx('error');
      return;
    }
    const rep = checksumReport(this.words(cand));
    if (!rep.ok) {
      this.host.status(`Checksum mismatch: “${cand}” would make the last 4 bits ${rep.checksumBits}, but SHA-256 says ${rep.expectedBits}.`, 'bad');
      this.host.sfx('error');
      return;
    }
    this.busy = true;
    this.host.status('Checksum OK. Deriving seed: PBKDF2-HMAC-SHA512, 2048 rounds…', 'info');
    const seed = await seedFromMnemonic(this.words(cand).join(' '));
    const fp = masterFingerprint(seed);
    this.busy = false;
    if (fp !== g.fingerprint) {
      this.host.status(`Checksum passes, but the fingerprint is ${fp}, not ${g.fingerprint}. About one wrong word in sixteen slips past a 4-bit checksum.`, 'warn');
      this.host.sfx('error');
      return;
    }
    this.host.sfx('unlock');
    this.input.style.borderColor = 'var(--green)';
    this.input.disabled = true;
    this.host.solve(`Fingerprint ${fp} matches. The seed is whole again.`);
  }

  private async sweep(): Promise<void> {
    if (this.busy) return;
    this.busy = true;
    this.sweepBtn.disabled = true;
    const g = grandpa().mnemonic;
    const masked = g.words.map((w, i) => (i === g.missingIndex ? null : w));
    const stage = (label: string, count: number, frac: number): HTMLElement => {
      const bar = h('div', { class: 'bar', style: 'width:0%' });
      const el = h('div', { class: 'stage' }, h('div', null, label), h('div', { class: 'count' }, String(count)), h('div', null, bar));
      this.funnelBox.append(el);
      requestAnimationFrame(() => (bar.style.width = `${Math.max(1, frac * 100)}%`));
      return el;
    };
    this.funnelBox.replaceChildren();
    stage('whole wordlist', 2048, 1);
    await pause(350);
    const valid = checksumCandidates(masked, g.missingIndex);
    stage('checksum fits', valid.length, valid.length / 2048);
    await pause(350);
    const lettered = valid.filter((w) => w.startsWith(g.firstLetter));
    stage(`starts with “${g.firstLetter}”`, lettered.length, lettered.length / 2048);
    const pills = h('div', { class: 'pill-list', style: 'margin-top:8px' });
    this.funnelBox.append(pills);
    const els = lettered.map((w) => {
      const p = h('span', { class: 'pill dim', 'data-word': w }, w);
      pills.append(p);
      return p;
    });
    let match = '';
    for (let i = 0; i < lettered.length; i++) {
      const w = lettered[i]!;
      const seed = await seedFromMnemonic(this.words(w).join(' '));
      const fp = masterFingerprint(seed);
      const ok = fp === g.fingerprint;
      els[i]!.className = `pill ${ok ? 'hit' : 'miss'}`;
      els[i]!.title = `fingerprint ${fp}`;
      if (ok) match = w;
      await pause(60);
    }
    stage('fingerprint matches', match ? 1 : 0, 1 / 2048);
    this.funnelBox.append(note(`Each survivor cost one PBKDF2 run (2048 × HMAC-SHA512). The checksum did the cheap filtering; the fingerprint did the rest.`));
    if (match) {
      const use = btn(`Use “${match}”`, () => {
        typeValue(this.input, match);
        void this.tryWord();
      }, 'primary', 'seed-use');
      this.funnelBox.append(h('div', { style: 'margin-top:10px' }, use));
    }
    this.busy = false;
  }

  async autoplay(): Promise<void> {
    const g = grandpa().mnemonic;
    typeValue(this.input, g.words[g.missingIndex]!);
    await this.tryWord();
  }
}
