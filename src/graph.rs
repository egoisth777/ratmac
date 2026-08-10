use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// A named position in a machine graph.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct State(String);

impl State {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for State {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<&str> for State {
    fn from(name: &str) -> Self {
        Self::new(name)
    }
}

impl From<String> for State {
    fn from(name: String) -> Self {
        Self::new(name)
    }
}

impl From<&State> for State {
    fn from(state: &State) -> Self {
        state.clone()
    }
}

impl fmt::Display for State {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A directed edge between two states.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Transition {
    from: State,
    to: State,
    /// FDC-001: the exact closed-list value selecting this ordinary edge.
    /// Straight-line and blocked routes carry no input.
    input: Option<String>,
    /// ETB-003: this transition closes intake integration and freezes the goal.
    freezes_goal: bool,
    /// PGE-006: the human-confirmed escape from a blocked ticket. `step`
    /// never takes it, so ordinary routing stays deterministic and never
    /// branches on any lifecycle field.
    blocked_route: bool,
}

impl Transition {
    pub fn new(from: impl Into<State>, to: impl Into<State>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            input: None,
            freezes_goal: false,
            blocked_route: false,
        }
    }

    /// Label this ordinary edge with its exact transition input (FDC-001).
    pub fn with_input(mut self, input: impl Into<String>) -> Self {
        self.input = Some(input.into());
        self
    }

    pub fn input(&self) -> Option<&str> {
        self.input.as_deref()
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

    pub fn from(&self) -> &State {
        &self.from
    }

    pub fn to(&self) -> &State {
        &self.to
    }
}

/// The machine's graph. Lifecycle information is deliberately not represented here.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MachineGraph {
    states: BTreeSet<State>,
    transitions: Vec<Transition>,
}

impl MachineGraph {
    pub fn new<I, P, J>(states: I, transitions: J) -> Self
    where
        I: IntoIterator<Item = P>,
        P: Into<State>,
        J: IntoIterator<Item = Transition>,
    {
        Self {
            states: states.into_iter().map(Into::into).collect(),
            transitions: transitions.into_iter().collect(),
        }
    }

    pub fn states(&self) -> impl Iterator<Item = &State> {
        self.states.iter()
    }

    pub fn transitions(&self) -> impl Iterator<Item = &Transition> {
        self.transitions.iter()
    }

    /// Finds the sole unlabelled ordinary transition leaving `state`.
    ///
    /// Branching edges are selected only by [`Self::transition_for_input`].
    /// Blocked routes are skipped: only a human-confirmed hold may take one.
    pub fn transition_for<P: AsRef<str>>(&self, state: P) -> Option<&Transition> {
        self.transition_for_input(state, None)
    }

    /// FDC-001: select the ordinary edge carrying exactly `input`.
    ///
    /// Declaration order has no routing meaning. `None` selects only an
    /// unlabelled straight-line edge; blocked routes never participate.
    pub fn transition_for_input<P: AsRef<str>>(
        &self,
        state: P,
        input: Option<&str>,
    ) -> Option<&Transition> {
        let state = state.as_ref();
        let mut matches = self.transitions.iter().filter(|transition| {
            transition.from.as_str() == state
                && !transition.blocked_route
                && transition.input() == input
        });
        let selected = matches.next()?;
        matches.next().is_none().then_some(selected)
    }

    /// FDC-008: every elementary cycle over ordinary edges.
    ///
    /// Each cycle is reported exactly once, rooted at its lexicographically
    /// smallest State, in deterministic order. Blocked routes never form
    /// cycles: `rtm step` cannot take one, so they carry no repetition.
    pub fn ordinary_cycles(&self) -> Vec<Vec<State>> {
        let mut adjacency: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for transition in &self.transitions {
            if transition.blocked_route {
                continue;
            }
            adjacency
                .entry(transition.from.as_str())
                .or_default()
                .insert(transition.to.as_str());
        }

        fn explore<'a>(
            root: &'a str,
            at: &'a str,
            adjacency: &BTreeMap<&'a str, BTreeSet<&'a str>>,
            path: &mut Vec<&'a str>,
            cycles: &mut Vec<Vec<State>>,
        ) {
            let Some(nexts) = adjacency.get(at) else {
                return;
            };
            for &next in nexts {
                if next == root {
                    cycles.push(path.iter().map(|name| State::new(*name)).collect());
                } else if next > root && !path.contains(&next) {
                    // Restricting the walk to States after the root reports
                    // each cycle once, at its smallest member.
                    path.push(next);
                    explore(root, next, adjacency, path, cycles);
                    path.pop();
                }
            }
        }

        let mut cycles = Vec::new();
        for state in &self.states {
            let root = state.as_str();
            let mut path = vec![root];
            explore(root, root, &adjacency, &mut path, &mut cycles);
        }
        cycles
    }

    /// PGE-006: the blocked route leaving `state`, if the Runbook declares one.
    pub fn blocked_route_for<P: AsRef<str>>(&self, state: P) -> Option<&Transition> {
        let state = state.as_ref();
        self.transitions
            .iter()
            .find(|transition| transition.from.as_str() == state && transition.blocked_route)
    }

    /// FDC-002: true when `state` has at least one ordinary
    /// (non-blocked-route) outgoing transition. A State without one is
    /// structurally terminal: entering it completes ordinary execution.
    pub fn has_ordinary_outgoing<P: AsRef<str>>(&self, state: P) -> bool {
        let state = state.as_ref();
        self.transitions
            .iter()
            .any(|transition| transition.from.as_str() == state && !transition.blocked_route)
    }

    /// Finds the first destination of a transition leaving `state`.
    pub fn next_state<P: AsRef<str>>(&self, state: P) -> Option<&State> {
        self.transition_for(state).map(Transition::to)
    }
}

/// Runtime machine position. It has no lifecycle/status dimension.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MachineState {
    state: State,
}

impl MachineState {
    pub fn new(state: impl Into<State>) -> Self {
        Self {
            state: state.into(),
        }
    }

    pub fn state(&self) -> &State {
        &self.state
    }
}
