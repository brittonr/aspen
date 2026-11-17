#![expect(
    tigerstyle::path_segment_repetition,
    reason = "Owner: noble-maintainers; the module path names the domain and the item name keeps its canonical operation spelling"
)]

//! Parameter snapshot files `noble-ngraph-snapshot/0` (design D6).
//!
//! ```text
//! noble-ngraph-snapshot/0
//! snapshot <64 hex>            # ParamSnapshotId
//! graph <64 hex>               # GraphLogicalId
//! layout <64 hex>              # param layout key
//! param <slot> 0x<8 hex>       # one line per param slot
//! optimizer-state <64 hex>
//! parent <64 hex>
//! subject <kind> <64 hex> closure <ids|->
//! profile ngraph-f32/0 ngraph-sem/0
//! facts <64 hex>               # fact-binding digest of the model graph
//! loss <64 hex>
//! data <64 hex>
//! policy ngraph-deriv-real/0
//! opt-in %<node> boolean straight-through|clamped-linear 0x<8 hex>|refuse
//! opt-in %<node> index zero|backward-difference|refuse
//! seed 0x<16 hex>
//! prng splitmix64/0
//! optimizer adam/0 lr 0x<16 hex> beta1 0x<16 hex> beta2 0x<16 hex> eps 0x<16 hex>
//! steps <n>
//! tools <tool revisions>
//! provenance <64 hex>          # SnapshotFile::provenance_digest over every record above
//! ```
//!
//! Loading recomputes the layout key, the snapshot identity and the
//! provenance digest from the recorded fields; any mismatch is
//! `E-SNAPSHOT-INTEGRITY`. A matching identity identifies bytes; it proves
//! nothing about the parameters, the optimizer or the evaluators.

use crate::identity::IdentityScheme;
/// Schema line.
pub const SCHEMA: &str = "noble-ngraph-snapshot/0";

/// A loaded or publishable snapshot file: the immutable parameter snapshot
/// and its training provenance.
#[derive(Clone, Debug, PartialEq)]
pub struct SnapshotFile {
    /// The parameter snapshot (graph, layout, slots, bytes, identity).
    pub snapshot: crate::snapshot::ParamSnapshot,
    /// The training provenance (`provenance.result == snapshot.id()`).
    pub provenance: crate::train::Provenance,
}

fn integrity(why: impl Into<alloc::string::String>) -> crate::diag::Diagnostic {
    crate::diag::Diagnostic::new(
        crate::diag::Code::SnapshotIntegrity,
        crate::diag::Site::Graph,
        why,
    )
}

