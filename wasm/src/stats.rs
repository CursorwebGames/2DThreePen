/// Declares Stats fields once; generates Stats + Aggregator from the same list.
/// Each field becomes a Vec<T> in Aggregator, serialized as { key: [T] } JSON.
#[cfg(not(target_arch = "wasm32"))]
macro_rules! define_stats {
    ( $( $(#[$doc:meta])* $field:ident : $ty:ty ),* $(,)? ) => {
        #[cfg(not(target_arch = "wasm32"))]
        #[derive(Default, Clone, Copy, Debug)]
        pub struct Stats {
            $( $(#[$doc])* pub $field: $ty, )*
        }

        #[cfg(not(target_arch = "wasm32"))]
        #[derive(Default)]
        pub struct Aggregator {
            $( pub $field: Vec<$ty>, )*
        }

        impl Aggregator {
            pub fn push(&mut self, s: Stats) {
                $( self.$field.push(s.$field); )*
            }

            pub fn to_json(&self) -> String {
                let pairs: &[(&str, String)] = &[$(
                    (stringify!($field),
                     format!("[{}]", self.$field.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")))
                ),*];
                let body = pairs.iter()
                    .map(|(k, v)| format!("  \"{k}\": {v}"))
                    .collect::<Vec<_>>()
                    .join(",\n");
                format!("{{\n{body}\n}}")
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
define_stats! {
    /// total time for this run in ms (set by bench.rs after timing)
    ms: f64,
    /// boards attempted
    rolls: usize,
    /// boards rejected by the cheap matching prefilter
    unsolvable: usize,
    /// total num of solver invocations
    solves: usize,
    /// solves that hit SOLVE_BUDGET and were abandoned
    over_budget: usize,
    /// total repairs across all rolls
    repairs: usize,
    /// num of rolls where capped regions left empty cells requiring a secondary fill pass
    secondary_fills: usize,
    /// repairs where kill_sol(0,1) failed and kill_sol(1,0) was tried
    kill_fallback: usize,
    /// num of repairs on successful board
    success_repairs: usize,
    /// total recursion steps across all solver invocations
    solver_steps: usize,
    /// cumulative time growing regions (ms)
    t_regions: f64,
    /// cumulative time in the matching prefilter (ms)
    t_solvable: f64,
    /// cumulative time in the solver (ms)
    t_solve: f64,
    /// cumulative time in repair / kill_sol (ms)
    t_kill: f64,
}

#[cfg(not(target_arch = "wasm32"))]
#[macro_export]
macro_rules! timed {
    ($self:ident.$field:ident, $body:expr) => {{
        let t = std::time::Instant::now();
        let out = $body;
        $self.stats.$field += t.elapsed().as_secs_f64() * 1000.0;
        out
    }};
}

#[cfg(target_arch = "wasm32")]
#[macro_export]
macro_rules! timed {
    ($self:ident.$field:ident, $body:expr) => {
        $body
    };
}

#[cfg(not(target_arch = "wasm32"))]
#[macro_export]
macro_rules! bump {
    ($self:ident.$field:ident) => {
        $self.stats.$field += 1;
    };
    ($self:ident.$field:ident, $amt:expr) => {
        $self.stats.$field += $amt;
    };
    ($field:ident) => {
        $field += 1;
    };
}

#[cfg(target_arch = "wasm32")]
#[macro_export]
macro_rules! bump {
    ($self:ident.$field:ident) => {};
    ($self:ident.$field:ident, $amt:expr) => {};
    ($field:ident) => {};
}
