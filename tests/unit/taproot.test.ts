import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { fromHex, toHex } from '../../src/crypto/bytes';
import { pairAddresses, pushNumber, tapLeafHash, treeHash, tweakKey, decodeTaprootAddress, leafSingleSig, leafTimelocked, leafTwoOfTwo, type ScriptTree } from '../../src/crypto/taproot';
import { mul, xOnly } from '../../src/crypto/secp';

const v = JSON.parse(readFileSync(new URL('../vectors/wallet-test-vectors.json', import.meta.url), 'utf8'));

type VTree = { script: string; leafVersion: number } | VTree[];
const toTree = (t: VTree): ScriptTree => (Array.isArray(t) ? [toTree(t[0]!), toTree(t[1]!)] : { script: fromHex(t.script), leafVersion: t.leafVersion });
const leaves = (t: VTree): { script: string; leafVersion: number }[] => (Array.isArray(t) ? t.flatMap(leaves) : [t]);

describe('BIP-341 Taproot', () => {
  it('matches every scriptPubKey test vector', () => {
    for (const tc of v.scriptPubKey) {
      const internal = fromHex(tc.given.internalPubkey);
      const tree: VTree | null = tc.given.scriptTree;
      if (tree) {
        const hashes = leaves(tree).map((l) => toHex(tapLeafHash(fromHex(l.script), l.leafVersion)));
        expect(hashes).toEqual(tc.intermediary.leafHashes);
      }
      const root = tree ? treeHash(toTree(tree)) : null;
      expect(root ? toHex(root) : null).toBe(tc.intermediary.merkleRoot);
      const t = tweakKey(internal, root);
      expect(t.tweak.toString(16).padStart(64, '0')).toBe(tc.intermediary.tweak);
      expect(toHex(t.outputX)).toBe(tc.intermediary.tweakedPubkey);
      expect(toHex(t.scriptPubKey)).toBe(tc.expected.scriptPubKey);
      expect(t.address).toBe(tc.expected.bip350Address);
      expect(toHex(decodeTaprootAddress(t.address))).toBe(tc.intermediary.tweakedPubkey);
    }
  });

  it('encodes lock times as minimal script numbers', () => {
    expect(toHex(pushNumber(1893456000))).toBe('0480d8db70');
    expect(toHex(pushNumber(16))).toBe('60');
    expect(toHex(pushNumber(128))).toBe('028000');
  });

  it('gives every leaf pair a different address', () => {
    const k = (i: bigint) => xOnly(mul(i));
    const ls = [
      leafTwoOfTwo('a', 'A', k(2n), 'A', k(3n), 'B'),
      leafTimelocked('b', 'B', 1893456000, k(4n), 'C'),
      leafSingleSig('c', 'C', k(5n), 'D'),
      leafTimelocked('d', 'D', 1924992000, k(6n), 'E'),
    ];
    const pairs = pairAddresses(k(7n), ls);
    expect(pairs.length).toBe(6);
    expect(new Set(pairs.map((p) => p.address)).size).toBe(6);
  });
});
