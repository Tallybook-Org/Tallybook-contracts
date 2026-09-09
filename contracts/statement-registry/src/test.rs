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

mod fixtures {
    use serde_json::Value;
    use soroban_sdk::vec;

    use crate::merkle::fold;

    use super::*;

    /// Parses a 64-hex-character digest into a BytesN<32>. Panics on
    /// malformed input — test-only, and a fixture that doesn't parse is a
    /// fixture bug worth failing loudly on.
    fn hex_to_bytesn(env: &Env, hex: &str) -> BytesN<32> {
        let mut bytes = [0u8; 32];
        for (i, byte) in bytes.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap();
        }
        BytesN::from_array(env, &bytes)
    }

    /// Asserts every (leaf, proof) pair in the fixture folds to its root.
    fn assert_fixture_folds_to_root(json: &str) {
        let env = Env::default();
        let parsed: Value = serde_json::from_str(json).unwrap();
        let root = hex_to_bytesn(&env, parsed["root"].as_str().unwrap());

        for entry in parsed["proofs"].as_array().unwrap() {
            let leaf = hex_to_bytesn(&env, entry["leaf"].as_str().unwrap());
            let mut proof = vec![&env];
            for node in entry["proof"].as_array().unwrap() {
                proof.push_back(hex_to_bytesn(&env, node.as_str().unwrap()));
            }
            assert_eq!(fold(&env, leaf, &proof), root);
        }
    }

    #[test]
    fn single_leaf_fixture_folds_to_its_root() {
        assert_fixture_folds_to_root(include_str!("../../../fixtures/merkle/single-leaf.json"));
    }

    #[test]
    fn four_leaves_fixture_folds_to_its_root() {
        assert_fixture_folds_to_root(include_str!("../../../fixtures/merkle/four-leaves.json"));
    }

    #[test]
    fn seven_leaves_fixture_folds_to_its_root() {
        // The unbalanced tree: leaf index 6 is the odd one out at the first
        // level and gets a shorter proof than the rest — exactly the case
        // sorted-pair implementations tend to diverge on.
        assert_fixture_folds_to_root(include_str!("../../../fixtures/merkle/seven-leaves.json"));
    }

    #[test]
    fn corrupted_proof_node_does_not_reach_the_root() {
        let env = Env::default();
        let parsed: Value =
            serde_json::from_str(include_str!("../../../fixtures/merkle/four-leaves.json"))
                .unwrap();
        let root = hex_to_bytesn(&env, parsed["root"].as_str().unwrap());
        let entry = &parsed["proofs"][0];
        let leaf = hex_to_bytesn(&env, entry["leaf"].as_str().unwrap());

        let mut proof = vec![&env];
        for (i, node) in entry["proof"].as_array().unwrap().iter().enumerate() {
            let mut node_bytes = hex_to_bytesn(&env, node.as_str().unwrap()).to_array();
            if i == 0 {
                // Flip a bit in the first proof node — same shape as a
                // valid proof, wrong content.
                node_bytes[0] ^= 0xff;
            }
            proof.push_back(BytesN::from_array(&env, &node_bytes));
        }

        assert_ne!(fold(&env, leaf, &proof), root);
    }
}
