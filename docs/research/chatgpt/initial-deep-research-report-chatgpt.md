# Repository naming research for a GraphQL‑inspired SDL and JTD‑like IR validator in Rust

## Executive summary

You’re naming an open-source Rust SDK whose “center of gravity” is: a GraphQL-inspired schema language (SDL) that compiles down into a constrained, validation-friendly intermediate representation (IR) reminiscent of JSON Type Definition (JTD). JTD is itself standardized as entity["organization","IETF","internet standards body"] RFC 8927, with explicit goals around portable validation and being no more expressive than mainstream programming-language type systems. citeturn0search12turn21search10

Two naming pitfalls matter for your project description:

* “SDL” is strongly overloaded in the broader software ecosystem (for example, **Simple DirectMedia Layer** is commonly called “SDL”), so names that are just `sdl-*` can be confusing or collide with existing ecosystem expectations. citeturn4search0turn4search1  
* “GraphQL parser + SDL tooling” already has strong, recognizable Rust ecosystem anchors—e.g., the **Apollo** Rust toolchain and `apollo-parser` (GraphQL schema + query parsing), so names that imply “full GraphQL implementation” can create expectation mismatches if your project is “validation-only + JTD-like IR.” citeturn0search1turn0search17turn11search9

Availability highlights from checks performed for this report:

* The crate name **`maproom`** is already taken on crates.io (docs exist on docs.rs), so avoid it if you want a matching crate name. citeturn32search0  
* Some “spec-*” brand territory is actively used in adjacent spaces (e.g., “SpecLedger” as a product/project name), which increases collision and confusion risk for “spec/ledger” metaphors. citeturn17search0turn17search2

For a project like yours, the best names usually:
* avoid bare `sdl-*`
* include a GraphQL hint (`gql` or `graphql`) without claiming full GraphQL semantics
* optionally mention `jtd` or `ir` to make the “compile-to-IR then validate YAML/JSON” positioning obvious

## Method and timestamps

Checks were performed using publicly indexed results for entity["company","GitHub","code hosting platform"] repositories/organizations and entity["company","crates.io","rust package registry"] crates (with docs.rs as a strong existence signal for published crates). The timestamp for the primary check window was **2026-04-15 18:50:06 UTC**. citeturn0time0

A key limitation: negative results (“free”) are inherently less provable via public indexing than positive matches (“taken”). Where I could confirm “taken” with a primary artifact (repo page, org page, docs.rs crate docs), I cite it explicitly; “free” should be treated as “no collision found in public indexing at check time,” and you should still attempt creation/publish as the final authoritative check.

## Naming patterns shaped by your architecture

Your architecture has three “signals” you can encode in a name:

First is **language shape and familiarity**: you’re GraphQL-inspired, using SDL-like syntax, but you’re not necessarily implementing all GraphQL type-system semantics or execution behavior. The `apollo-parser` crate and its surrounding ecosystem (Apollo’s Rust GraphQL tooling) demonstrate how prominent and “spec-aligned” that space already is, so names that look like a full GraphQL engine can mislead. citeturn0search17turn11search9

Second is your **validation stance**: you validate YAML/JSON content against schemas. It’s helpful to borrow the mental model from tools like `kubeconform`, which is explicitly a manifest validation tool and reserves more complex semantics to the server-side; this maps well to your “schema adherence” goal. citeturn9search0turn0search2

Third is the **IR constraint and portability story**: “JTD-like IR” is compelling because RFC 8927’s design goals emphasize portability and staying within common type-system expressiveness—exactly the kind of constraint that makes Rust implementations clean and testable. citeturn0search12turn21search10

Practically: you get the clearest positioning when the name implies “SDL → constrained IR → validate documents” rather than “GraphQL runtime.”

## Candidate names with availability checks

Availability notes:
* **GitHub availability** below means “no public repo/org collision found” vs “collision found (taken)”.
* **crates.io availability** below means “no published crate discovered” vs “published crate exists”.
* If a name is **unavailable** on one axis but **available** on the other, the “Mitigation” note in that row suggests suffixes like `-rs`, `-rust`, or `-sdk`.

