#!/usr/bin/env Rscript
# Generate validation fixtures for ggplot-rs vs R ggplot2 comparison.
#
# Usage: Rscript validation/generate_all.R
#
# Requirements: ggplot2, readr, dplyr
# Install: install.packages(c("ggplot2", "readr", "dplyr"))

library(ggplot2)
library(readr)
library(dplyr)

fixtures_dir <- "validation/fixtures"
data_dir <- file.path(fixtures_dir, "data")
dir.create(data_dir, recursive = TRUE, showWarnings = FALSE)

cat("Generating validation fixtures...\n")

# ===== Input datasets (deterministic, no RNG) =====

# uniform_100: 100 evenly-spaced values from 0 to 9.9
uniform_100 <- data.frame(x = seq(0, 9.9, by = 0.1))
write_csv(uniform_100, file.path(data_dir, "uniform_100.csv"))

# normal_200: 200 evenly-spaced values from -3 to 3
normal_200 <- data.frame(x = seq(-3, 3, length.out = 200))
write_csv(normal_200, file.path(data_dir, "normal_200.csv"))

# boxplot_groups: 3 groups x 20 values each
boxplot_groups <- rbind(
  data.frame(x = "A", y = seq(10.0, 19.5, by = 0.5)),
  data.frame(x = "B", y = seq(20.0, 29.5, by = 0.5)),
  data.frame(x = "C", y = seq(15.0, 24.5, by = 0.5))
)
write_csv(boxplot_groups, file.path(data_dir, "boxplot_groups.csv"))

# grouped_bars: categorical x with fill groups
grouped_bars <- rbind(
  data.frame(x = rep("a", 3), fill = "g1"),
  data.frame(x = rep("a", 2), fill = "g2"),
  data.frame(x = rep("b", 2), fill = "g1"),
  data.frame(x = rep("b", 3), fill = "g2"),
  data.frame(x = rep("c", 1), fill = "g1"),
  data.frame(x = rep("c", 4), fill = "g2")
)
write_csv(grouped_bars, file.path(data_dir, "grouped_bars.csv"))

# smooth_input: x = 1..50, y = 2x + 1 + 0.5*sin(0.3x)
smooth_input <- data.frame(
  x = 1:50,
  y = 2 * (1:50) + 1 + 0.5 * sin(0.3 * (1:50))
)
write_csv(smooth_input, file.path(data_dir, "smooth_input.csv"))

# summary_input: x = 1..10, 5 values per x
summary_input <- do.call(rbind, lapply(1:10, function(x) {
  data.frame(x = x, y = x * 2 + 0:4)
}))
write_csv(summary_input, file.path(data_dir, "summary_input.csv"))

# qq_input: y = 0..99
qq_input <- data.frame(y = 0:99)
write_csv(qq_input, file.path(data_dir, "qq_input.csv"))

# bin2d_input: 100 sin/cos points
bin2d_input <- data.frame(
  x = sin((0:99) * 0.1),
  y = cos((0:99) * 0.1)
)
write_csv(bin2d_input, file.path(data_dir, "bin2d_input.csv"))

# position_input: 6 rows for position adjustment tests
position_input <- data.frame(
  x = c(1, 1, 2, 2, 3, 3),
  y = c(3, 2, 5, 4, 1, 3),
  fill = c("g1", "g2", "g1", "g2", "g1", "g2")
)
write_csv(position_input, file.path(data_dir, "position_input.csv"))

# ===== Expected outputs via ggplot_build() =====

# --- stat_ecdf ---
p <- ggplot(uniform_100, aes(x = x)) + stat_ecdf()
d <- ggplot_build(p)$data[[1]]
write_csv(d %>% select(x, y), file.path(fixtures_dir, "stat_ecdf.csv"))
cat("  stat_ecdf: ", nrow(d), " rows\n")

# --- stat_count ---
p <- ggplot(grouped_bars, aes(x = x)) + geom_bar()
d <- ggplot_build(p)$data[[1]]
write_csv(d %>% select(x, y = count), file.path(fixtures_dir, "stat_count.csv"))
cat("  stat_count: ", nrow(d), " rows\n")

# --- stat_bin ---
p <- ggplot(uniform_100, aes(x = x)) + geom_histogram(bins = 30)
d <- ggplot_build(p)$data[[1]]
write_csv(d %>% select(x, y = count, density, xmin, xmax),
          file.path(fixtures_dir, "stat_bin.csv"))
cat("  stat_bin: ", nrow(d), " rows\n")

# --- stat_boxplot ---
p <- ggplot(boxplot_groups, aes(x = x, y = y)) + geom_boxplot()
d <- ggplot_build(p)$data[[1]]
write_csv(d %>% select(x = group, ymin, lower, middle, upper, ymax,
                        notchupper, notchlower),
          file.path(fixtures_dir, "stat_boxplot.csv"))
cat("  stat_boxplot: ", nrow(d), " rows\n")

