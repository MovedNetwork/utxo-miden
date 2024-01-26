use crate::{
    advice_provider::UtxoAdvice,
    config::Config,
    utils,
    utxo::{SignedTransaction, State},
};
use anyhow::Context;
use miden::{
    math::Felt, Assembler, DefaultHost, ExecutionProof, ProvingOptions, StackInputs, StackOutputs,
};
use miden_stdlib::StdLibrary;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ProveOutput {
    pub program_hash: [u8; 32],
    pub stack_outputs: StackOutputs,
    pub proof: ExecutionProof,
}

pub fn execute(config: &Config, signed_tx: SignedTransaction) -> anyhow::Result<ProveOutput> {
    let code = std::fs::read_to_string(&config.code_path)?;
    let assembler = Assembler::default().with_library(&StdLibrary::default())?;
    let program = assembler.compile(code)?;
    let state: State =
        utils::read_json_file(&config.state_path).context("Failed to read state file")?;

    let stack_inputs = prepare_stack_inputs(&state, &signed_tx);
    let advice_provider = UtxoAdvice::new(&state, signed_tx)
        .ok_or_else(|| anyhow::Error::msg("Input UTXO not present in the state"))?;
    let host = DefaultHost::new(advice_provider);

    let (stack_outputs, proof) =
        miden::prove(&program, stack_inputs, host, ProvingOptions::default())?;

    Ok(ProveOutput {
        program_hash: program.hash().into(),
        stack_outputs,
        proof,
    })
}

impl ProveOutput {
    pub fn write_to_file(&self, path: &Path) -> anyhow::Result<()> {
        let serialized_outputs: Vec<String> = self
            .stack_outputs
            .stack()
            .iter()
            .map(|x| x.to_string())
            .collect();
        let serialized_overflows: Vec<String> = self
            .stack_outputs
            .overflow_addrs()
            .iter()
            .map(|x| x.to_string())
            .collect();
        let serialized_proof = hex::encode(self.proof.to_bytes());
        let json_encoding = serde_json::json!({
            "program_hash": format!("0x{}", hex::encode(self.program_hash)),
            "stack_outputs": serialized_outputs,
            "overflow_addrs": serialized_overflows,
            "proof": serialized_proof,
        });

        let data = serde_json::to_string_pretty(&json_encoding)?;
        std::fs::write(path, data)?;

        Ok(())
    }
}

// The operand stack starts as transaction_size then transaction hash and finally state root
pub fn prepare_stack_inputs(state: &State, signed_tx: &SignedTransaction) -> StackInputs {
    let tx_size = Felt::new(signed_tx.transaction.to_elems().len() as u64);
    let transaction_hash = signed_tx.transaction.hash();
    let state_root = state.get_root();

    // Insert stack elements in reverse, stack top is at the rear
    let input_stack: Vec<Felt> = state_root
        .into_iter()
        .chain(transaction_hash.into_iter())
        .chain(std::iter::once(tx_size))
        .collect();

    StackInputs::new(input_stack)
}