| Category | Name | Slug | GitHub availability | crates.io availability | Rationale | Suggested namespace |
|---|---|---|---|---|---|---|
| Technical | gqlsdl-jtd | gqlsdl-jtd | free (no collision found) | free (no collision found) | Directly signals GraphQL‑inspired SDL plus a JTD-style constraint story; short and explicit. | unspecified |
| Technical | gqlsdl-validate | gqlsdl-validate | free (no collision found) | free (no collision found) | Emphasizes “validate” as the product; `gqlsdl` avoids confusion with other “SDL” meanings. | unspecified |
| Technical | gqlsdl-validator | gqlsdl-validator | free (no collision found) | free (no collision found) | Clear and user-facing; reads well in docs and CLI contexts. | unspecified |
| Technical | gqlsdl-schema | gqlsdl-schema | free (no collision found) | free (no collision found) | Frames the library as the schema-definition layer rather than a runtime. | unspecified |
| Technical | gqlsdl-kit | gqlsdl-kit | free (no collision found) | free (no collision found) | “Kit” implies an SDK + tooling surface (parser, IR, validator, diagnostics). | unspecified |
| Technical | gqlsdl-rs | gqlsdl-rs | free (no collision found) | free (no collision found) | Concise Rust convention; `-rs` mitigates collisions and makes intent explicit. | unspecified |
| Technical | jtd-ir | jtd-ir | free (no collision found) | free (no collision found) | Focuses on the “JTD-like IR” concept; works if you expect multiple front-ends later (not only SDL). | unspecified |
| Technical | jtd-ir-rs | jtd-ir-rs | free (no collision found) | free (no collision found) | Adds Rust disambiguation; good if you anticipate non-Rust libraries later. | unspecified |
| Technical | jtd-ir-validate | jtd-ir-validate | free (no collision found) | free (no collision found) | Makes the IR’s primary purpose obvious: validation. | unspecified |
| Technical | jtd-schema-bridge | jtd-schema-bridge | free (no collision found) | free (no collision found) | Conveys “compile/translate” between schema dialects; good if you foresee exporters/importers. | unspecified |
| Technical | yamljson-gqlsdl | yamljson-gqlsdl | free (no collision found) | free (no collision found) | Very on-the-nose for YAML/JSON validation via the SDL; longer but unambiguous. | unspecified |
| Technical | yamljson-jtd | yamljson-jtd | free (no collision found) | free (no collision found) | Highlights the “JTD-like” validation model for YAML/JSON payloads without stressing GraphQL. | unspecified |
| Technical | workflow-sdl | workflow-sdl | free (no collision found) | free (no collision found) | If your primary example domain is “workflow specs,” this is memorable; risk: “SDL” overload in other ecosystems. Mitigation: consider `gqlsdl-workflow`. citeturn4search0turn4search1 | unspecified |
| Technical | workflow-sdl-validator | workflow-sdl-validator | free (no collision found) | free (no collision found) | Most explicit if you want “workflow” in the project identity; slightly long but clear. | unspecified |
| Abstract | manifest-lens | manifest-lens | free (no collision found) | free (no collision found) | “Lens” suggests inspection/validation; “manifest” ties to YAML/JSON documents. | unspecified |
| Abstract | schema-harbor | schema-harbor | free (no collision found) | free (no collision found) | “Harbor” evokes a safe place to dock definitions; suitable for a registry/SDK vibe. | unspecified |
| Abstract | type-vault | type-vault | free (no collision found) | free (no collision found) | “Vault” suggests durability and integrity—nice metaphor for safe schema validation. Mitigation: if name collides later, append `-rs`. | unspecified |
| Abstract | docketry | docketry | free (no collision found) | free (no collision found) | Evokes official records/specs without being overly literal; pronounceable and brandable. | unspecified |
| Abstract | folioforge | folioforge | free (no collision found) | free (no collision found) | “Folio” is document-centric; “forge” implies compilation/IR generation (but avoid “SpecForge” confusion). citeturn5search0turn30search0 | unspecified |
| Abstract | index-prism | index-prism | free (no collision found) | free (no collision found) | Suggests “structured view” of documents; good match for an AST→IR pipeline. | unspecified |
| Abstract | vaultscribe | vaultscribe | free (no collision found) | free (no collision found) | “Scribe” implies a language/DSL; “vault” implies correctness and governance. | unspecified |
| Abstract | blueprint-bay | blueprint-bay | free (no collision found) | free (no collision found) | Blueprint metaphor for schemas; “bay” hints at organization/warehouse without “registry” clichés. | unspecified |
| Abstract | catalog-cove | catalog-cove | free (no collision found) | free (no collision found) | Friendly brand; subtly implies cataloging and safe storage. | unspecified |
| Abstract | archive-lantern | archive-lantern | free (no collision found) | free (no collision found) | Lantern = illumination/visibility; archive = durable specs; memorable for docs and tooling. | unspecified |
| Abstract | registry-nest | registry-nest | free (no collision found) | free (no collision found) | A “nest” metaphor suggests a home for schemas; works well if you ever add a remote registry feature. | unspecified |
| Abstract | maproom | maproom | free (no collision found) | **taken —** `https://crates.io/crates/maproom` (docs exist) citeturn32search0 | Great metaphor for “navigation through structure,” but already used as a Rust crate name. **Mitigation:** `maproom-sdl`, `maproom-rs-sdk`, or `maproom-validate`. citeturn32search0 | unspecified |
| Abstract | deskfile | deskfile | free (no collision found) | free (no collision found) | Evokes desktop/folder workflows; good if your library targets config-heavy engineering teams. | unspecified |
| Abstract | slipcase | slipcase | free (no collision found) | free (no collision found) | A slipcase protects books—nice analogy for “wrap your documents in a validating schema.” | unspecified |

