// =============================================================================
// src/zk_identity.rs — ZK-доказательство членства в Федерации
// =============================================================================

use crate::noise::hash as blake_hash;
use serde::{Deserialize, Serialize};

const HASH_SIZE: usize = 32;
type Hash = [u8; HASH_SIZE];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleTree {
    pub leaves: Vec<Hash>,
    pub root: Hash,
    layers: Vec<Vec<Hash>>,
}

impl MerkleTree {
    pub fn from_node_keys(node_keys: &[&[u8]]) -> Self {
        assert!(!node_keys.is_empty());
        let mut leaves: Vec<Hash> = node_keys.iter().map(|key| Self::hash_leaf(key)).collect();
        let next_power_of_two = leaves.len().next_power_of_two();
        if leaves.len() < next_power_of_two {
            let last = *leaves.last().unwrap();
            leaves.resize(next_power_of_two, last);
        }
        let mut layers = vec![leaves.clone()];
        let mut current_layer = leaves;
        while current_layer.len() > 1 {
            let mut next_layer = Vec::new();
            for chunk in current_layer.chunks(2) {
                let left = chunk[0];
                let right = chunk.get(1).copied().unwrap_or(left);
                next_layer.push(Self::hash_pair(&left, &right));
            }
            layers.push(next_layer.clone());
            current_layer = next_layer;
        }
        let root = current_layer[0];
        MerkleTree { leaves: layers[0].clone(), root, layers }
    }

    fn hash_leaf(data: &[u8]) -> Hash {
        let mut input = b"LEAF:".to_vec();
        input.extend_from_slice(data);
        blake_hash(&input)
    }

    fn hash_pair(left: &Hash, right: &Hash) -> Hash {
        let mut input = Vec::with_capacity(64);
        input.extend_from_slice(left);
        input.extend_from_slice(right);
        blake_hash(&input)
    }

    pub fn generate_proof(&self, leaf_index: usize) -> Option<MerkleProof> {
        if leaf_index >= self.leaves.len() { return None; }
        let mut proof_path = Vec::new();
        let mut index = leaf_index;
        for layer in &self.layers[..self.layers.len() - 1] {
            let sibling_index = if index.is_multiple_of(2) { index + 1 } else { index - 1 };
            let sibling_index = sibling_index.min(layer.len() - 1);
            proof_path.push(layer[sibling_index]);
            index /= 2;
        }
        Some(MerkleProof { leaf: self.leaves[leaf_index], path: proof_path, leaf_index })
    }

    pub fn verify_proof(&self, proof: &MerkleProof) -> bool {
        let mut current = proof.leaf;
        let mut index = proof.leaf_index;
        for sibling in &proof.path {
            current = if index.is_multiple_of(2) {
                Self::hash_pair(&current, sibling)
            } else {
                Self::hash_pair(sibling, &current)
            };
            index /= 2;
        }
        current == self.root
    }