impl SnapshotFile {
    /// The provenance digest recorded in the file: the identity encoding of
    /// every record in file order except the schema and `provenance` lines
    /// (design D6). With `bytes(x) = u64le(len x) ‖ x` and
    /// `str(s) = bytes(utf8 s)`, it is `H("noble.ngraph.provenance.v0" ‖ 0x00 ‖
    /// bytes(snapshot) ‖ bytes(graph) ‖ bytes(layout) ‖ u64le(#params) ‖
    /// (u32le(slot) ‖ u32le(bits))* ‖ bytes(optimizer-state) ‖ bytes(parent) ‖
    /// str(subject kind) ‖ bytes(subject identity) ‖ u64le(#closure) ‖
    /// u64le(id)* ‖ str(sub-profile) ‖ str(semantic revision) ‖ bytes(facts) ‖
    /// bytes(loss) ‖ bytes(data) ‖ str(policy) ‖ u64le(#opt-ins) ‖
    /// (u32le(node) ‖ str(kind) ‖ str(rule) ‖ [u32le(width bits)])* ‖
    /// u64le(seed) ‖ str(prng) ‖ str(optimizer) ‖ u64le(lr) ‖ u64le(beta1) ‖
    /// u64le(beta2) ‖ u64le(eps) ‖ u64le(steps) ‖ str(tools))`, where the
    /// width is present only for `clamped-linear`.
    #[must_use]
    pub fn provenance_digest(&self, ids: &dyn IdentityScheme) -> crate::ir::Digest {
        let s = &self.snapshot;
        let p = &self.provenance;
        let mut c = crate::ir::Canon::new(ids, "noble.ngraph.provenance.v0");
        c.bytes(&s.id());
        c.bytes(&s.graph());
        c.bytes(&s.layout());
        c.u64(s.slots().len() as u64);
        for (pos, k) in s.slots().iter().enumerate() {
            c.u32(*k);
            c.u32(s.bits_at(pos).unwrap_or(0));
        }
        c.bytes(&p.optimizer_state);
        c.bytes(&p.parent);
        c.str(p.subject.kind.name());
        c.bytes(&p.subject.identity);
        c.u64(p.subject.closure.len() as u64);
        for d in &p.subject.closure {
            c.u64(*d);
        }
        c.str(p.profile.name());
        c.str(crate::ir::SEMANTIC_REVISION);
        c.bytes(&p.facts);
        c.bytes(&p.loss);
        c.bytes(&p.data);
        c.str(crate::grad::DERIV_POLICY);
        let opts = opt_ins(&p.deriv);
        c.u64(opts.len() as u64);
        for o in &opts {
            c.u32(o.node);
            c.str(o.kind);
            c.str(o.rule);
            if let Some(w) = o.width {
                c.u32(w);
            }
        }
        c.u64(p.seed);
        c.str(crate::prng::PRNG);
        c.str(crate::train::ADAM);
        c.u64(p.optimizer.lr.to_bits());
        c.u64(p.optimizer.beta1.to_bits());
        c.u64(p.optimizer.beta2.to_bits());
        c.u64(p.optimizer.eps.to_bits());
        c.u64(p.steps);
        c.str(&p.tools);
        c.finish()
    }

    /// Prints the file in canonical form.
    #[must_use]
    pub fn print(&self, ids: &dyn IdentityScheme) -> alloc::string::String {
        let s = &self.snapshot;
        let p = &self.provenance;
        let mut out = alloc::format!("{SCHEMA}\n");
        crate::format::write_line!(out, "snapshot {}", crate::ir::hex(&s.id()));
        crate::format::write_line!(out, "graph {}", crate::ir::hex(&s.graph()));
        crate::format::write_line!(out, "layout {}", crate::ir::hex(&s.layout()));
        for (pos, k) in s.slots().iter().enumerate() {
            let bits = s.bits_at(pos).unwrap_or(0);
            crate::format::write_line!(out, "param {k} 0x{bits:08x}");
        }
        crate::format::write_line!(
            out,
            "optimizer-state {}",
            crate::ir::hex(&p.optimizer_state)
        );
        crate::format::write_line!(out, "parent {}", crate::ir::hex(&p.parent));
        crate::format::write_line!(out, "subject {}", super::graph::subject_text(&p.subject));
        crate::format::write_line!(
            out,
            "profile {} {}",
            p.profile.name(),
            crate::ir::SEMANTIC_REVISION
        );
        crate::format::write_line!(out, "facts {}", crate::ir::hex(&p.facts));
        crate::format::write_line!(out, "loss {}", crate::ir::hex(&p.loss));
        crate::format::write_line!(out, "data {}", crate::ir::hex(&p.data));
        crate::format::write_line!(out, "policy {}", crate::grad::DERIV_POLICY);
        for o in opt_ins(&p.deriv) {
            match o.width {
                Some(w) => crate::format::write_line!(
                    out,
                    "opt-in %{} {} {} 0x{w:08x}",
                    o.node,
                    o.kind,
                    o.rule
                ),
                None => crate::format::write_line!(out, "opt-in %{} {} {}", o.node, o.kind, o.rule),
            };
        }
        crate::format::write_line!(out, "seed 0x{:016x}", p.seed);
        crate::format::write_line!(out, "prng {}", crate::prng::PRNG);
        let a = &p.optimizer;
        crate::format::write_line!(
            out,
            "optimizer {} lr 0x{:016x} beta1 0x{:016x} beta2 0x{:016x} eps 0x{:016x}",
            crate::train::ADAM,
            a.lr.to_bits(),
            a.beta1.to_bits(),
            a.beta2.to_bits(),
            a.eps.to_bits()
        );
        crate::format::write_line!(out, "steps {}", p.steps);
        crate::format::write_line!(out, "tools {}", p.tools);
        crate::format::write_line!(
            out,
            "provenance {}",
            crate::ir::hex(&self.provenance_digest(ids))
        );
        out
    }

