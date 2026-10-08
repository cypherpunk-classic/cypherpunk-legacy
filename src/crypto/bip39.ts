import { sha256 } from '@noble/hashes/sha2.js';
import { mnemonicToSeed, mnemonicToSeedSync, entropyToMnemonic, validateMnemonic } from '@scure/bip39';
import { wordlist } from '@scure/bip39/wordlists/english.js';
import { HDKey } from '@scure/bip32';
import { bitsToBytes, bytesToBits, toHex, type Bytes } from './bytes';
import type { Drbg } from './drbg';

/**
 * BIP-39 mnemonic codes and the "missing word" recovery used in the prologue.
 *
 * A 12-word mnemonic encodes 128 bits of entropy (ENT) plus a 4-bit checksum
 * (CS = first ENT/32 bits of SHA-256(ENT)): 12 words x 11 bits = 132 bits.
 * Because only 4 bits are redundancy, a single unknown word leaves about
 * 2048 / 16 = 128 candidates; extra knowledge (a visible first letter, a key
 * fingerprint written on a sticky note) narrows it to one.
 */

export const WORDLIST: readonly string[] = wordlist;
const INDEX = new Map(WORDLIST.map((w, i) => [w, i] as const));

export const isWord = (w: string): boolean => INDEX.has(w.trim().toLowerCase());
export const wordIndex = (w: string): number => INDEX.get(w.trim().toLowerCase()) ?? -1;

export function wordBits(w: string): string {
  const i = wordIndex(w);
  if (i < 0) throw new Error(`not a BIP-39 word: ${w}`);
  return i.toString(2).padStart(11, '0');
}

export interface ChecksumReport {
  bits: string; // all 132 bits
  entropyBits: string; // first 128 bits
  checksumBits: string; // last 4 bits, as written in the words
  expectedBits: string; // first 4 bits of SHA-256(entropy)
  entropy: Bytes;
  ok: boolean;
}

export function checksumReport(words: readonly string[]): ChecksumReport {
  if (words.length % 3 !== 0 || words.length < 12) throw new Error('mnemonic length must be 12, 15, 18, 21 or 24');
  const bits = words.map(wordBits).join('');
  const csLen = words.length / 3; // ENT/32 where ENT = 32 * words / 3
  const entropyBits = bits.slice(0, bits.length - csLen);
  const checksumBits = bits.slice(bits.length - csLen);
  const entropy = bitsToBytes(entropyBits);
  const expectedBits = bytesToBits(sha256(entropy)).slice(0, csLen);
  return { bits, entropyBits, checksumBits, expectedBits, entropy, ok: checksumBits === expectedBits };
}

export const checksumOk = (words: readonly string[]): boolean => {
  try {
    return checksumReport(words).ok;
  } catch {
    return false;
  }
};

/** Every word that makes the mnemonic's checksum valid when placed at `missing`. */
export function checksumCandidates(words: readonly (string | null)[], missing: number): string[] {
  const out: string[] = [];
  const trial = words.map((w) => w ?? 'abandon');
  for (const w of WORDLIST) {
    trial[missing] = w;
    if (checksumReport(trial).ok) out.push(w);
  }
  return out;
}

export interface Funnel {
  total: number;
  checksumValid: string[];
  letterMatches: string[];
}

export function recoveryFunnel(words: readonly (string | null)[], missing: number, firstLetter: string): Funnel {
  const checksumValid = checksumCandidates(words, missing);
  const letterMatches = checksumValid.filter((w) => w.startsWith(firstLetter.toLowerCase()));
  return { total: WORDLIST.length, checksumValid, letterMatches };
}

/** BIP-39 seed: PBKDF2-HMAC-SHA512(mnemonic, "mnemonic" + passphrase, 2048 rounds, 64 bytes). */
export async function seedFromMnemonic(phrase: string, passphrase = ''): Promise<Bytes> {
  return mnemonicToSeed(phrase, passphrase);
}

export function seedFromMnemonicSync(phrase: string, passphrase = ''): Bytes {
  return mnemonicToSeedSync(phrase, passphrase);
}

export const isValidMnemonic = (phrase: string): boolean => validateMnemonic(phrase, wordlist);

/** BIP-32 master fingerprint: first 4 bytes of HASH160(master public key), as written in descriptors. */
export function masterFingerprint(seed: Bytes): string {
  return HDKey.fromMasterSeed(seed).fingerprint.toString(16).padStart(8, '0');
}

export interface DerivedKey {
  path: string;
  privateKey: Bytes;
  publicKey: Bytes; // 33-byte compressed
  xOnly: Bytes; // 32-byte BIP340 key
  xpub: string;
}

export function deriveKey(seed: Bytes, path: string): DerivedKey {
  const k = HDKey.fromMasterSeed(seed).derive(path);
  if (!k.privateKey || !k.publicKey) throw new Error('derivation failed');
  return { path, privateKey: k.privateKey, publicKey: k.publicKey, xOnly: k.publicKey.slice(1), xpub: k.publicExtendedKey };
}

/** Standard BIP-86 single-key Taproot paths. */
export const BIP86_ACCOUNT = "m/86'/0'/0'";
export const BIP86_FIRST = "m/86'/0'/0'/0/0";

export function descriptor(seed: Bytes): string {
  const fp = masterFingerprint(seed);
  const acct = deriveKey(seed, BIP86_ACCOUNT);
  return `tr([${fp}/86h/0h/0h]${acct.xpub}/0/*)`;
}

export interface GrandpaSeed {
  words: string[];
  phrase: string;
  missingIndex: number;
  firstLetter: string;
  fingerprint: string;
  funnel: Funnel;
}

/**
 * Generate a mnemonic whose recovery funnel is interesting: the visible first
 * letter must leave at least 3 checksum-valid candidates, so the fingerprint on
 * the sticky note is genuinely needed to pick the right one.
 */
export function generateGrandpaSeed(rng: Drbg): GrandpaSeed {
  for (;;) {
    const entropy = rng.bytes(16);
    const phrase = entropyToMnemonic(entropy, wordlist);
    const words = phrase.split(' ');
    const missingIndex = 3 + rng.int(7); // somewhere in the middle of the letter
    const firstLetter = words[missingIndex]![0]!;
    const masked = words.map((w, i) => (i === missingIndex ? null : w));
    const funnel = recoveryFunnel(masked, missingIndex, firstLetter);
    if (funnel.letterMatches.length < 3 || funnel.letterMatches.length > 9) continue;
    const fingerprint = masterFingerprint(mnemonicToSeedSync(phrase));
    return { words, phrase, missingIndex, firstLetter, fingerprint, funnel };
  }
}

export { toHex };