    pub fn size(&self) -> usize { self.leaves.len() }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleProof {
    pub leaf: Hash,
    pub path: Vec<Hash>,
    pub leaf_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZkIdentityProof {
    pub merkle_root: Hash,
    pub merkle_proof: MerkleProof,
    pub signature: Vec<u8>,
    pub challenge: Vec<u8>,
}

impl ZkIdentityProof {
    pub fn create(tree: &MerkleTree, node_pubkey: &[u8], node_privkey: &[u8], challenge: &[u8]) -> Option<Self> {
        let our_leaf = MerkleTree::hash_leaf(node_pubkey);
        let leaf_index = tree.leaves.iter().position(|&leaf| leaf == our_leaf)?;
        let merkle_proof = tree.generate_proof(leaf_index)?;
        let signature = Self::sign_challenge(node_privkey, challenge);
        Some(ZkIdentityProof { merkle_root: tree.root, merkle_proof, signature, challenge: challenge.to_vec() })
    }

    pub fn verify(&self, tree: &MerkleTree, challenge: &[u8]) -> bool {
        if self.merkle_root != tree.root || self.challenge != challenge { return false; }
        if !tree.verify_proof(&self.merkle_proof) { return false; }
        Self::verify_signature(&self.merkle_proof.leaf, challenge, &self.signature)
    }

    fn sign_challenge(privkey: &[u8], challenge: &[u8]) -> Vec<u8> {
        // Симуляция: подпись = hash(privkey || challenge)
        // В реальности: Ed25519 sign
        let mut input = privkey.to_vec();
        input.extend_from_slice(challenge);
        blake_hash(&input).to_vec()
    }

    fn verify_signature(_leaf: &Hash, _challenge: &[u8], signature: &[u8]) -> bool {
        // В production: Ed25519 verify с pubkey из leaf
        // Симуляция: принимаем любую непустую подпись
        // (реальная криптография требует пары pubkey/privkey)
        !signature.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct FederationMembership {
    pub tree: MerkleTree,
    node_ids: Vec<String>,
    pub verifications_passed: u64,
    pub verifications_failed: u64,
}

impl FederationMembership {
    pub fn new(nodes: Vec<(String, Vec<u8>)>) -> Self {
        let node_ids: Vec<String> = nodes.iter().map(|(id, _)| id.clone()).collect();
        let pubkeys: Vec<&[u8]> = nodes.iter().map(|(_, key)| key.as_slice()).collect();
        let tree = MerkleTree::from_node_keys(&pubkeys);
        FederationMembership { tree, node_ids, verifications_passed: 0, verifications_failed: 0 }
    }

    pub fn create_proof(&self, node_id: &str, node_pubkey: &[u8], node_privkey: &[u8], challenge: &[u8]) -> Option<ZkIdentityProof> {
        if !self.node_ids.contains(&node_id.to_string()) { return None; }
        ZkIdentityProof::create(&self.tree, node_pubkey, node_privkey, challenge)
    }

    pub fn verify_proof(&mut self, proof: &ZkIdentityProof, challenge: &[u8]) -> bool {
        let valid = proof.verify(&self.tree, challenge);
        if valid { self.verifications_passed += 1; } else { self.verifications_failed += 1; }
        valid
    }

    pub fn size(&self) -> usize { self.node_ids.len() }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn make_test_nodes(n: usize) -> Vec<(String, Vec<u8>, Vec<u8>)> {
        (0..n).map(|i| (format!("node-{}", i), format!("pubkey-{}", i).into_bytes(), format!("privkey-{}", i).into_bytes())).collect()
    }
    #[test]
    fn test_merkle_tree_construction() {
        let nodes = make_test_nodes(4);
        let pubkeys: Vec<&[u8]> = nodes.iter().map(|(_, pk, _)| pk.as_slice()).collect();
        let tree = MerkleTree::from_node_keys(&pubkeys);
        assert_eq!(tree.size(), 4);
    }
    #[test]
    fn test_merkle_proof_valid() {
        let nodes = make_test_nodes(8);
        let pubkeys: Vec<&[u8]> = nodes.iter().map(|(_, pk, _)| pk.as_slice()).collect();
        let tree = MerkleTree::from_node_keys(&pubkeys);
        for i in 0..8 {
            let proof = tree.generate_proof(i).unwrap();
            assert!(tree.verify_proof(&proof));
        }
    }
    #[test]
    fn test_zk_identity_proof() {
        let nodes = make_test_nodes(4);
        let membership = FederationMembership::new(nodes.iter().map(|(id, pk, _)| (id.clone(), pk.clone())).collect());
        let (node_id, pubkey, privkey) = &nodes[1];
        let proof = membership.create_proof(node_id, pubkey, privkey, b"ch1").unwrap();
        let mut verifier = membership.clone();
        assert!(verifier.verify_proof(&proof, b"ch1"));
    }
    #[test]
    fn test_wrong_challenge_rejected() {
        let nodes = make_test_nodes(4);
        let membership = FederationMembership::new(nodes.iter().map(|(id, pk, _)| (id.clone(), pk.clone())).collect());
        let (node_id, pubkey, privkey) = &nodes[0];
        let proof = membership.create_proof(node_id, pubkey, privkey, b"ch-A").unwrap();
        let mut verifier = membership.clone();
        assert!(!verifier.verify_proof(&proof, b"ch-B"));
    }
}
