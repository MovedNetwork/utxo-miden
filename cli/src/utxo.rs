use crate::utils::HexString;
use miden_crypto::{
    dsa::rpo_falcon512::{FalconError, KeyPair, Signature},
    hash::rpo::Rpo256,
    merkle::MerkleTree,
    Felt, Word,
};
use std::fmt;
use winter_utils::{Deserializable, Serializable};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(try_from = "SerializedKey", into = "SerializedKey")]
pub struct Key {
    pub pair: KeyPair,
    pub owner: Word,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(try_from = "SerializedUtxo", into = "SerializedUtxo")]
pub struct Utxo {
    pub owner: Word,
    pub value: Felt,
}
impl Utxo {
    pub fn serialize(&self) -> Vec<Felt> {
        let mut output = Vec::with_capacity(5);
        self.serialize_inner(&mut output);
        output
    }

    pub fn hash(&self) -> Word {
        let elems = self.serialize();
        let h = Rpo256::hash_elements(&elems);
        h.into()
    }

    fn serialize_inner(&self, target: &mut Vec<Felt>) {
        for e in self.owner {
            target.push(e);
        }
        target.push(self.value);
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(try_from = "SerializedTransaction", into = "SerializedTransaction")]
pub struct Transaction {
    /// Hash of input UTXO.
    pub input: Word,
    /// List of newly created UTXOs.
    /// It must be true that `outputs.map(|x| x.value).sum() <= input.value`
    /// (less than or equal because burning is allowed)
    pub outputs: Vec<Utxo>,
}

impl Transaction {
    pub fn to_elems(&self) -> Vec<Felt> {
        let mut elems = Vec::with_capacity(4 + 5 * self.outputs.len());
        for e in self.input {
            elems.push(e);
        }
        for u in self.outputs.iter() {
            u.serialize_inner(&mut elems);
        }
        elems
    }

    pub fn hash(&self) -> Word {
        let elems = self.to_elems();
        let h = Rpo256::hash_elements(&elems);
        h.into()
    }

    pub fn verify(&self, input: &Utxo) -> Result<(), TransactionError> {
        if input.hash() != self.input {
            return Err(TransactionError::InvalidInputHash);
        }
        let total_output: u64 = self.outputs.iter().map(|u| u.value.inner()).sum();
        if total_output > input.value.inner() {
            return Err(TransactionError::ExcessiveOutput);
        }
        Ok(())
    }
}

pub struct SignedTransaction {
    pub transaction: Transaction,
    /// `transaction.input.owner.verify(transaction.hash(), signature)` must return `true`.
    pub signature: Signature,
}

impl SignedTransaction {
    pub fn new(transaction: Transaction, key: KeyPair) -> Result<Self, FalconError> {
        let message = transaction.hash();
        let signature = key.sign(message)?;
        Ok(Self {
            transaction,
            signature,
        })
    }

    pub fn verify(&self, input: &Utxo) -> Result<(), TransactionError> {
        self.transaction.verify(input)?;
        if !self.signature.verify(self.transaction.hash(), input.owner) {
            return Err(TransactionError::InvalidSignature);
        }
        Ok(())
    }
}

/// State of the UTXO system.
/// It can only hold up to `Self::MAX_SIZE` UTXOs (after that transactions must have 0 or 1 outputs)
/// because the set of UTXOs must fit in a binary Merkle tree of fixed depth.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct State {
    pub tree: MerkleTree,
    pub utxos: Vec<Utxo>,
}

impl State {
    const MAX_SIZE: usize = 8;

    pub fn empty() -> Self {
        // Safety: unwrap is safe because `Self::MAX_SIZE` is a power of 2 greater than 1.
        let tree = MerkleTree::new(vec![Word::default(); Self::MAX_SIZE]).unwrap();
        let utxos = Vec::with_capacity(Self::MAX_SIZE);
        Self { tree, utxos }
    }

    pub fn process_tx(&mut self, transaction: SignedTransaction) -> Result<(), StateError> {
        let tx = &transaction.transaction;

        // Verify transaction
        let (input_tree_index, _) = self
            .tree
            .leaves()
            .find(|(_, hash)| *hash == &tx.input)
            .ok_or(StateError::UnknownUtxoHash)?;
        let input_vec_index = self
            .utxos
            .iter()
            .position(|u| u.hash() == tx.input)
            .ok_or(StateError::UnknownUtxoHash)?;
        // Safety: unwrap is safe because index comes from the Vec itself.
        let input = self.utxos.get(input_vec_index).unwrap();
        transaction.verify(input)?;

        // Remove spent UTXO
        self.utxos.swap_remove(input_vec_index);
        // Safety: unwrap is safe because index came from the tree itself.
        self.tree
            .update_leaf(input_tree_index, Word::default())
            .unwrap();

        // Insert output UTXOs
        for u in transaction.transaction.outputs {
            self.insert(u)?;
        }

        Ok(())
    }

