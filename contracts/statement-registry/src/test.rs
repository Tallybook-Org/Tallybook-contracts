#![cfg(test)]

// The crate is #![no_std]; tests need std for the standard test harness and
// for building throwaway data like the leaf array below.
extern crate std;

use soroban_sdk::{Bytes, BytesN, Env};

/// sha256(min(a, b) || max(a, b)) — a reference implementation independent
/// of merkle::fold, so tests assert against a computation that doesn't
/// share fold's own logic.
fn hash_pair(env: &Env, a: &BytesN<32>, b: &BytesN<32>) -> BytesN<32> {
    let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
    let mut concatenated = Bytes::from(lo.clone());
    concatenated.append(&Bytes::from(hi.clone()));
    env.crypto().sha256(&concatenated).to_bytes()
}

mod merkle {
    use soroban_sdk::vec;

    use crate::merkle::fold;

    use super::*;

    #[test]
    fn single_leaf_with_empty_proof_returns_the_leaf_unchanged() {
        let env = Env::default();
        let leaf = BytesN::from_array(&env, &[7u8; 32]);

        let root = fold(&env, leaf.clone(), &vec![&env]);

        assert_eq!(root, leaf);
    }

    #[test]
    fn two_leaf_tree_matches_a_hand_computed_root() {
        let env = Env::default();
        let leaf0 = BytesN::from_array(&env, &[1u8; 32]);
        let leaf1 = BytesN::from_array(&env, &[2u8; 32]);
        let expected_root = hash_pair(&env, &leaf0, &leaf1);

        // Sorted-pair hashing means the root doesn't depend on which leaf
        // the proof starts from.
        assert_eq!(fold(&env, leaf0.clone(), &vec![&env, leaf1.clone()]), expected_root);
        assert_eq!(fold(&env, leaf1, &vec![&env, leaf0]), expected_root);
    }

    #[test]
    fn four_leaf_balanced_tree_matches_a_hand_computed_root() {
        let env = Env::default();
        let leaves: std::vec::Vec<BytesN<32>> =
            (0u8..4).map(|i| BytesN::from_array(&env, &[i + 1; 32])).collect();

        // Build the reference tree bottom-up: two internal nodes, then the
        // root over those.
        let node01 = hash_pair(&env, &leaves[0], &leaves[1]);
        let node23 = hash_pair(&env, &leaves[2], &leaves[3]);
        let expected_root = hash_pair(&env, &node01, &node23);

        let root = fold(&env, leaves[0].clone(), &vec![&env, leaves[1].clone(), node23.clone()]);
        assert_eq!(root, expected_root);

        // Every leaf's proof should fold to the same root.
        let root = fold(&env, leaves[2].clone(), &vec![&env, leaves[3].clone(), node01]);
        assert_eq!(root, expected_root);
    }
}
