Test-only font fixtures (not used by the shipped library — only by
`#[cfg(test)]` code, same `include_bytes!` pattern as `assets/fonts/*.ttf`).

- `HankenGrotesk-wght.ttf` — the variable-weight family anofox-visualization
  (the primary consumer of `GeomAttribution::fit`) sets as its UI/chart font,
  used by sipemu/ggplot-rs#267's tests to measure real glyph advances against
  `render::text::text_width`'s heuristic estimate, instead of trusting the
  estimate alone. Downloaded from the `google/fonts` repository
  (`ofl/hankengrotesk/HankenGrotesk[wght].ttf`), licensed under the SIL Open
  Font License 1.1 — see `OFL-HankenGrotesk.txt`.