    pub fn get_root(&self) -> Word {
        self.tree.root().into()
    }

    pub fn insert(&mut self, utxo: Utxo) -> Result<(), StateError> {
        let (index, _) = self
            .tree
            .leaves()
            .find(|(_, hash)| *hash == &Word::default())
            .ok_or(StateError::Full)?;
        let h = utxo.hash();
        // Safety: unwrap is safe because index came from the tree itself.
        self.tree.update_leaf(index, h).unwrap();
        self.utxos.push(utxo);
        Ok(())
    }
}

#[derive(Debug)]
pub enum TransactionError {
    InvalidInputHash,
    ExcessiveOutput,
    InvalidSignature,
}

impl fmt::Display for TransactionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for TransactionError {}

#[derive(Debug)]
pub enum StateError {
    Full,
    UnknownUtxoHash,
    InvalidTransaction(TransactionError),
}

impl fmt::Display for StateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for StateError {}

impl From<TransactionError> for StateError {
    fn from(value: TransactionError) -> Self {
        Self::InvalidTransaction(value)
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct SerializedKey {
    pub pair: HexString,
    pub owner: HexString,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct SerializedUtxo {
    pub owner: HexString,
    pub value: HexString,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct SerializedTransaction {
    pub input: HexString,
    pub outputs: Vec<SerializedUtxo>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct SerializedSignedTransaction {
    pub transaction: SerializedTransaction,
    /// `transaction.input.owner.verify(transaction.hash(), signature)` must return `true`.
    pub signature: HexString,
}

impl TryFrom<SerializedKey> for Key {
    type Error = anyhow::Error;

    fn try_from(value: SerializedKey) -> Result<Self, Self::Error> {
        let owner = value.owner.try_into()?;
        let pair = KeyPair::read_from_bytes(&value.pair.bytes)
            .map_err(|e| anyhow::Error::msg(format!("{e:?}")))?;
        Ok(Self { pair, owner })
    }
}

impl From<Key> for SerializedKey {
    fn from(value: Key) -> Self {
        let owner = value.owner.into();
        let pair_bytes = {
            let mut buf = Vec::new();
            value.pair.write_into(&mut buf);
            buf
        };
        Self {
            pair: HexString { bytes: pair_bytes },
            owner,
        }
    }
}

impl TryFrom<SerializedUtxo> for Utxo {
    type Error = anyhow::Error;

    fn try_from(utxo: SerializedUtxo) -> Result<Self, Self::Error> {
        let owner = utxo.owner.try_into()?;
        let value = utxo.value.try_into()?;
        Ok(Self { owner, value })
    }
}

impl From<Utxo> for SerializedUtxo {
    fn from(value: Utxo) -> Self {
        let owner = value.owner.into();
        let value_bytes = {
            let mut buf = Vec::new();
            value.value.write_into(&mut buf);
            buf
        };
        Self {
            owner,
            value: HexString { bytes: value_bytes },
        }
    }
}

impl TryFrom<SerializedTransaction> for Transaction {
    type Error = anyhow::Error;

    fn try_from(tx: SerializedTransaction) -> Result<Self, Self::Error> {
        let input = tx.input.try_into()?;
        let outputs: anyhow::Result<Vec<Utxo>> =
            tx.outputs.into_iter().map(|utxo| utxo.try_into()).collect();
        Ok(Self {
            input,
            outputs: outputs?,
        })
    }
}

impl From<Transaction> for SerializedTransaction {
    fn from(value: Transaction) -> Self {
        let input = value.input.into();
        let outputs = value.outputs.into_iter().map(Into::into).collect();
        Self { input, outputs }
    }
}

impl TryFrom<SerializedSignedTransaction> for SignedTransaction {
    type Error = anyhow::Error;

    fn try_from(value: SerializedSignedTransaction) -> Result<Self, Self::Error> {
        let transaction = value.transaction.try_into()?;
        let signature = Signature::read_from_bytes(&value.signature.bytes)
            .map_err(|e| anyhow::Error::msg(format!("Failed to deserialize signature: {e:?}")))?;
        Ok(Self {
            transaction,
            signature,
        })
    }
}

impl From<SignedTransaction> for SerializedSignedTransaction {
    fn from(value: SignedTransaction) -> Self {
        let transaction = value.transaction.into();
        let signature = value.signature.to_bytes();
        Self {
            transaction,
            signature: HexString { bytes: signature },
        }
    }
}
