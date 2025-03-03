use crate::utxo::{SignedTransaction, State, Utxo};
use miden::{math::Felt, AdviceInputs, AdviceProvider, ExecutionError, MemAdviceProvider, Word};
use miden_core::SignatureKind;
use miden_crypto::{dsa::rpo_falcon512::Polynomial, merkle::MerkleStore, StarkField};
use miden_processor::ProcessState;
use std::collections::{BTreeMap, HashMap};

pub struct UtxoAdvice {
    inner: MemAdviceProvider,
    known_transactions: HashMap<[u64; 4], SignedTransaction>,
    known_utxos: HashMap<[u64; 4], Utxo>,
}

impl UtxoAdvice {
    pub fn new(state: &State, signed_tx: SignedTransaction) -> Option<Self> {
        // Merkle store contains the state
        let mut merkle_store = MerkleStore::default();
        merkle_store.extend(state.tree.inner_nodes());

        let mut known_utxos = HashMap::new();
        let input_utxo = state.utxos.iter().find(|u| u.hash() == signed_tx.transaction.input)?;
        known_utxos.insert(raw_word(signed_tx.transaction.input), input_utxo.clone());

        // Starting UTXO in advice stack for signature verification
        let mut utxo_elems = input_utxo.owner.to_vec();
        utxo_elems.push(input_utxo.value);

        // Transaction input hash and output UTXOs are in the advice map
        let mut map: BTreeMap<[u8; 32], Vec<Felt>> = BTreeMap::new();
        let mut key = [0u8; 32];
        map.insert(key, signed_tx.transaction.input.to_vec());
        signed_tx.transaction.outputs.iter().enumerate().for_each(|(i, utxo)| {
            key[0] = i as u8 + 1;
            map.insert(key, utxo.serialize());
        });

        let mut known_transactions = HashMap::new();
        let key = raw_word(signed_tx.transaction.hash());
        known_transactions.insert(key, signed_tx);

        let advice_inputs = AdviceInputs::default()
            .with_stack(utxo_elems)
            .with_map(map)
            .with_merkle_store(merkle_store);

        Some(Self {
            inner: MemAdviceProvider::from(advice_inputs),
            known_transactions,
            known_utxos,
        })
    }
}

impl AdviceProvider for UtxoAdvice {
    fn pop_stack<S: ProcessState>(&mut self, process: &S) -> Result<Felt, ExecutionError> {
        self.inner.pop_stack(process)
    }

    fn pop_stack_word<S: ProcessState>(&mut self, process: &S) -> Result<Word, ExecutionError> {
        self.inner.pop_stack_word(process)
    }

    fn pop_stack_dword<S: ProcessState>(
        &mut self,
        process: &S,
    ) -> Result<[Word; 2], ExecutionError> {
        self.inner.pop_stack_dword(process)
    }

    fn push_stack(&mut self, source: miden_processor::AdviceSource) -> Result<(), ExecutionError> {
        self.inner.push_stack(source)
    }

    fn get_mapped_values(&self, key: &[u8; 32]) -> Option<&[Felt]> {
        self.inner.get_mapped_values(key)
    }

    fn insert_into_map(&mut self, key: Word, values: Vec<Felt>) -> Result<(), ExecutionError> {
        self.inner.insert_into_map(key, values)
    }

    fn get_signature(
        &self,
        kind: SignatureKind,
        pub_key: Word,
        msg: Word,
    ) -> Result<Vec<Felt>, ExecutionError> {
        match kind {
            SignatureKind::RpoFalcon512 => {
                let key = raw_word(msg);
                let signed_tx = self.known_transactions.get(&key).ok_or_else(|| {
                    ExecutionError::FailedSignatureGeneration("Unknown transaction hash")
                })?;
                let key = raw_word(signed_tx.transaction.input);
                let input_utxo = self.known_utxos.get(&key).ok_or_else(|| {
                    ExecutionError::FailedSignatureGeneration("Unknown input utxo")
                })?;
                if input_utxo.owner != pub_key {
                    return Err(ExecutionError::FailedSignatureGeneration(
                        "Invalid pub key for transaction",
                    ));
                }
                let sig = &signed_tx.signature;

                // For details on this signature post-processing, see
                // ...
                let nonce = sig.nonce();
                let s2 = sig.sig_poly();
                let h = sig.pub_key_poly();
                let pi = Polynomial::mul_modulo_p(&h, &s2);

                let mut result: Vec<Felt> = nonce.to_vec();
                result.extend(h.inner().iter().map(|a| Felt::from(*a)));
                result.extend(s2.inner().iter().map(|a| Felt::from(*a)));
                result.extend(pi.iter().map(|a| Felt::new(*a)));
                result.reverse();
                Ok(result)
            }
        }
    }

    fn get_tree_node(
        &self,
        root: Word,
        depth: &Felt,
        index: &Felt,
    ) -> Result<Word, ExecutionError> {
        self.inner.get_tree_node(root, depth, index)
    }

    fn get_merkle_path(
        &self,
        root: Word,
        depth: &Felt,
        index: &Felt,
    ) -> Result<miden_crypto::merkle::MerklePath, ExecutionError> {
        self.inner.get_merkle_path(root, depth, index)
    }

    fn get_leaf_depth(
        &self,
        root: Word,
        tree_depth: &Felt,
        index: &Felt,
    ) -> Result<u8, ExecutionError> {
        self.inner.get_leaf_depth(root, tree_depth, index)
    }

    fn find_lone_leaf(
        &self,
        root: Word,
        root_index: miden_crypto::merkle::NodeIndex,
        tree_depth: u8,
    ) -> Result<Option<(miden_crypto::merkle::NodeIndex, Word)>, ExecutionError> {
        self.inner.find_lone_leaf(root, root_index, tree_depth)
    }

    fn update_merkle_node(
        &mut self,
        root: Word,
        depth: &Felt,
        index: &Felt,
        value: Word,
    ) -> Result<(miden_crypto::merkle::MerklePath, Word), ExecutionError> {
        self.inner.update_merkle_node(root, depth, index, value)
    }

    fn merge_roots(&mut self, lhs: Word, rhs: Word) -> Result<Word, ExecutionError> {
        self.inner.merge_roots(lhs, rhs)
    }

    fn get_store_subset<I, R>(&self, roots: I) -> miden_crypto::merkle::MerkleStore
    where
        I: Iterator<Item = R>,
        R: std::borrow::Borrow<miden_processor::Digest>,
    {
        self.inner.get_store_subset(roots)
    }
}

fn raw_word(word: Word) -> [u64; 4] {
    let mut output = [0; 4];
    for (el, o) in word.into_iter().zip(output.iter_mut()) {
        *o = el.as_int();
    }
    output
}
