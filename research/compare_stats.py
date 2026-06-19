"""Statistical comparison of GenPen vs genpenai generation times.

Reads the CSV written by `wasm/src/bin/tails.rs` (columns: method,n,seed,ms)
and, for each board size, runs:

  1. Welch's two-sample t-test (unequal variances), one-sided H1: genpen > genpenai.
     UNPAIRED on purpose: the two generators use different RNGs, so seed `i`
     produces unrelated boards in each — they are independent samples, not pairs.

  2. Mann-Whitney U (rank-sum). Timings are heavy right-skewed (reroll tail),
     which violates the t-test's normality assumption; this nonparametric test
     does not care about the shape and compares the distributions directly.

  3. TOST equivalence test. A non-significant t-test does NOT prove equality —
     absence of evidence isn't evidence of absence. TOST flips the burden:
     H1 is "the means differ by less than MARGIN", so a significant result
     actually lets us *conclude* practical equivalence.

p-values use the normal approximation (our per-group n is in the hundreds, so
the t- and U-statistics are effectively z); stdlib only, no scipy required.

    python compare_stats.py ../wasm/times.csv
"""

import csv
import math
import statistics
import sys
from collections import defaultdict

ALPHA = 0.05
MARGIN_FRAC = 0.10  # TOST: call them equivalent if means within 10% of pooled mean


def phi(z: float) -> float:
    """Standard normal CDF."""
    return 0.5 * (1.0 + math.erf(z / math.sqrt(2.0)))


def welch_t(a: list[float], b: list[float]) -> tuple[float, float]:
    """Welch's t-statistic and standard error of the mean difference."""
    n1, n2 = len(a), len(b)
    v1, v2 = statistics.variance(a), statistics.variance(b)
    se = math.sqrt(v1 / n1 + v2 / n2)
    t = (statistics.mean(a) - statistics.mean(b)) / se
    return t, se


def mann_whitney_z(a: list[float], b: list[float]) -> float:
    """Mann-Whitney U as a z-score, with tie correction."""
    n1, n2 = len(a), len(b)
    combined = sorted([(v, 0) for v in a] + [(v, 1) for v in b])

    # average ranks, tracking tie-group sizes for the variance correction
    ranks = [0.0] * len(combined)
    tie_term = 0.0
    i = 0
    while i < len(combined):
        j = i
        while j < len(combined) and combined[j][0] == combined[i][0]:
            j += 1
        avg_rank = (i + 1 + j) / 2.0  # ranks i+1..j averaged
        for k in range(i, j):
            ranks[k] = avg_rank
        t = j - i
        tie_term += t**3 - t
        i = j

    r1 = sum(rank for rank, (_, grp) in zip(ranks, combined) if grp == 0)
    u1 = r1 - n1 * (n1 + 1) / 2.0
    mu = n1 * n2 / 2.0
    n = n1 + n2
    sigma = math.sqrt((n1 * n2 / 12.0) * ((n + 1) - tie_term / (n * (n - 1))))
    return (u1 - mu) / sigma


def tost(a: list[float], b: list[float], margin: float) -> float:
    """TOST equivalence p-value (max of the two one-sided tests)."""
    t, se = welch_t(a, b)
    diff = statistics.mean(a) - statistics.mean(b)
    # H1 left: diff > -margin ; H1 right: diff < +margin
    p_lower = 1.0 - phi((diff + margin) / se)
    p_upper = phi((diff - margin) / se)
    return max(p_lower, p_upper)


def report(n: int, gp: list[float], ai: list[float], log_mode: bool) -> None:
    diff = statistics.mean(gp) - statistics.mean(ai)
    if log_mode:
        # margin as a +/-10% ratio: a log-difference of log(1.1)
        margin = math.log(1 + MARGIN_FRAC)
        unit = "log-ms"
    else:
        margin = MARGIN_FRAC * statistics.mean(gp + ai)
        unit = "ms"

    print(f"\n=== n={n}  (genpen={len(gp)}, genpenai={len(ai)} samples, {unit}) ===")
    print(f"  genpen   : mean={statistics.mean(gp):8.3f}  median={statistics.median(gp):8.3f}  sd={statistics.stdev(gp):8.3f}")
    print(f"  genpenai : mean={statistics.mean(ai):8.3f}  median={statistics.median(ai):8.3f}  sd={statistics.stdev(ai):8.3f}")
    if log_mode:
        # geometric means in ms are the interpretable summary on a log scale
        print(f"  geo mean : genpen={math.exp(statistics.mean(gp)):.3f} ms  "
              f"genpenai={math.exp(statistics.mean(ai)):.3f} ms  "
              f"ratio={math.exp(diff):.4f}")
    print(f"  mean diff (genpen - genpenai) = {diff:+.3f} {unit}")

    # 1. Welch one-sided H1: genpen > genpenai
    t, _ = welch_t(gp, ai)
    p_t = 1.0 - phi(t)
    verdict = "REJECT H0 (genpen slower)" if p_t < ALPHA else "fail to reject H0"
    print(f"  Welch t  : t={t:+.3f}  one-sided p(genpen>genpenai)={p_t:.4f}  -> {verdict}")

    # 2. Mann-Whitney (two-sided)
    z = mann_whitney_z(gp, ai)
    p_u = 2.0 * (1.0 - phi(abs(z)))
    print(f"  M-W U    : z={z:+.3f}  two-sided p={p_u:.4f}  -> {'distributions differ' if p_u < ALPHA else 'no difference detected'}")

    # 3. TOST equivalence within MARGIN
    p_eq = tost(gp, ai, margin)
    eq = "EQUIVALENT" if p_eq < ALPHA else "not shown equivalent"
    print(f"  TOST     : margin=+/-{margin:.3f} {unit}  p={p_eq:.4f}  -> {eq}")


def main() -> None:
    args = sys.argv[1:]
    log_mode = "--log" in args
    args = [a for a in args if a != "--log"]
    path = args[0] if args else "times.csv"

    data: dict[tuple[str, int], list[float]] = defaultdict(list)
    with open(path, newline="") as f:
        for row in csv.DictReader(f):
            data[(row["method"], int(row["n"]))].append(float(row["ms"]))

    if log_mode:
        # Timings are right-skewed (log-normal-ish); the log transform tames
        # the tail's leverage on variance, so equivalence has real power.
        for k in data:
            data[k] = [math.log(v) for v in data[k]]
        print("[--log] testing on log(ms); equivalence margin is a +/-10% ratio")

    sizes = sorted({n for _, n in data})
    for n in sizes:
        gp = data.get(("genpen", n), [])
        ai = data.get(("genpenai", n), [])
        if gp and ai:
            report(n, gp, ai, log_mode)


if __name__ == "__main__":
    main()
