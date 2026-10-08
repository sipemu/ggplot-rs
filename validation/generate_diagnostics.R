#!/usr/bin/env Rscript
# Reference values for the 0.17 regression-diagnostic stats (QQ against
# several distributions + confidence bands, ECDF DKW band, Cook's-distance
# contours, Kaplan–Meier steps).
#
# Usage: Rscript validation/generate_diagnostics.R
# Requirements: ggplot2 (>= 4.0), qqplotr (0.0.7), survival.
# (qqplotr can live in a private library: set R_LIBS_USER.)

suppressMessages({
  library(ggplot2)
  library(qqplotr)
  library(survival)
})

out <- "validation/fixtures/diag"
dir.create(out, recursive = TRUE, showWarnings = FALSE)
w <- function(d, f) write.csv(format(d, digits = 15), file.path(out, f), row.names = FALSE, quote = FALSE)

# ---- input: deterministic, rounded so the CSV round-trip is exact ----------
set.seed(17)
qq_input <- data.frame(
  resid = round(rt(30, 4) * 1.5, 6),
  pos = round(rexp(30, 0.5), 6)
)
w(qq_input, "qq_input.csv")

# Half-normal for qqplotr / ggplot2 (they look the functions up by name).
qhalfnorm <- function(p, sd = 1) sd * qnorm((1 + p) / 2)
dhalfnorm <- function(x, sd = 1) ifelse(x < 0, 0, 2 / sd * dnorm(x / sd))
rhalfnorm <- function(n, sd = 1) abs(rnorm(n, sd = sd))  # unused (no "boot" band)
assign("qhalfnorm", qhalfnorm, envir = globalenv())
assign("rhalfnorm", rhalfnorm, envir = globalenv())
assign("dhalfnorm", dhalfnorm, envir = globalenv())

dists <- list(
  norm = list(col = "resid", q = qnorm, name = "norm", dp = list(mean = 0, sd = 1)),
  t5 = list(col = "resid", q = qt, name = "t", dp = list(df = 5)),
  exp = list(col = "pos", q = qexp, name = "exp", dp = list(rate = 0.5)),
  halfnorm = list(col = "pos", q = qhalfnorm, name = "halfnorm", dp = list(sd = 1))
)

pts <- NULL; lines <- NULL; bands <- NULL
for (key in names(dists)) {
  d <- dists[[key]]
  df <- data.frame(sample = qq_input[[d$col]])
  p <- ggplot(df, aes(sample = sample)) +
    ggplot2::stat_qq(distribution = d$q, dparams = d$dp)
  ld <- layer_data(p)
  pts <- rbind(pts, data.frame(dist = key, x = ld$x, y = ld$y))
  p <- ggplot(df, aes(sample = sample)) +
    ggplot2::stat_qq_line(distribution = d$q, dparams = d$dp)
  ld <- layer_data(p)
  lines <- rbind(lines, data.frame(dist = key, x = ld$x, y = ld$y))
  for (bt in c("pointwise", "ks")) {
    p <- ggplot(df, aes(sample = sample)) +
      qqplotr::stat_qq_band(distribution = d$name, dparams = d$dp,
                            bandType = bt, conf = 0.95)
    ld <- layer_data(p)
    bands <- rbind(bands, data.frame(dist = key, band = bt, x = ld$x,
                                     ymin = ld$ymin, ymax = ld$ymax))
  }
}
# stats::qqnorm agrees with stat_qq(norm) (theoretical = qnorm(ppoints(n))).
stopifnot(isTRUE(all.equal(sort(qqnorm(qq_input$resid, plot.it = FALSE)$x),
                           pts$x[pts$dist == "norm"])))
w(pts, "qq_points.csv")
w(lines, "qq_line.csv")
w(bands, "qq_band.csv")

# ---- ECDF + DKW band ---------------------------------------------------------
x <- qq_input$resid
ld <- layer_data(ggplot(data.frame(x = x), aes(x)) + stat_ecdf())
eps <- sqrt(log(2 / 0.05) / (2 * length(x)))
ecdf_band <- data.frame(x = ld$x, y = ld$y,
                        ymin = pmax(ld$y - eps, 0), ymax = pmin(ld$y + eps, 1))
w(ecdf_band, "ecdf_band.csv")

# ---- Cook's distance contours (plot.lm, which = 5) ---------------------------
# sqrt(level * p * (1 - h) / h) for p = 3 at a grid of leverages.
h <- c(0.02, 0.05, 0.1, 0.2, 0.35, 0.5, 0.8)
cooks <- rbind(
  data.frame(level = 0.5, h = h, y = sqrt(0.5 * 3 * (1 - h) / h)),
  data.frame(level = 1, h = h, y = sqrt(1 * 3 * (1 - h) / h))
)
w(cooks, "cooks_contour.csv")
# And on a real fit: the contour passes through every point whose Cook's D
# equals the level — check D = r^2 h / (p (1 - h)) inverts the formula.
fit <- lm(mpg ~ wt + hp, data = mtcars)
infl <- data.frame(h = hatvalues(fit), r = rstandard(fit), d = cooks.distance(fit))
w(infl, "cooks_mtcars.csv")

# ---- Kaplan–Meier (survival) for the step-ribbon / censor-mark example ------
km <- survfit(Surv(time, status) ~ 1, data = lung)
s <- summary(km, censored = TRUE)
w(data.frame(time = km$time, surv = km$surv, lower = km$lower,
             upper = km$upper, n_censor = km$n.censor), "km_lung.csv")

cat("diagnostic fixtures written to", out, "\n")
