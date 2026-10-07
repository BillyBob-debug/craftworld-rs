# D77: branch 0x4431 (DOUBLE11) header gate

Date 2026-10-07 (America/Chicago). Cloud implementer; **not verified against disc data**.

## Question
The 01.33 scan (D68, `docs/update-overlay.md`) leaves 116 resources `unsupported` with
"branch outside tested LBP1/2 scope", all branch ID 0x4431. What header layout do they
use, and can their header and dependency table be read without guessing payloads?

## Evidence (Toolkit @ d6a2ef80b5a09999f08f23a803b516096055e5c9, lib/cwlib/src/main/java/cwlib)

| Rule | Source | Label |
|---|---|---|
| 0x4431 is `Branch.DOUBLE11` (tag D1, LittleBigPlanet Vita); head `Revisions.D1_HEAD` = 0x3e2, max branch revision `Revisions.D1_MAX` = 0x87 | `enums/Branch.java`, `enums/Revisions.java` | CONFIRMED_FROM_COMMUNITY_CODE |
| `Revision.isVita()`: branch ID 0x4431 and `head & 0xffff` in 0x3c1..=0x3e2 ("can go as early as 0x3c1") | `types/data/Revision.java` | CONFIRMED_FROM_COMMUNITY_CODE |
| `SerializedResource.process`: at head >= 0x271 reads i16 branch ID + i16 branch revision, at head >= 0x297 one compression-flags byte, then the compressed bool, for every branch; no branch-specific header field (only LD at 0x272 differs) | `types/SerializedResource.java::process` | CONFIRMED_FROM_COMMUNITY_CODE |
| Vita payloads differ from PS3 LBP2 (e.g. `D1_PERFDATA`/`D1_SHADER`/`D1_UV_OFFSCALE` in RGfxMaterial, `D1_VERTEX_COLORS` in RMesh) | `enums/Revisions.java` D1_* | CONFIRMED_FROM_COMMUNITY_CODE |
| The 116 local resources' heads, branch revisions and types | not read in this task | UNKNOWN (PC verify) |

## Implementation (`crates/lbp-resource`)
- `metadata.rs`: branch 0x4431 with head in 0x3c1..=0x3e2 and branch revision <= 0x87
  is read with the ordinary >= 0x297 layout; header `scope` = `double11_branch_header`,
  dependency table parsed, status `metadata_parsed`. Head outside the range or branch
  revision above D1_MAX stays `unsupported` with a specific reason and the header kept.
  Other non-zero branches unchanged. Constants `DOUBLE11_BRANCH_ID`, `DOUBLE11_HEADS`,
  `DOUBLE11_MAX_BRANCH_REVISION` exported.
- `binary.rs::payload`: refuses DOUBLE11 with `Unsupported("DOUBLE11 branch payload")`
  before decompression. Material/plan/animation already required branch 0; lbp-mesh had
  no branch check and would otherwise have decoded a Vita MSHb with the PS3 layout.
- Toolkit's stricter `isVita` range is adopted instead of its permissive header read, so
  a 0x4431 resource at a non-Vita head (e.g. 0x3f8) is not accepted on layout alone.

## Tests (synthetic recipes only)
`tests/metadata.rs`: `d77_double11_header_gate_vita_heads_and_d1_max_only` (range ends,
D1_MAX boundary, layout equality with branch 0, dependencies, out-of-range reasons,
MIZUKI/LD-off-head/unknown branches, every truncation malformed) and
`d77_double11_payloads_are_never_decoded`.

## Expected effect / not tested
- If all 116 are Vita heads with branch revision <= 0x87: `unsupported` 117 -> 1 (font),
  `metadata_parsed` +116, dependency links +their tables. Otherwise the new reasons give
  the split. Viewable/Complete: +0 (payloads refused by design).
- Not tested: real 0x4431 bytes, ALSR tail, any Vita payload, oracle parity.
