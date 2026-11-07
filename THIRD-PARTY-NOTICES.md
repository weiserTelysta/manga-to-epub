# Third-party notices

Telysta uses open-source Rust libraries. Their exact versions are recorded in
`Cargo.lock`; license identifiers and source URLs can be inspected with
`cargo metadata --locked --format-version 1`.

- **Noto Sans SC**, © The Noto Project Authors, SIL Open Font License 1.1.
  Embedded for offline Chinese text rendering. See `assets/fonts/OFL.txt`
  (included as `FONT-LICENSE.txt` in the portable package).
  Source: https://github.com/google/fonts/tree/main/ofl/notosanssc
- **Noto Serif SC**, © The Noto Project Authors, SIL Open Font License 1.1.
  Prepared on the `ui-redesign` branch for the next UI iteration; not embedded
  in the first `v0.1.0` application. See `assets/fonts/Serif-OFL.txt`.
  Source: https://github.com/google/fonts/tree/main/ofl/notoserifsc
- **UnRAR**, Alexander Roshal / RARLAB. The `unrar` crate statically embeds the
  UnRAR extraction library. Its license is reproduced in
  `assets/licenses/UNRAR.txt` (included as `UNRAR-LICENSE.txt` in the portable
  package). It may not be used to recreate the RAR compression algorithm.
  Source: https://www.rarlab.com/rar_add.htm
- **Rosé Pine** palette, Rosé Pine contributors. Theme inspiration and color
  values: https://rosepinetheme.com/palette/

Telysta's own code and vector icon are available under the MIT license.
