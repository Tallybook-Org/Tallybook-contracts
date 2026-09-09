// Unused until verify_usage() (build sequence step 38) calls it.
#![allow(dead_code)]

use soroban_sdk::{Bytes, BytesN, Env, Vec};

/// Folds `leaf` up through `proof` using sorted-pair hashing: at each step,
/// hashes the concatenation of the two 32-byte nodes ordered by byte
/// comparison — `sha256(min(a, b) || max(a, b))`. No leaf index is needed,
/// which makes index off-by-one bugs structurally impossible. Returns the
/// resulting root.
///
/// The leaf format itself is fixed by the off-chain collector and is not
/// computed here — this function only folds whatever leaf it is given.
pub fn fold(env: &Env, leaf: BytesN<32>, proof: &Vec<BytesN<32>>) -> BytesN<32> {
    let mut node = leaf;
    for sibling in proof.iter() {
        let (lo, hi) = if node <= sibling { (node, sibling) } else { (sibling, node) };
        let mut concatenated = Bytes::from(lo);
        concatenated.append(&Bytes::from(hi));
        node = env.crypto().sha256(&concatenated).to_bytes();
    }
    node
}
