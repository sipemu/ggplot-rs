# Validation: ggplot-rs vs R ggplot2

This directory contains data-level validation tests that compare ggplot-rs's computed
statistical data against R's `ggplot_build()` output.

## Why data-level, not visual?

Different rendering engines (plotters vs R's grid graphics) produce fundamentally
different SVG/pixel output. Comparing images would be fragile and uninformative.
Instead, we compare the **computed statistical data** before rendering — this is
where algorithmic correctness matters.

## Directory structure

```
validation/
  README.md              # This file
  generate_all.R         # R script to regenerate all fixtures
  fixtures/
    data/                # Shared input datasets (used by both R and Rust)
      uniform_100.csv    # 100 evenly-spaced values [0, 9.9]
      normal_200.csv     # 200 evenly-spaced values [-3, 3]
      boxplot_groups.csv # 3 groups x 20 values
      grouped_bars.csv   # Categorical data with fill groups
      smooth_input.csv   # 50-point x/y for regression testing
      summary_input.csv  # Grouped x/y for stat_summary
      qq_input.csv       # 100 integer values for QQ plots
      bin2d_input.csv    # 100 sin/cos points for 2D binning
      position_input.csv # 6 rows for position adjustment tests
    stat_ecdf.csv        # Expected ECDF output
    stat_count.csv       # Expected count output
    stat_bin.csv         # Expected histogram bins
    stat_boxplot.csv     # Expected boxplot 5-number summaries
    stat_density.csv     # Expected KDE density curve
    stat_smooth_lm.csv   # Expected linear regression fit
    stat_smooth_loess.csv # Expected LOESS fit
    stat_summary.csv     # Expected summary (mean/min/max)
    stat_qq.csv          # Expected QQ theoretical quantiles
    stat_qq_line.csv     # Expected QQ reference line
    stat_bin2d.csv       # Expected 2D rectangular bins
    stat_binhex.csv      # Expected hexagonal bins
    stat_ydensity.csv    # Expected violin density
    position_stack.csv   # Expected stacked positions
    position_fill.csv    # Expected fill-normalized positions
    position_dodge.csv   # Expected dodged positions
    scale_continuous_breaks.csv # Expected axis breaks
```

## Running validation tests

```bash
# Run only validation tests
cargo test --test validation

# Run all tests (includes existing smoke tests + validation)
cargo test
```

No R installation is needed — the CSV fixtures are committed to the repository.
The Rust tests just read them and compare.

## Regenerating fixtures

If you need to regenerate the fixture files (e.g., after changing test data):

```bash
# Requires: R with ggplot2, readr, dplyr
Rscript validation/generate_all.R
```

The R script generates both input datasets and expected output using `ggplot_build()`.

### R dependencies

```r
install.packages(c("ggplot2", "readr", "dplyr"))
```

## Tolerance levels

| Stat/Position | Tolerance | Rationale |
|---------------|-----------|-----------|
| stat_ecdf | 1e-10 | Exact: rank/n arithmetic |
| stat_count | exact | Integer counts |
| stat_bin | 1e-6 | Floating-point bin arithmetic |
| stat_boxplot | 1e-6 | Type-7 quantile interpolation |
| stat_density | 0.01 | KDE is sensitive to bandwidth differences |
| stat_smooth_lm | 1e-4 | OLS is exact; CI uses z vs t approximation |
| stat_smooth_loess | 0.05 | LOESS implementations vary |
| stat_summary | 1e-10 | Exact: mean/min/max arithmetic |
| stat_qq | 0.01 | qnorm approximation differs slightly |
| stat_qq_line | 0.05 | Compounds qnorm and quantile differences |
| stat_bin2d | exact | Integer counts in fixed grid |
| stat_binhex | exact | Integer counts in hex grid |
| stat_ydensity | 0.01 | Same sensitivity as stat_density |
| position_stack | 1e-10 | Exact cumulative addition |
| position_fill | 1e-10 | Exact normalized division |
| position_dodge | 1e-6 | Floating-point offset arithmetic |

## Coverage matrix

| Component | Validated | Input data | Notes |
|-----------|-----------|------------|-------|
| StatEcdf | yes | uniform_100 | Near-exact match |
| StatCount | yes | grouped_bars | Exact match |
| StatBin | yes | uniform_100 | Bin boundary computation may differ |
| StatBoxplot | yes | boxplot_groups | Type-7 quantiles |
| StatDensity | yes | uniform_100 | Silverman bandwidth + Gaussian kernel |
| StatSmooth (Lm) | yes | smooth_input | OLS regression |
| StatSmooth (Loess) | yes | smooth_input | Local weighted regression |
| StatSummary | yes | summary_input | Mean/min/max |
| StatQQ | yes | qq_input | ppoints() + qnorm() |
| StatQQLine | yes | qq_input | Reference line through Q1/Q3 |
| StatBin2d | yes | bin2d_input | 2D rectangular binning |
| StatBinHex | yes | bin2d_input | Hexagonal binning |
| StatYDensity | yes | boxplot_groups | Violin density estimation |
| PositionStack | yes | position_input | Cumulative stacking |
| PositionFill | yes | position_input | Normalized stacking |
| PositionDodge | yes | position_input | Side-by-side offset |
| ScaleContinuous | yes | uniform_100 | Break computation (pretty()) |

## Algorithm fixes applied

The following algorithms were corrected to match R's implementations:

1. **`iqr()` in stat/density.rs and stat/ydensity.rs**: Changed from integer
   indexing (`sorted[n/4]`) to R-compatible type-7 quantile interpolation.
   This affects Silverman bandwidth calculation for density and violin plots.

2. **`ppoints()` in stat/qq.rs**: Changed from `(i+0.5)/n` (Blom formula) to
   R's `ppoints()`: `(i+1-a)/(n+1-2a)` where a=3/8 for n>10. This affects
   theoretical quantile computation in QQ plots.

3. **`quantile()` in stat/qq.rs (QQ line)**: Changed from integer indexing
   (`values[n/4]`, `values[3*n/4]`) to type-7 quantile interpolation for
   computing sample Q1/Q3 in the QQ reference line.

## Known remaining differences from R

| Component | Difference | Impact |
|-----------|-----------|--------|
| stat_smooth_lm CI | Uses z=1.96 instead of t-distribution | Negligible for n>30; wider CI for small n |
| stat_smooth_loess SE | Approximate formula vs hat-matrix | SE band width may differ |
| stat_bin boundaries | Our `min/max` range vs R's `fullseq` padding | Slightly different bin edges at extremes |
