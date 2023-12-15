use crate::{config::Config, inputs::InputFile};
use miden::{
    crypto::{MerkleStore, MerkleTree},
    math::Felt,
    AdviceInputs, Assembler, DefaultHost, ExecutionProof, MemAdviceProvider, ProvingOptions,
    StackInputs, StackOutputs, Word,
};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ProveOutput {
    pub program_hash: [u8; 32],
    pub stack_outputs: StackOutputs,
    pub proof: ExecutionProof,
}

pub fn execute(config: &Config) -> anyhow::Result<ProveOutput> {
    let code = std::fs::read_to_string(&config.code_path)?;
    let assembler = Assembler::default();
    let program = assembler.compile(code)?;
    let input_file = InputFile::parse(&config.inputs_path)?;
    let stack_inputs = StackInputs::try_from_values(input_file.operand_stack.into_iter())?;
    let advice_inputs = create_advice_inputs(input_file.merkle_tree.unwrap_or_default())?;
    let host = DefaultHost::new(MemAdviceProvider::from(advice_inputs));

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

fn create_advice_inputs(merkle_data: Vec<[u8; 32]>) -> anyhow::Result<AdviceInputs> {
    let leaves: anyhow::Result<Vec<Word>> = merkle_data
        .into_iter()
        .map(|bytes| {
            let mut word = Word::default();
            for (w, value) in word.iter_mut().zip(bytes.chunks_exact(8)) {
                *w = Felt::try_from(value).map_err(|e| anyhow::Error::msg(format!("{e:?}")))?;
            }
            Ok(word)
        })
        .collect();
    let tree = MerkleTree::new(leaves?)?;
    let merkle_store = {
        let mut tmp = MerkleStore::default();
        tmp.extend(tree.inner_nodes());
        tmp
    };
    Ok(AdviceInputs::default().with_merkle_store(merkle_store))
}
