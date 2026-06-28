use crate::perturbation_graph::PerturbationGraph;
use std::io::Write;
use std::path::Path;

/// Components of the system-level convergence score.
#[derive(Clone, Debug)]
pub struct ConvergenceSnapshot {
    pub tick: u64,
    /// Mean Beta posterior variance across all relations (Diaconis-Freedman: O(1/n) decay).
    pub mean_beta_variance: f64,
    /// Mean prediction MSE across VIB bands (highlevel predictor).
    pub prediction_mse: f64,
    /// Graph instability (population std-dev of relation strengths).
    pub graph_instability: f64,
    /// Aggregate VFE-like scalar: weighted sum of the three components.
    /// Lower = more converged.
    pub vfe: f64,
}

const W_BETA_VAR: f64 = 10.0;
const W_PRED_MSE: f64 = 1.0;
const W_INSTABILITY: f64 = 2.0;

pub fn vfe_aggregate(pg: &PerturbationGraph, prediction_mse: f64) -> ConvergenceSnapshot {
    vfe_aggregate_at(pg, prediction_mse, 0)
}

pub fn vfe_aggregate_at(
    pg: &PerturbationGraph,
    prediction_mse: f64,
    tick: u64,
) -> ConvergenceSnapshot {
    let mean_beta_variance = if pg.relations.is_empty() {
        1.0
    } else {
        let sum: f64 = pg.relations.iter().map(|r| r.beta_variance()).sum();
        sum / pg.relations.len() as f64
    };

    let graph_instability = pg.graph_instability();

    let vfe = W_BETA_VAR * mean_beta_variance
        + W_PRED_MSE * prediction_mse
        + W_INSTABILITY * graph_instability;

    ConvergenceSnapshot {
        tick,
        mean_beta_variance,
        prediction_mse,
        graph_instability,
        vfe,
    }
}

/// Appends a convergence snapshot as a CSV row. Creates header on first call.
pub fn append_csv(path: &Path, snap: &ConvergenceSnapshot) -> std::io::Result<()> {
    let exists = path.exists() && std::fs::metadata(path).map(|m| m.len() > 0).unwrap_or(false);
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    if !exists {
        writeln!(
            f,
            "tick,vfe,mean_beta_variance,prediction_mse,graph_instability"
        )?;
    }
    writeln!(
        f,
        "{},{:.6},{:.6},{:.6},{:.6}",
        snap.tick, snap.vfe, snap.mean_beta_variance, snap.prediction_mse, snap.graph_instability
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::perturbation_graph::PerturbationGraph;

    #[test]
    fn empty_graph_gives_max_uncertainty() {
        let pg = PerturbationGraph::default();
        let snap = vfe_aggregate(&pg, 0.5);
        assert!(
            snap.mean_beta_variance >= 0.99,
            "empty graph should signal max uncertainty: {}",
            snap.mean_beta_variance
        );
        assert!(snap.vfe > 0.0, "VFE must be positive");
    }

    #[test]
    fn vfe_decreases_with_tighter_posteriors() {
        let mut pg = PerturbationGraph::default();
        let sig_a = [0.1; crate::perturbation_graph::PG_NODE_SIG_DIM];
        let sig_b = [0.5; crate::perturbation_graph::PG_NODE_SIG_DIM];
        let n_a = pg.add_node(0, sig_a);
        let n_b = pg.add_node(0, sig_b);
        let ep_a = crate::perturbation_graph::Endpoint::Node(n_a);
        let ep_b = crate::perturbation_graph::Endpoint::Node(n_b);

        pg.observe_relation(
            1000,
            ep_a,
            ep_b,
            crate::perturbation_graph::EdgeKind::Cooccur,
        );

        let vfe_wide = vfe_aggregate(&pg, 0.3);

        for tick in 0..200u64 {
            pg.observe_relation(
                2000 + tick * 1000,
                ep_a,
                ep_b,
                crate::perturbation_graph::EdgeKind::Cooccur,
            );
        }

        let vfe_tight = vfe_aggregate(&pg, 0.3);

        assert!(
            vfe_tight.vfe < vfe_wide.vfe,
            "tighter posterior must lower VFE: wide={:.4} tight={:.4}",
            vfe_wide.vfe,
            vfe_tight.vfe
        );
    }

    #[test]
    fn csv_roundtrip() {
        let dir = std::env::temp_dir().join("bionic_convergence_test");
        let _ = std::fs::create_dir_all(&dir);
        let csv_path = dir.join("conv_test.csv");
        let _ = std::fs::remove_file(&csv_path);

        let snap1 = ConvergenceSnapshot {
            tick: 100,
            mean_beta_variance: 0.25,
            prediction_mse: 0.1,
            graph_instability: 0.05,
            vfe: 2.6,
        };
        let snap2 = ConvergenceSnapshot {
            tick: 200,
            mean_beta_variance: 0.10,
            prediction_mse: 0.05,
            graph_instability: 0.02,
            vfe: 1.09,
        };

        append_csv(&csv_path, &snap1).unwrap();
        append_csv(&csv_path, &snap2).unwrap();

        let text = std::fs::read_to_string(&csv_path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 3, "header + 2 data rows");
        assert!(lines[0].starts_with("tick,vfe,"));
        assert!(lines[1].starts_with("100,"));
        assert!(lines[2].starts_with("200,"));

        let _ = std::fs::remove_file(&csv_path);
    }
}
