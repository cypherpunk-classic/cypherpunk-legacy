import { state } from '../core/state';

/** Things that end up on your keyring. Details are computed from the save. */

export interface ItemDef {
  id: string;
  name: string;
  desc: string;
  details?: () => [string, string][];
}

type DetailFn = () => [string, string][];
const detailFns = new Map<string, DetailFn>();

/** Story modules register live details (keys, shares) once the world is derived. */
export function registerItemDetails(id: string, fn: DetailFn): void {
  detailFns.set(id, fn);
}

export const ITEMS: ItemDef[] = [
  { id: 'letter', name: "Grandpa's letter", desc: 'Twelve words in blue ink. One of them drowned in a coffee ring.' },
  { id: 'descriptor', name: 'Sticky note', desc: 'A wallet descriptor in Grandpa\'s handwriting. The fingerprint is underlined twice.' },
  { id: 'gateway-key', name: 'Gateway identity', desc: "Grandpa's Taproot key, derived from the recovered seed at m/86'/0'/0'/0/0." },
  { id: 'share1', name: 'Share #1 (Mara)', desc: 'A Shamir share of the Vault key, written on a waterproof card.' },
  { id: 'share2', name: 'Share #2 (Felix)', desc: 'A Shamir share of the Vault key, read out over a secured line.' },
  { id: 'share3', name: 'Share #3 (dead drop)', desc: 'Found behind a loose brick in the Commons. The chalk mark looked fresh.' },
  { id: 'share4', name: 'Share #4 (dead drop)', desc: 'Found in the hollow of the old oak in the Commons.' },
  { id: 'vault-key', name: 'Vault key', desc: 'Reconstructed from three good shares. Opens the Legacy vault.' },
  { id: 'identity', name: 'Your identity key', desc: 'A key Grandpa generated for you long ago and kept in the vault.' },
  { id: 'warden-key', name: "Warden's signing key", desc: 'Recovered from two signatures that shared a nonce.' },
  { id: 'archive', name: 'The Archive', desc: 'Decades of cypherpunk writing and code, released from the Mint.' },
  { id: 'checkpoint-key', name: 'Checkpoint master key', desc: 'Extracted by rewinding a careless prover.' },
  { id: 'pq-keys', name: 'ML-KEM keypair', desc: 'A post-quantum key for the Legacy capsule.' },
];

export function itemDef(id: string): ItemDef | undefined {
  const d = ITEMS.find((i) => i.id === id);
  if (!d) return undefined;
  const fn = detailFns.get(id);
  return fn ? { ...d, details: fn } : d;
}

export function heldItems(): ItemDef[] {
  return state.data.items.map(itemDef).filter((d): d is ItemDef => Boolean(d));
}
