// Host-selected Choregraph target registry for the construction route; included by `construction.rs`.

const JOB_CONSTRUCTION_VALUE_TYPE: &str = "u64-sequence";
const JOB_CONSTRUCTION_UNIT_TYPE: &str = "Unit";
const JOB_CONSTRUCTION_SOURCE_FIELD: &str = "values";
const JOB_CONSTRUCTION_SINGLE_INPUT_OPERATIONS: [&str; 5] =
    ["identity-u64", "concat-u64", "partition-even-u64", "drop-u64", "sum-u64"];

/// The host-selected Choregraph registry for `selected-sequence-v2`. Each implementation is the
/// exact Aspen stage signature, so changing a built-in changes the registry and every binding.
/// Author labels never select it.
pub fn job_construction_registry() -> Result<choregraph_nickel::construction::model::host::TargetRegistry> {
    let boundary = |name: &str| choregraph_nickel::construction::model::interface::Boundary {
        name: name.to_owned(),
        value_type: JOB_CONSTRUCTION_VALUE_TYPE.to_owned(),
    };
    let signature = |operation: &str,
                     implementation: &str|
     -> Result<choregraph_nickel::construction::model::host::OperationSignature> {
        Ok(choregraph_nickel::construction::model::host::OperationSignature {
            operation: operation.to_owned(),
            implementation: selected_stage_signature(implementation)?,
            inputs: Vec::new(),
            outputs: vec![boundary("out")],
            invocation: Vec::new(),
            configuration: Vec::new(),
            effects: Vec::new(),
        })
    };
    let mut operations = Vec::with_capacity(JOB_CONSTRUCTION_SINGLE_INPUT_OPERATIONS.len() + 2);
    let mut source = signature("source-u64", "source-u64")?;
    source.configuration.push(choregraph_nickel::construction::model::host::ConfigurationField {
        name: JOB_CONSTRUCTION_SOURCE_FIELD.to_owned(),
        value_type: "array".to_owned(),
        references: false,
        operators: Vec::new(),
    });
    operations.push(source);
    for operation in JOB_CONSTRUCTION_SINGLE_INPUT_OPERATIONS {
        let mut selected = signature(operation, operation)?;
        selected.inputs.push(boundary("in"));
        if operation == "partition-even-u64" {
            selected.outputs = vec![boundary("left"), boundary("right")];
        }
        operations.push(selected);
    }
    let mut join = signature(JOB_CONSTRUCTION_SUM_JOIN, "sum-u64")?;
    join.inputs = vec![boundary("left"), boundary("right")];
    operations.push(join);
    let types = vec![
        choregraph_nickel::construction::model::host::ValueType {
            name: JOB_CONSTRUCTION_UNIT_TYPE.to_owned(),
            copyable: true,
            droppable: true,
        },
        // Aspen u64 sequences fan out without re-execution, and unread outputs are allowed.
        choregraph_nickel::construction::model::host::ValueType {
            name: JOB_CONSTRUCTION_VALUE_TYPE.to_owned(),
            copyable: true,
            droppable: true,
        },
    ];
    let selected = serde_json::to_vec(&(&types, &operations))
        .map_err(|error| MoltenError::invalid_harness(format!("construction registry encoding: {error}")))?;
    Ok(choregraph_nickel::construction::model::host::TargetRegistry {
        schema: choregraph_nickel::construction::model::profile::REGISTRY_SCHEMA.to_owned(),
        target: JOB_CONSTRUCTION_TARGET.to_owned(),
        profile: SELECTED_SEQUENCE_PROFILE.to_owned(),
        implementation: crate::preserves_rail::content_ref_from_bytes(&selected),
        types,
        operations,
    })
}