    /// Parses a snapshot file and recomputes its identities.
    ///
    /// # Errors
    /// `E-FORMAT` (schema, field order, syntax), `E-BUDGET-SNAPSHOT-BYTES`
    /// over `snapshot_bytes`, or `E-SNAPSHOT-INTEGRITY` when the layout key,
    /// the snapshot identity or the provenance digest differs from its
    /// recomputation, a parameter is not finite, or the recorded `adam/0`
    /// hyperparameters are outside the optimizer's domain
    /// ([`crate::train::Adam::admit`]) — values `train` can never publish.
    #[expect(
        tigerstyle::unbounded_collection_growth,
        reason = "Owner: noble-maintainers; each `param` line is charged against `snapshot_bytes` before it is retained, so both vectors are bounded by the budget"
    )]
    pub fn parse(
        ids: &dyn IdentityScheme,
        text: &str,
        snapshot_bytes: u64,
    ) -> Result<Self, crate::diag::Diagnostic> {
        let mut c = Cursor {
            lines: super::lines(text).peekable(),
            last: 1,
        };
        let header = c.lines.next().ok_or_else(|| {
            super::value::format_diag(1, "missing noble-ngraph-snapshot/0 schema line")
        })?;
        if header.tokens != [SCHEMA] {
            return Err(super::value::format_diag(
                header.number,
                "missing noble-ngraph-snapshot/0 schema line",
            ));
        }
        c.last = header.number;
        let snapshot_id = c.digest("snapshot")?;
        let graph = c.digest("graph")?;
        let layout = c.digest("layout")?;
        let mut slots = alloc::vec::Vec::new();
        let mut bytes = alloc::vec::Vec::new();
        while let Some(l) = c.take_if("param") {
            let (k, b) = match l.tokens.as_slice() {
                [_, k, b] => (super::decimal_u32(k), bits32(b)),
                _ => (None, None),
            };
            let (Some(k), Some(b)) = (k, b) else {
                return Err(super::value::format_diag(
                    l.number,
                    "expected `param <slot> 0x<bits>`",
                ));
            };
            // Charge the line before it is retained: the format declares no
            // slot count, so `snapshot_bytes` is the only bound.
            let observed = (bytes.len() as u64).saturating_add(4);
            if observed > snapshot_bytes {
                return Err(crate::diag::Diagnostic::budget(
                    crate::diag::BudgetKind::SnapshotBytes,
                    snapshot_bytes,
                    observed,
                ));
            }
            if !f32::from_bits(b).is_finite() {
                return Err(integrity(alloc::format!(
                    "non-finite parameter 0x{b:08x} in slot {k}"
                )));
            }
            slots.push(k);
            bytes.extend_from_slice(&b.to_le_bytes());
        }
        debug_assert!((bytes.len() as u64) <= snapshot_bytes);
        let optimizer_state = c.digest("optimizer-state")?;
        let parent = c.digest("parent")?;
        let (t, n) = c.field("subject")?;
        let subject = subject(&t, n)?;
        let (t, n) = c.field("profile")?;
        if t != [
            crate::ir::SubProfile::F32.name(),
            crate::ir::SEMANTIC_REVISION,
        ] {
            return Err(super::value::format_diag(
                n,
                "expected `profile ngraph-f32/0 ngraph-sem/0`",
            ));
        }
        let facts = c.digest("facts")?;
        let loss = c.digest("loss")?;
        let data = c.digest("data")?;
        let (t, n) = c.field("policy")?;
        if t != [crate::grad::DERIV_POLICY] {
            return Err(super::value::format_diag(
                n,
                "expected `policy ngraph-deriv-real/0`",
            ));
        }
        let mut deriv = crate::grad::DerivOptions::default();
        let mut previous: Option<(u32, u8)> = None;
        while let Some(l) = c.take_if("opt-in") {
            let key = opt_in(&l.tokens[1..], l.number, &mut deriv)?;
            if previous.is_some_and(|p| p >= key) {
                return Err(super::value::format_diag(
                    l.number,
                    "opt-in lines must be strictly ordered by node and then kind",
                ));
            }
            previous = Some(key);
        }
        let (t, n) = c.field("seed")?;
        let seed = match t.as_slice() {
            [s] => hex_u64(s),
            _ => None,
        }
        .ok_or_else(|| super::value::format_diag(n, "expected `seed 0x<16 hex>`"))?;
        let (t, n) = c.field("prng")?;
        if t != [crate::prng::PRNG] {
            return Err(super::value::format_diag(n, "expected `prng splitmix64/0`"));
        }
        let (t, n) = c.field("optimizer")?;
        let optimizer = match t.as_slice() {
            [o, "lr", lr, "beta1", b1, "beta2", b2, "eps", eps] if *o == crate::train::ADAM => {
                match (hex_u64(lr), hex_u64(b1), hex_u64(b2), hex_u64(eps)) {
                    (Some(lr), Some(b1), Some(b2), Some(eps)) => Some(crate::train::Adam {
                        lr: f64::from_bits(lr),
                        beta1: f64::from_bits(b1),
                        beta2: f64::from_bits(b2),
                        eps: f64::from_bits(eps),
                    }),
                    _ => None,
                }
            }
            _ => None,
        }
        .ok_or_else(|| {
            super::value::format_diag(
                n,
                "expected `optimizer adam/0 lr 0x… beta1 0x… beta2 0x… eps 0x…`",
            )
        })?;
        // `train` never publishes hyperparameters outside the adam/0 domain.
        optimizer
            .admit()
            .map_err(|d| integrity(alloc::format!("recorded optimizer: {}", d.detail)))?;
        let (t, n) = c.field("steps")?;
        let steps = match t.as_slice() {
            [s] => super::decimal_u64(s),
            _ => None,
        }
        .ok_or_else(|| super::value::format_diag(n, "expected `steps <n>`"))?;
        let (t, n) = c.field("tools")?;
        if t.is_empty() {
            return Err(super::value::format_diag(n, "expected `tools <revisions>`"));
        }
        let tools = t.join(" ");
        let recorded = c.digest("provenance")?;
        if let Some(l) = c.lines.next() {
            return Err(super::value::format_diag(
                l.number,
                "unexpected line after the provenance digest",
            ));
        }
        if crate::snapshot::layout_key_of(ids, &slots) != layout {
            return Err(integrity("the layout key differs from its recomputation"));
        }
        let snapshot = crate::snapshot::ParamSnapshot::from_parts(ids, graph, layout, slots, bytes);
        if snapshot.id() != snapshot_id {
            return Err(integrity(
                "the snapshot identity differs from its recomputation",
            ));
        }
        let file = Self {
            snapshot,
            provenance: crate::train::Provenance {
                parent,
                subject,
                graph,
                profile: crate::ir::SubProfile::F32,
                facts,
                loss,
                data,
                deriv,
                seed,
                optimizer,
                steps,
                tools,
                result: snapshot_id,
                optimizer_state,
            },
        };
        if file.provenance_digest(ids) != recorded {
            return Err(integrity(
                "the provenance digest differs from its recomputation",
            ));
        }
        Ok(file)
    }

    /// Binds the snapshot to a checked graph before any node is evaluated
    /// (design D6).
    ///
    /// # Errors
    /// `E-SNAPSHOT-STALE` (another `GraphLogicalId`, subject, fact-binding
    /// digest or tool-revision string), `E-SNAPSHOT-LAYOUT` (another param
    /// layout) or `E-SNAPSHOT-ID` (not the caller-pinned snapshot).
    pub fn bind(
        &self,
        ids: &dyn IdentityScheme,
        checked: &crate::check::CheckedGraph,
        pinned: Option<&crate::ir::Digest>,
    ) -> Result<(), crate::diag::Diagnostic> {
        if self.snapshot.graph() != checked.id() {
            return Err(crate::diag::Diagnostic::new(
                crate::diag::Code::SnapshotStale,
                crate::diag::Site::Graph,
                "snapshot belongs to another GraphLogicalId",
            ));
        }
        if self.provenance.facts != checked.fact_bindings(ids) {
            return Err(crate::diag::Diagnostic::new(
                crate::diag::Code::SnapshotStale,
                crate::diag::Site::Graph,
                "snapshot was trained under other fact bindings",
            ));
        }
        if self.provenance.subject != checked.graph().subject {
            return Err(crate::diag::Diagnostic::new(
                crate::diag::Code::SnapshotStale,
                crate::diag::Site::Graph,
                "snapshot was trained for another subject",
            ));
        }
        if self.provenance.tools != crate::train::tools() {
            return Err(crate::diag::Diagnostic::new(
                crate::diag::Code::SnapshotStale,
                crate::diag::Site::Graph,
                "snapshot records other tool revisions",
            ));
        }
        self.snapshot.check_use(checked)?;
        if let Some(p) = pinned {
            if *p != self.snapshot.id() {
                return Err(crate::diag::Diagnostic::new(
                    crate::diag::Code::SnapshotId,
                    crate::diag::Site::Graph,
                    "snapshot differs from the pinned snapshot identity",
                ));
            }
        }
        Ok(())
    }
}

