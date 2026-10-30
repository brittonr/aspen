use crate::support::*;

pub(super) fn operation(name: &str, fields: Vec<preserves::IOValue>) -> preserves::IOValue {
    preserves::IOValue::record(preserves::IOValue::symbol(format!("node-service-host-{name}-v1")), fields)
}

pub(super) fn work(value: &molten::system_extension::WorkPresentation) -> preserves::IOValue {
    molten::preserves_rail::record("node-service-host-work-v1", vec![
        molten::preserves_rail::string(&value.work_ref),
        molten::preserves_rail::string(value.class.as_str()),
        molten::preserves_rail::u64_value(value.generation),
        molten::preserves_rail::u64_value(value.incarnation),
    ])
}

pub(super) fn payload(value: &molten::system_extension::NativeCallbackValue) -> preserves::IOValue {
    molten::preserves_rail::record("node-service-host-value-v1", vec![
        molten::preserves_rail::string(&value.value_ref),
        molten::preserves_rail::sequence(
            value.bytes.iter().map(|byte| molten::preserves_rail::u64_value(u64::from(*byte))).collect(),
        ),
    ])
}

pub(super) fn outcome(value: &preserves::IOValue) -> TestResult<preserves::IOValue> {
    let fields = molten::preserves_rail::simple_record_fields(value, "node-service-host-receipt-v1", 5)?;
    Ok(molten::preserves_rail::value_to_iovalue(&fields[4]))
}

pub(super) fn text(value: &preserves::Value<preserves::IOValue>) -> TestResult<String> {
    Ok(molten::preserves_rail::required_string_field(value, "actual node observation")?)
}

pub(super) fn number(value: &preserves::Value<preserves::IOValue>) -> TestResult<u64> {
    value.as_u64().or_fail("actual unsigned observation")?.or_fail("bounded unsigned observation")
}

pub(super) fn optional(value: &preserves::Value<preserves::IOValue>) -> TestResult<Option<String>> {
    let values = molten::preserves_rail::required_sequence_field(value, "optional actual reference")?;
    if values.len() > 1 {
        return Err("malformed optional observation".into());
    }
    values.first().map(text).transpose()
}

#[derive(Debug, Clone)]
pub(super) struct Member {
    pub generation: u64,
    pub incarnation: u64,
    pub phase: String,
    pub accepting: bool,
    pub state_ref: Option<String>,
    pub checkpoint_ref: Option<String>,
    pub sequence: u64,
    pub manifest_ref: String,
}

#[derive(Debug, Clone)]
pub(super) struct LogicalWork {
    pub member: String,
    pub disposition: String,
    pub claim_ref: Option<String>,
    pub receipt_ref: Option<String>,
    pub generation: u64,
    pub incarnation: u64,
    pub payload_ref: String,
    pub lineage: Vec<String>,
}

pub(super) struct Group {
    pub terminal: bool,
    pub pending: u64,
    pub history: Vec<(String, u64)>,
}

pub(super) struct Snapshot {
    pub members: std::collections::BTreeMap<String, Member>,
    pub work: std::collections::BTreeMap<String, LogicalWork>,
    pub groups: std::collections::BTreeMap<String, Group>,
}

impl Snapshot {
    pub(super) fn parse(value: &preserves::IOValue) -> TestResult<Self> {
        let fields = molten::preserves_rail::simple_record_fields(value, "node-service-host-status-v1", 4)?;
        let mut result = Self {
            members: std::collections::BTreeMap::new(),
            work: std::collections::BTreeMap::new(),
            groups: std::collections::BTreeMap::new(),
        };
        for value in molten::preserves_rail::required_sequence_field(&fields[2], "members")?.iter() {
            let value = molten::preserves_rail::value_to_iovalue(value);
            let fields =
                value.collect_simple_record("host-member-status-v1", None).or_fail("member observation record")?;
            if fields.fields_iter().count() < 9 {
                return Err("incomplete member observation".into());
            }
            let member = Member {
                generation: number(&fields[1])?,
                incarnation: number(&fields[2])?,
                phase: text(&fields[3])?,
                accepting: fields[4].as_boolean().or_fail("actual ingress fence")?,
                state_ref: optional(&fields[5])?,
                checkpoint_ref: optional(&fields[6])?,
                sequence: number(&fields[7])?,
                manifest_ref: text(&fields[8])?,
            };
            if result.members.insert(text(&fields[0])?, member).is_some() {
                return Err("duplicate member observation".into());
            }
        }
        for value in molten::preserves_rail::required_sequence_field(&fields[3], "work")?.iter() {
            let value = molten::preserves_rail::value_to_iovalue(value);
            let fields = molten::preserves_rail::simple_record_fields(&value, "host-work-status-v1", 10)?;
            let lineage = molten::preserves_rail::required_sequence_field(&fields[9], "migration lineage")?
                .iter()
                .map(|value| {
                    let value = molten::preserves_rail::value_to_iovalue(value);
                    let fields = molten::preserves_rail::simple_record_fields(&value, "host-work-lineage-v1", 5)?;
                    text(&fields[3])
                })
                .collect::<TestResult<Vec<_>>>()?;
            let work = LogicalWork {
                member: text(&fields[0])?,
                disposition: text(&fields[2])?,
                claim_ref: optional(&fields[3])?,
                receipt_ref: optional(&fields[4])?,
                generation: number(&fields[6])?,
                incarnation: number(&fields[7])?,
                payload_ref: text(&fields[8])?,
                lineage,
            };
            if result.work.insert(text(&fields[1])?, work).is_some() {
                return Err("duplicate work observation".into());
            }
        }
        for value in molten::preserves_rail::required_sequence_field(&fields[1], "groups")?.iter() {
            let value = molten::preserves_rail::value_to_iovalue(value);
            let fields = molten::preserves_rail::simple_record_fields(&value, "host-group-status-v1", 8)?;
            let history = molten::preserves_rail::required_sequence_field(&fields[7], "restart history")?
                .iter()
                .map(|value| {
                    let value = molten::preserves_rail::value_to_iovalue(value);
                    let fields = molten::preserves_rail::simple_record_fields(&value, "host-attempt-v1", 2)?;
                    Ok((text(&fields[0])?, number(&fields[1])?))
                })
                .collect::<TestResult<Vec<_>>>()?;
            let group = Group {
                terminal: fields[4].as_boolean().or_fail("terminal group observation")?,
                pending: number(&fields[3])?,
                history,
            };
            if result.groups.insert(text(&fields[0])?, group).is_some() {
                return Err("duplicate group observation".into());
            }
        }
        Ok(result)
    }
}
