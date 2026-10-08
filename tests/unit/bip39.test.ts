import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { entropyToMnemonic } from '@scure/bip39';
import { wordlist } from '@scure/bip39/wordlists/english.js';
import {
  BIP86_FIRST,
  checksumCandidates,
  checksumOk,
  checksumReport,
  deriveKey,
  generateGrandpaSeed,
  masterFingerprint,
  recoveryFunnel,
  seedFromMnemonicSync,
} from '../../src/crypto/bip39';
import { tweakKey } from '../../src/crypto/taproot';
import { fromHex, toHex } from '../../src/crypto/bytes';
import { Drbg } from '../../src/crypto/drbg';

const trezor = JSON.parse(readFileSync(new URL('../vectors/vectors.json', import.meta.url), 'utf8')).english as string[][];
const ABANDON = 'abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about';

describe('BIP-39', () => {
  it('matches the reference vectors (entropy -> words -> seed with passphrase TREZOR)', () => {
    for (const [entropy, phrase, seed] of trezor.slice(0, 8)) {
      expect(entropyToMnemonic(fromHex(entropy!), wordlist)).toBe(phrase);
      expect(checksumOk(phrase!.split(' '))).toBe(true);
      expect(toHex(seedFromMnemonicSync(phrase!, 'TREZOR'))).toBe(seed);
    }
  });

  it('exposes the checksum bits honestly', () => {
    const r = checksumReport(ABANDON.split(' '));
    expect(r.entropyBits).toBe('0'.repeat(128));
    expect(r.checksumBits).toBe(r.expectedBits);
    expect(r.checksumBits).toBe('0011'); // SHA-256(0^128) starts with 0x37
    const broken = ABANDON.split(' ');
    broken[11] = 'abandon';
    expect(checksumOk(broken)).toBe(false);
  });

  it('about one word in sixteen fits a hole', () => {
    const words: (string | null)[] = ABANDON.split(' ');
    words[5] = null;
    const c = checksumCandidates(words, 5);
    expect(c).toContain('abandon');
    expect(c.length).toBeGreaterThan(90);
    expect(c.length).toBeLessThan(170);
  });

  it('derives the BIP-86 test vector and the well-known master fingerprint', () => {
    const seed = seedFromMnemonicSync(ABANDON);
    expect(masterFingerprint(seed)).toBe('73c5da0a');
    const k = deriveKey(seed, BIP86_FIRST);
    expect(toHex(k.xOnly)).toBe('cc8a4bc64d897bddc5fbc2f670f7a8ba0b386779106cf1223c6fc5d7cd6fc115');
    const t = tweakKey(k.xOnly, null);
    expect(toHex(t.outputX)).toBe('a60869f0dbcf1dc659c9cecbaf8050135ea9e8cdc487053f1dc6880949dc684c');
    expect(t.address).toBe('bc1p5cyxnuxmeuwuvkwfem96lqzszd02n6xdcjrs20cac6yqjjwudpxqkedrcr');
  });

  it('generates a recovery puzzle with a unique fingerprint match', () => {
    for (let i = 0; i < 3; i++) {
      const g = generateGrandpaSeed(new Drbg(new Uint8Array(32).fill(i + 3), 'grandpa'));
      const masked = g.words.map((w, j) => (j === g.missingIndex ? null : w));
      const f = recoveryFunnel(masked, g.missingIndex, g.firstLetter);
      expect(f.letterMatches).toContain(g.words[g.missingIndex]);
      expect(f.letterMatches.length).toBeGreaterThanOrEqual(3);
      const matching = f.letterMatches.filter((w) => {
        const trial = g.words.slice();
        trial[g.missingIndex] = w;
        return masterFingerprint(seedFromMnemonicSync(trial.join(' '))) === g.fingerprint;
      });
      expect(matching).toEqual([g.words[g.missingIndex]]);
    }
  });
});
