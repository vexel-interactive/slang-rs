# Compatibility matrix

Statuses describe implemented behavior. **Partial** means working code with
explicit limits and regression tests. **Not ported** means the equivalent
subsystem is absent. No subsystem currently claims complete upstream parity.

| Slang subsystem | Rust crate | Status | Evidence or remaining work |
| --- | --- | --- | --- |
| Source manager and diagnostics | slang-source | Partial | UTF-8, CRLF, spans; upstream diagnostic IDs absent |
| Lexer | slang-lexer | Partial | Lossless tokens, decimal/hex floating syntax, typed rounding and range warnings, 32/64-bit and pointer-sized integer bases/suffixes, uint character values and escapes; complete string grammar absent |
| Preprocessor | slang-preprocessor | Partial | Macros, #, ##, variadics, includes, conditionals, __LINE__/__FILE__, #line remapping and resets; complete predefined macros and expansion stacks absent |
| Parser and AST | slang-syntax | Partial | Functions, globals, structs, user constructors and methods, declaration groups, expressions, casts, loops, switches, discard |
| Name and type checking | slang-sema | Partial | Scalars, vectors, matrices, fixed arrays with computed and zero dimensions, aliases, scopes, overloads, writable locations |
| Local storage and declaration scopes | slang-module, slang-sema, slang-ir | Partial | Lexical unbraced declarations, loop scopes, guarded local static initialization; definite-assignment analysis absent |
| Function default arguments | slang-sema, slang-ir | Partial | Trailing input defaults, first-declaration prototype rules, declaration-scope evaluation; parameter/receiver-dependent defaults absent |
| Struct methods | slang-syntax, slang-module, slang-sema, slang-ir | Partial | Nongeneric instance/static methods, immutable or mutating receivers, implicit member calls, overloads, prototypes, defaults, imports, aggregate results and resource receivers; generic methods, extensions, reference receiver modes and method values absent |
| Struct static fields | slang-syntax, slang-module, slang-sema, slang-ir | Partial | Shared numeric/aggregate/resource storage, per-specialization storage, constants in shapes and defaults, type/value access without base evaluation, imports and qualified reflection; general upstream global constant graph semantics absent |
| Nested types | slang-syntax, slang-module, slang-sema, slang-ir | Partial | Struct and alias members, nested type templates and generic owners, qualified type/value lookup, enclosing declaration scopes, constructors, methods, shapes, matrix aliases, module fragments and access control; local types and namespaces absent |
| Function inlining | slang-sema, slang-ir | Partial | ForceInline on numeric/aggregate/resource inputs, outputs and results, integer address views, void bodies, lexical scopes, control flow, early returns, static state, global initializer functions and expression-effect normalization; shared/volatile locals absent |
| Compile-time scalar constants | slang-sema, slang-ir | Partial | Typed 16/32/64-bit, pointer-sized integer and boolean expressions, narrow truncation, SSA local propagation and constant compound shift saturation, rounded literal-to-integer conversions, direct prefix signs and signed minima, forward globals, initializer cycle diagnostics, computed thread sizes; upstream high-bit unsigned comparison folding, general floating evaluation and constant function execution remain incomplete |
| Type aliases | slang-syntax, slang-sema | Partial | Transparent global and struct-member typedef/typealias, typealias templates, forwards, cycles, imports and qualified lookup; local aliases absent |
| Numeric type applications | slang-syntax, slang-module, slang-sema | Partial | Vector/matrix dimensions 1-4, scalar constant expressions, typed forward globals and lexical locals, core defaults, constructors, nested resource arguments; general constant execution and layout arguments absent |
| Explicit numeric casts | slang-syntax, slang-sema, slang-ir | Partial | Scalar splats, same-width vectors, matrix component conversions and truncation, boolean conversions, parameter copy casts for numeric/aggregate/resource types and symbolic shapes; standalone alias and aggregate cast syntax absent |
| Matrix component selectors | slang-sema, slang-codegen | Partial | Zero/one-based scalar and vector selections, checked bounds, distinct writable components, resource and atomic locations across dimensions 1-4 |
| Matrix storage order | slang-syntax, slang-sema, slang-ir, slang-layout | Partial | Row/column qualifiers, aliases, casts, resource elements, API/CLI defaults, legacy uniform reflection, logical matrix-array loads; layout template arguments and other target rules absent |
| Aggregate initialization | slang-syntax, slang-sema, slang-ir | Partial | Struct and fixed-array construction, constructors on specialized type owners, user __init overloads, declared defaults, implicit field scope, default construction, copies, constructor returns and outputs, contextual brace lists, fixed-array results, numeric padding/splats; generic constructor parameters and full generic inference absent |
| Core module | slang-sema | Partial | Small built-in signature set |
| Bit reinterpretation | slang-sema, slang-ir | Partial | 32-bit scalar/vector/matrix conversions, bool/half promotions, value-aware wide integer input conversions, scalar double splitting, scalar/double2 packing; 16/64-bit reinterpretation variants absent |
| Integer bit operations | slang-sema | Partial | 16/32/64-bit and pointer-sized scalar/vector count, reverse, and first-bit searches with signed inputs and bool promotion; 8-bit variants and other targets absent |
| Geometric functions | slang-sema, slang-ir | Partial | Scalar/vector dot, length, distance, normalize, reflect, refract, faceforward and cross; integer/double dot uses component arithmetic, other double HLSL geometry uses float precision; complete execution parity and AD absent |
| Screen derivatives | slang-sema, slang-ir | Partial | ddx/ddy/fwidth and coarse/fine variants, floating shapes and integer conversion, compute quads; full uniformity analysis absent |
| Integer atomics | slang-sema, slang-ir | Partial | Global 32-bit Interlocked operations on groupshared and RW locations, checked outputs; float and 64-bit atomics absent |
| Texture and sampler operations | slang-sema, slang-codegen | Partial | 1D/2D/3D/cube, indexed access, load/sample/bias/gradients/comparison/dimensions; advanced resources absent |
| Constant buffers | slang-syntax, slang-module, slang-sema, slang-ir, slang-codegen | Partial | Scalar/vector/matrix/array/struct/void elements, implicit value loads, handle overloads/returns/defaults, direct cbuffer, packoffset on structs and nested value arrays, module visibility, static members, read-only locations and legacy HLSL byte layout; packed handle arrays and mixed resource fields absent |
| Byte-address buffers | slang-sema, slang-codegen | Partial | Load/store widths, byte count, uint atomic operations with explicit outputs; typed access and advanced atomics absent |
| Modules and imports | slang-module, slang-sema | Partial | Imports, reexports, member files, isolated preprocessing, public/internal, legacy visibility; session API and cross-module overload merging absent |
| Type generics | slang-syntax, slang-module, slang-sema | Partial | Struct/typealias type parameters, defaults, opaque parameter body checking, explicit cached specialization, nested templates, methods/constructors/static storage on concrete owners, imports and canonical scalar/array/numeric shapes; general constrained checking, inference, value parameters and witnesses absent |
| Generic functions | slang-syntax, slang-module, slang-sema, slang-ir | Partial | Explicit/partial arguments, semantic value-argument inference before defaults, structural arrays/numeric shapes/resources/user templates/nested owners, canonical cached per-overload specialization, opaque body checking, prototype identity, first-declaration defaults, ordinary/generic conversion/default ranking, concrete ambiguity, output arguments and imports; generic methods, deferred candidate-body checking and complete constrained overload viability/ranking absent |
| Interfaces | ? | Not ported | Constraints, conformance checking, witnesses and interface dispatch required |
| Capability system | slang-sema, slang-compiler | Partial | Fragment operations, shared memory, barriers and derivative requirements through selected calls, globals and defaults; complete profile and target requirements absent |
| Shader profiles | slang-compiler | Partial | HLSL shader model 6 profile selection, derivative and native 16-bit upgrades, reflected target profiles and DXC type flags; complete capability and device validation absent |
| Automatic differentiation | ? | Not ported | AD passes, JVP/VJP, runtime contracts required |
| Structured IR | slang-ir | Partial | Typed tree lowering |
| SSA, linking, optimization | slang-ir | Partial | Scalar local SSA, CFG block arguments, use-def worklists, dominance and edge verifier, sparse conditional constant propagation; aggregate/memory SSA, IR linking, specialization, canonicalization and full pass pipeline absent |
| HLSL emitter | slang-codegen | Partial | Compute, vertex, fragment subset; examples validated by DXC |
| SPIR-V emitter | ? | Not ported | Native binary emission and spirv-val required |
| DXIL emitter | ? | Not ported | Textual HLSL output does not implement a native DXIL backend |
| GLSL, WGSL, Metal | ? | Not ported | Target-specific lowering and validators required |
| CUDA, C++, CPU targets | ? | Not ported | ABI lowering and runtime required |
| Resource binding and layout | slang-layout, slang-sema, slang-ir | Partial | Explicit HLSL classes/spaces, first-fit allocation, resource arrays, overlap warnings, shared global uniform buffer, legacy and native 16-bit HLSL storage, checked offsets/sizes/strides, struct packoffset and native matrix value reconstruction; packed handle arrays and other target rules absent |
| Declaration reflection | slang-compiler | Partial | Source/target names, types, stages, group sizes, module graph, resource ranges, binding origins, constant-buffer byte offsets and shared type layouts |
| Complete reflection API | ? | Not ported | Upstream API, layouts for all targets and buffer classes, nested resource aggregates, specialization absent |
| Native Rust API | slang-compiler | Partial | Own API for the implemented pipeline |
| Upstream C ABI and API | ? | Not ported | API/ABI compatibility suite required |
| slangc CLI | slang-cli | Partial | Limited option set |
| Language server | ? | Not ported | Incremental model and LSP transport required |
| Repro, serialization, caches | ? | Not ported | Stable formats and versioning required |
| Upstream test coverage | tools, tests/conformance | Partial | Source inventory and a small separate differential corpus |

## Full rewrite completion criteria

1. Port all compiler-facing declarations and subsystems in the pinned baseline.
2. Classify and run the full upstream corpus, with documented platform skips.
   A test is not passed merely because its source parses.
3. Validate every supported target and compare behavior: shader execution,
   layout, reflection, diagnostics, and specialization.
4. Pass API/ABI tests for applications that consume `slang.h`.
5. Build and test on target operating systems without an upstream compiler in
   the production pipeline.
6. Document verified behavior for all implemented features.

These criteria are not yet met. Crate counts, renamed files, and source
inventories are not evidence of language or API parity.