/// Sequential field reader over significant lines.
struct Cursor<'a, I: Iterator<Item = super::Line<'a>>> {
    lines: core::iter::Peekable<I>,
    last: usize,
}

impl<'a, I: Iterator<Item = super::Line<'a>>> Cursor<'a, I> {
    /// The next line, which must start with `key`: its remaining tokens and
    /// its line number.
    fn field(
        &mut self,
        key: &str,
    ) -> Result<(alloc::vec::Vec<&'a str>, usize), crate::diag::Diagnostic> {
        let l = self.lines.next().ok_or_else(|| {
            super::value::format_diag(self.last, alloc::format!("missing {key} line"))
        })?;
        if l.tokens[0] != key {
            return Err(super::value::format_diag(
                l.number,
                alloc::format!("expected {key}, got {:?}", l.tokens[0]),
            ));
        }
        self.last = l.number;
        Ok((l.tokens[1..].to_vec(), l.number))
    }

    /// `key <64 hex>`.
    fn digest(&mut self, key: &str) -> Result<crate::ir::Digest, crate::diag::Diagnostic> {
        let (t, n) = self.field(key)?;
        match t.as_slice() {
            [h] => super::digest_from_hex(h).ok_or_else(|| {
                super::value::format_diag(n, alloc::format!("{key} must be 64 hex digits"))
            }),
            _ => Err(super::value::format_diag(
                n,
                alloc::format!("{key} takes one digest"),
            )),
        }
    }

    /// The next line if it starts with `key`.
    fn take_if(&mut self, key: &str) -> Option<super::Line<'a>> {
        if self.lines.peek().is_some_and(|l| l.tokens[0] == key) {
            let l = self.lines.next()?;
            self.last = l.number;
            Some(l)
        } else {
            None
        }
    }
}