# --- stat_density ---
p <- ggplot(uniform_100, aes(x = x)) + geom_density(n = 512)
d <- ggplot_build(p)$data[[1]]
write_csv(d %>% select(x, y), file.path(fixtures_dir, "stat_density.csv"))
cat("  stat_density: ", nrow(d), " rows\n")

# --- stat_smooth_lm ---
p <- ggplot(smooth_input, aes(x = x, y = y)) + geom_smooth(method = "lm", n = 80)
d <- ggplot_build(p)$data[[1]]
write_csv(d %>% select(x, y, ymin, ymax),
          file.path(fixtures_dir, "stat_smooth_lm.csv"))
cat("  stat_smooth_lm: ", nrow(d), " rows\n")

# --- stat_smooth_loess ---
p <- ggplot(smooth_input, aes(x = x, y = y)) +
  geom_smooth(method = "loess", span = 0.75, n = 80)
d <- ggplot_build(p)$data[[1]]
write_csv(d %>% select(x, y, ymin, ymax),
          file.path(fixtures_dir, "stat_smooth_loess.csv"))
cat("  stat_smooth_loess: ", nrow(d), " rows\n")

# --- stat_summary ---
p <- ggplot(summary_input, aes(x = x, y = y)) +
  stat_summary(fun = mean, fun.min = min, fun.max = max)
d <- ggplot_build(p)$data[[1]]
write_csv(d %>% select(x, y, ymin, ymax),
          file.path(fixtures_dir, "stat_summary.csv"))
cat("  stat_summary: ", nrow(d), " rows\n")

# --- stat_qq ---
p <- ggplot(qq_input, aes(sample = y)) + stat_qq()
d <- ggplot_build(p)$data[[1]]
write_csv(d %>% select(x = theoretical, y = sample),
          file.path(fixtures_dir, "stat_qq.csv"))
cat("  stat_qq: ", nrow(d), " rows\n")

# --- stat_qq_line ---
p <- ggplot(qq_input, aes(sample = y)) + stat_qq_line()
d <- ggplot_build(p)$data[[1]]
write_csv(d %>% select(x, y), file.path(fixtures_dir, "stat_qq_line.csv"))
cat("  stat_qq_line: ", nrow(d), " rows\n")

# --- stat_bin2d ---
p <- ggplot(bin2d_input, aes(x = x, y = y)) + geom_bin2d(bins = 5)
d <- ggplot_build(p)$data[[1]]
write_csv(d %>% select(xmin, xmax, ymin, ymax, fill = count),
          file.path(fixtures_dir, "stat_bin2d.csv"))
cat("  stat_bin2d: ", nrow(d), " rows\n")

# --- stat_binhex ---
p <- ggplot(bin2d_input, aes(x = x, y = y)) + geom_hex(bins = 5)
d <- ggplot_build(p)$data[[1]]
write_csv(d %>% select(x, y, fill = count),
          file.path(fixtures_dir, "stat_binhex.csv"))
cat("  stat_binhex: ", nrow(d), " rows\n")

# --- stat_ydensity ---
group_a <- boxplot_groups %>% filter(x == "A")
p <- ggplot(group_a, aes(x = x, y = y)) + geom_violin(n = 128)
d <- ggplot_build(p)$data[[1]]
# Extract density and width info
write_csv(d %>% select(y, density, width),
          file.path(fixtures_dir, "stat_ydensity.csv"))
cat("  stat_ydensity: ", nrow(d), " rows\n")

# --- position_stack ---
p <- ggplot(position_input, aes(x = x, y = y, fill = fill)) +
  geom_col(position = "stack")
d <- ggplot_build(p)$data[[1]]
write_csv(d %>% select(x, y = ymax, ymin),
          file.path(fixtures_dir, "position_stack.csv"))
cat("  position_stack: ", nrow(d), " rows\n")

# --- position_fill ---
p <- ggplot(position_input, aes(x = x, y = y, fill = fill)) +
  geom_col(position = "fill")
d <- ggplot_build(p)$data[[1]]
write_csv(d %>% select(x, y = ymax, ymin),
          file.path(fixtures_dir, "position_fill.csv"))
cat("  position_fill: ", nrow(d), " rows\n")

# --- position_dodge ---
p <- ggplot(position_input, aes(x = x, y = y, fill = fill)) +
  geom_col(position = "dodge")
d <- ggplot_build(p)$data[[1]]
write_csv(d %>% select(x, y, fill = group),
          file.path(fixtures_dir, "position_dodge.csv"))
cat("  position_dodge: ", nrow(d), " rows\n")

# --- scale_continuous_breaks ---
p <- ggplot(uniform_100, aes(x = x)) + geom_histogram()
built <- ggplot_build(p)
breaks <- built$layout$panel_scales_x[[1]]$break_positions()
write_csv(data.frame(breaks = breaks),
          file.path(fixtures_dir, "scale_continuous_breaks.csv"))
cat("  scale_continuous_breaks: ", length(breaks), " breaks\n")

cat("\nAll fixtures generated successfully!\n")