## Collision mitigation strategies

If you find a mismatch where a name is available on one surface but not the other, three strategies tend to be the least disruptive:

A `-rs` or `-rust` suffix is the most common compatibility move in the Rust ecosystem. It preserves naming while clarifying language and often avoids collisions with non-Rust projects. This is especially helpful for abstract names like `type-vault` or `slipcase`, which may already exist in other ecosystems.

A `-sdk` suffix is helpful when you plan multiple crates (e.g., `*-core`, `*-cli`, `*-macros`) and want the umbrella repo to read as an SDK. This is consistent with how other tooling ecosystems present themselves (toolkit + reference implementation); see how “parser crates” live inside larger tool repos like Apollo’s Rust GraphQL tooling. citeturn0search1turn0search17

A front-loaded disambiguator like `gqlsdl-` is particularly valuable given the name collision space around “SDL” (which is widely recognized as Simple DirectMedia Layer). Using `gqlsdl-*` up front preserves the “schema language” identity but avoids accidental association with unrelated SDL tooling. citeturn4search0turn4search1

## Final recommendations

The three names below are ranked to cover your “technical clarity,” “brandability,” and “fallback collision-proofing” goals.

Technical pick: **gqlsdl-jtd**  
It is concise, communicates the *two* most differentiating aspects (GraphQL-inspired SDL + JTD-like constraint model), and avoids the ambiguous standalone `sdl-*` namespace.

Abstract pick: **docketry**  
It’s short, memorable, and evokes “official records/specs” without pulling you into overloaded terms like “schema” or “validator.” It’s also flexible if the project grows beyond YAML/JSON or beyond a single SDL.

Fallback pick with suffix: **gqlsdl-jtd-rs**  
If either GitHub or crates.io conflicts appear later, this suffix preserves meaning while making the Rust identity explicit. It also scales well if you later publish subcrates like `gqlsdl-jtd-core`, `gqlsdl-jtd-cli`, etc.

## Reference links for context

```text
JTD RFC 8927 (JSON Type Definition):
https://www.rfc-editor.org/rfc/rfc8927.html

Apollo Rust GraphQL tooling (apollo-rs) and apollo-parser:
https://github.com/apollographql/apollo-rs
https://crates.io/crates/apollo-parser

kubeconform:
https://github.com/yannh/kubeconform
```