fn bits32(s: &str) -> Option<u32> {
    super::hex_bits(s, 8).and_then(|b| u32::try_from(b).ok())
}

fn hex_u64(s: &str) -> Option<u64> {
    super::hex_bits(s, 16)
}

fn subject(t: &[&str], n: usize) -> Result<crate::ir::Subject, crate::diag::Diagnostic> {
    let bad = || super::value::format_diag(n, "expected `subject <kind> <64 hex> closure <ids|->`");
    let [kind, id, c, ids] = t else {
        return Err(bad());
    };
    let kind = match *kind {
        "authored" => crate::ir::SubjectKind::Authored,
        "authored-nickel" => crate::ir::SubjectKind::AuthoredNickel,
        "extracted" => crate::ir::SubjectKind::Extracted,
        "captured" => crate::ir::SubjectKind::Captured,
        _ => return Err(bad()),
    };
    let identity = super::digest_from_hex(id).ok_or_else(bad)?;
    if *c != "closure" {
        return Err(bad());
    }
    let closure = if *ids == "-" {
        alloc::vec::Vec::new()
    } else {
        ids.split(',')
            .map(|d| super::decimal_u64(d).ok_or_else(bad))
            .collect::<Result<alloc::vec::Vec<u64>, crate::diag::Diagnostic>>()?
    };
    Ok(crate::ir::Subject {
        kind,
        identity,
        closure,
    })
}

