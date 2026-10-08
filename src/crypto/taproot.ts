import { bech32m } from '@scure/base';
import { bytesToBigInt, compareBytes, concat, fromHex, toHex, type Bytes } from './bytes';
import { hasEvenY, liftX, modN, mul, taggedHash, xOnly, N, type Pt } from './secp';

/**
 * BIP-341 Taproot outputs.
 *
 * An output key Q commits to an internal key P and (optionally) a Merkle tree
 * of scripts:   t = H_TapTweak(P.x || root),   Q = P + t*G.
 * Spending by key path needs only a signature for Q; spending by script path
 * reveals one leaf plus its Merkle path, and nothing about the other leaves.
 */

export const LEAF_VERSION_TAPSCRIPT = 0xc0;

export const OP = {
  CHECKSIG: 0xac,
  CHECKSIGVERIFY: 0xad,
  CHECKSIGADD: 0xba,
  CHECKLOCKTIMEVERIFY: 0xb1,
  CHECKSEQUENCEVERIFY: 0xb2,
  DROP: 0x75,
  NUMEQUAL: 0x9c,
  OP_2: 0x52,
  OP_3: 0x53,
} as const;

export function compactSize(n: number): Bytes {
  if (n < 0xfd) return Uint8Array.of(n);
  if (n <= 0xffff) return Uint8Array.of(0xfd, n & 0xff, n >> 8);
  throw new Error('script too long for this toy encoder');
}

/** Minimal CScriptNum encoding push (for lock times). */
export function pushNumber(v: number): Bytes {
  if (v === 0) return Uint8Array.of(0x00);
  if (v >= 1 && v <= 16) return Uint8Array.of(0x50 + v);
  const bytes: number[] = [];
  let x = Math.abs(v);
  while (x > 0) {
    bytes.push(x & 0xff);
    x = Math.floor(x / 256);
  }
  if (bytes[bytes.length - 1]! & 0x80) bytes.push(v < 0 ? 0x80 : 0x00);
  else if (v < 0) bytes[bytes.length - 1]! |= 0x80;
  return concat(Uint8Array.of(bytes.length), Uint8Array.from(bytes));
}

export const pushKey = (xonly: Bytes): Bytes => concat(Uint8Array.of(0x20), xonly);

export interface Leaf {
  id: string;
  label: string;
  asm: string;
  script: Bytes;
}

export function leafSingleSig(id: string, label: string, keyX: Bytes, keyName: string): Leaf {
  return { id, label, asm: `<${keyName}> OP_CHECKSIG`, script: concat(pushKey(keyX), Uint8Array.of(OP.CHECKSIG)) };
}

export function leafTwoOfTwo(id: string, label: string, aX: Bytes, aName: string, bX: Bytes, bName: string): Leaf {
  return {
    id,
    label,
    asm: `<${aName}> OP_CHECKSIGVERIFY <${bName}> OP_CHECKSIG`,
    script: concat(pushKey(aX), Uint8Array.of(OP.CHECKSIGVERIFY), pushKey(bX), Uint8Array.of(OP.CHECKSIG)),
  };
}

export function leafTimelocked(id: string, label: string, lockTime: number, keyX: Bytes, keyName: string): Leaf {
  return {
    id,
    label,
    asm: `${lockTime} OP_CHECKLOCKTIMEVERIFY OP_DROP <${keyName}> OP_CHECKSIG`,
    script: concat(pushNumber(lockTime), Uint8Array.of(OP.CHECKLOCKTIMEVERIFY, OP.DROP), pushKey(keyX), Uint8Array.of(OP.CHECKSIG)),
  };
}

export function tapLeafHash(script: Bytes, leafVersion = LEAF_VERSION_TAPSCRIPT): Bytes {
  return taggedHash('TapLeaf', Uint8Array.of(leafVersion), compactSize(script.length), script);
}

export function tapBranchHash(a: Bytes, b: Bytes): Bytes {
  return compareBytes(a, b) <= 0 ? taggedHash('TapBranch', a, b) : taggedHash('TapBranch', b, a);
}

/** A script tree as in the BIP-341 test vectors: a leaf, or a pair of subtrees. */
export type ScriptTree = { script: Bytes; leafVersion?: number } | [ScriptTree, ScriptTree];

export function treeHash(tree: ScriptTree): Bytes {
  if (Array.isArray(tree)) return tapBranchHash(treeHash(tree[0]), treeHash(tree[1]));
  return tapLeafHash(tree.script, tree.leafVersion ?? LEAF_VERSION_TAPSCRIPT);
}

export interface TweakResult {
  internal: Pt;
  merkleRoot: Bytes | null;
  tweak: bigint;
  output: Pt;
  outputX: Bytes;
  parity: 0 | 1;
  address: string;
  scriptPubKey: Bytes;
}

export function tweakKey(internalX: Bytes, merkleRoot: Bytes | null, hrp = 'bc'): TweakResult {
  const internal = liftX(internalX);
  const tHash = merkleRoot ? taggedHash('TapTweak', internalX, merkleRoot) : taggedHash('TapTweak', internalX);
  const tweak = bytesToBigInt(tHash);
  if (tweak >= N) throw new Error('tweak out of range');
  const output = internal.add(mul(modN(tweak)));
  const outputX = xOnly(output);
  return {
    internal,
    merkleRoot,
    tweak,
    output,
    outputX,
    parity: hasEvenY(output) ? 0 : 1,
    address: taprootAddress(outputX, hrp),
    scriptPubKey: concat(Uint8Array.of(0x51, 0x20), outputX),
  };
}

export function taprootAddress(outputX: Bytes, hrp = 'bc'): string {
  return bech32m.encode(hrp, [1, ...bech32m.toWords(outputX)]);
}

export function decodeTaprootAddress(addr: string): Bytes {
  const { words } = bech32m.decode(addr as `${string}1${string}`);
  if (words[0] !== 1) throw new Error('not a segwit v1 address');
  return Uint8Array.from(bech32m.fromWords(words.slice(1)));
}

/** All the leaf pairs a player could try, with the address each pair would produce. */
export function pairAddresses(internalX: Bytes, leaves: Leaf[]): { a: Leaf; b: Leaf; root: Bytes; address: string }[] {
  const out: { a: Leaf; b: Leaf; root: Bytes; address: string }[] = [];
  for (let i = 0; i < leaves.length; i++)
    for (let j = i + 1; j < leaves.length; j++) {
      const root = tapBranchHash(tapLeafHash(leaves[i]!.script), tapLeafHash(leaves[j]!.script));
      out.push({ a: leaves[i]!, b: leaves[j]!, root, address: tweakKey(internalX, root).address });
    }
  return out;
}

export { toHex, fromHex };
