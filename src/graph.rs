use std::collections::BTreeSet;
use std::fmt;

/// A named position in a machine graph.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Phase(String);

impl Phase {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for Phase {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<&str> for Phase {
    fn from(name: &str) -> Self {
        Self::new(name)
    }
}

impl From<String> for Phase {
    fn from(name: String) -> Self {
        Self::new(name)
    }
}

impl From<&Phase> for Phase {
    fn from(phase: &Phase) -> Self {
        phase.clone()
    }
}

impl fmt::Display for Phase {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A directed edge between two phases.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Transition {
    from: Phase,
    to: Phase,
    /// ETB-003: this transition closes intake integration and freezes the goal.
    freezes_goal: bool,
    /// PGE-006: the human-confirmed escape from a blocked ticket. `step`
    /// never takes it, so ordinary routing stays deterministic and never
    /// branches on any lifecycle field.
    blocked_route: bool,
}

impl Transition {
    pub fn new(from: impl Into<Phase>, to: impl Into<Phase>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            freezes_goal: false,
            blocked_route: false,
        }
    }

    /// Mark this transition as the intake-completion boundary (ETB-003).
    pub fn freezing_goal(mut self) -> Self {
        self.freezes_goal = true;
        self
    }

    pub fn freezes_goal(&self) -> bool {
        self.freezes_goal
    }

    /// Mark this transition as the human-confirmed blocked route (PGE-006).
    pub fn blocked_route(mut self) -> Self {
        self.blocked_route = true;
        self
    }

    pub fn is_blocked_route(&self) -> bool {
        self.blocked_route
    }

    pub fn from(&self) -> &Phase {
        &self.from
    }

    pub fn to(&self) -> &Phase {
        &self.to
    }
}

/// The machine's graph. Lifecycle information is deliberately not represented here.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MachineGraph {
    phases: BTreeSet<Phase>,
    transitions: Vec<Transition>,
}

impl MachineGraph {
    pub fn new<I, P, J>(phases: I, transitions: J) -> Self
    where
        I: IntoIterator<Item = P>,
        P: Into<Phase>,
        J: IntoIterator<Item = Transition>,
    {
        Self {
            phases: phases.into_iter().map(Into::into).collect(),
            transitions: transitions.into_iter().collect(),
        }
    }

    pub fn phases(&self) -> impl Iterator<Item = &Phase> {
        self.phases.iter()
    }

    pub fn transitions(&self) -> impl Iterator<Item = &Transition> {
        self.transitions.iter()
    }

    /// Finds the first ordinary transition leaving `phase`.
    ///
    /// Blocked routes are skipped: only a human-confirmed hold may take one.
    pub fn transition_for<P: AsRef<str>>(&self, phase: P) -> Option<&Transition> {
        let phase = phase.as_ref();
        self.transitions
            .iter()
            .find(|transition| transition.from.as_str() == phase && !transition.blocked_route)
    }

    /// PGE-006: the blocked route leaving `phase`, if the Runbook declares one.
    pub fn blocked_route_for<P: AsRef<str>>(&self, phase: P) -> Option<&Transition> {
        let phase = phase.as_ref();
        self.transitions
            .iter()
            .find(|transition| transition.from.as_str() == phase && transition.blocked_route)
    }

    /// Finds the first destination of a transition leaving `phase`.
    pub fn next_phase<P: AsRef<str>>(&self, phase: P) -> Option<&Phase> {
        self.transition_for(phase).map(Transition::to)
    }
}

/// Runtime machine position. It has no lifecycle/status dimension.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MachineState {
    phase: Phase,
}

impl MachineState {
    pub fn new(phase: impl Into<Phase>) -> Self {
        Self {
            phase: phase.into(),
        }
    }

    pub fn phase(&self) -> &Phase {
        &self.phase
    }
}