/// Parses one `opt-in` line into `deriv`; returns its `(node, kind)` key.
fn opt_in(
    t: &[&str],
    n: usize,
    deriv: &mut crate::grad::DerivOptions,
) -> Result<(u32, u8), crate::diag::Diagnostic> {
    let bad = || super::value::format_diag(n, "bad opt-in line");
    let node = t
        .first()
        .and_then(|s| s.strip_prefix('%'))
        .and_then(super::decimal_u32)
        .map(crate::ir::NodeId)
        .ok_or_else(bad)?;
    match &t[1..] {
        ["boolean", rest @ ..] => {
            let p = match rest {
                ["refuse"] => crate::grad::BooleanPolicy::Refuse,
                ["straight-through"] => crate::grad::BooleanPolicy::StraightThrough,
                ["clamped-linear", w] => crate::grad::BooleanPolicy::ClampedLinear(f32::from_bits(
                    bits32(w).ok_or_else(bad)?,
                )),
                _ => return Err(bad()),
            };
            deriv.boolean.insert(node, p);
            Ok((node.0, 0))
        }
        ["index", rest @ ..] => {
            let p = match rest {
                ["refuse"] => crate::grad::IndexPolicy::Refuse,
                ["zero"] => crate::grad::IndexPolicy::Zero,
                ["backward-difference"] => crate::grad::IndexPolicy::BackwardDifference,
                _ => return Err(bad()),
            };
            deriv.index.insert(node, p);
            Ok((node.0, 1))
        }
        _ => Err(bad()),
    }
}

/// One derivative opt-in record.
struct OptIn {
    node: u32,
    kind: &'static str,
    rule: &'static str,
    width: Option<u32>,
}

/// The opt-in records of `deriv`, ordered by node and then kind
/// (`boolean` before `index`).
#[expect(
    tigerstyle::unbounded_collection_growth,
    reason = "Owner: noble-maintainers; the output is bounded by the selected boolean and index option counts"
)]
fn opt_ins(deriv: &crate::grad::DerivOptions) -> alloc::vec::Vec<OptIn> {
    let mut out: alloc::vec::Vec<(u8, OptIn)> = alloc::vec::Vec::new();
    for (n, p) in &deriv.boolean {
        let (rule, width) = match p {
            crate::grad::BooleanPolicy::Refuse => ("refuse", None),
            crate::grad::BooleanPolicy::StraightThrough => ("straight-through", None),
            crate::grad::BooleanPolicy::ClampedLinear(w) => ("clamped-linear", Some(w.to_bits())),
        };
        out.push((
            0,
            OptIn {
                node: n.0,
                kind: "boolean",
                rule,
                width,
            },
        ));
    }
    for (n, p) in &deriv.index {
        let rule = match p {
            crate::grad::IndexPolicy::Refuse => "refuse",
            crate::grad::IndexPolicy::Zero => "zero",
            crate::grad::IndexPolicy::BackwardDifference => "backward-difference",
        };
        out.push((
            1,
            OptIn {
                node: n.0,
                kind: "index",
                rule,
                width: None,
            },
        ));
    }
    out.sort_by_key(|(k, o)| (o.node, *k));
    out.into_iter().map(|(_, o)| o).collect()
}
