# Hash Interleave Analysis Tool: Product Specification

## 1. Product Goals

The Hash Interleave analysis tool answers two questions:

1. Does an address mapping satisfy the expected mathematical properties?
2. Is the mapping sufficiently balanced under typical access patterns?

The tool is intended for design, verification, and performance-analysis engineers. Users describe address mappings and access scenarios in human-readable configuration files, and the tool produces conclusions that can be understood and independently verified.

This specification defines:

- the mathematical meaning and correctness criteria of an address mapping;
- the file formats supplied by users;
- the command-line interface;
- performance metrics and their formulas;
- output contracts for human-readable and structured reports;
- deterministic behavior for corner cases.

This specification does not prescribe a programming language, dependency library, internal data structure, or project organization.

## 2. Mathematical Model and Correctness Criteria

This chapter is the sole normative definition of Mapping correctness. Later chapters on input, output, and acceptance criteria refer back to these definitions rather than establishing a separate set of validation rules.

Formulas use GitHub-compatible LaTeX syntax: inline formulas use `$...$`, and display formulas use `$$...$$`. Even when a reader does not render LaTeX, the adjacent symbol table and prose make each formula understandable. See the [official GitHub documentation](https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/writing-mathematical-expressions) for syntax details.

### 2.1 Symbols

| Symbol | Meaning |
| --- | --- |
| $A$ | Total width of the byte address, in bits |
| $G$ | Access granule, in bytes |
| $g$ | Number of byte-offset bits within a granule |
| $n$ | Number of line-address bits that participate in the Mapping |
| $N$ | Number of Targets |
| $r$ | Number of bits in a Target ID |
| $s$ | Number of LA line bits within each Target |
| $a$ | Input byte address |
| $o$ | Byte offset within a granule |
| $x$ | Bit vector of the line address |
| $M$ | Matrix that generates the Target ID from the line address |
| $L$ | Matrix that generates the LA line from the line address |

The current scope requires both $G$ and $N$ to be powers of two. Therefore:

$$
g = \log_2 G,\qquad
n = A-g,\qquad
r = \log_2 N,\qquad
s = n-r
$$

The following constraints must hold:

$$
1 \le A \le 64,\qquad
1 \le G \le 2^A,\qquad
1 \le N \le 2^n
$$

### 2.2 Address Decomposition

For a valid address $0 \le a < 2^A$:

$$
o = a \bmod G,\qquad
q = \left\lfloor\frac{a}{G}\right\rfloor
$$

Write $q$ as an LSB-first bit vector:

$$
x =
\begin{bmatrix}
x_0 & x_1 & \cdots & x_{n-1}
\end{bmatrix}^{\mathsf T}
$$

Here, $x_0$ is the least-significant bit of the line address.

The Mapping operates only on $x$. The byte offset $o$ does not participate in the Target or LA-line calculation and is preserved unchanged in the final LA byte address.

### 2.3 Mapping

All matrix operations are over GF(2), where addition is equivalent to XOR.

$$
t = Mx,\qquad
\ell = Lx
$$

where:

- $M$ has dimensions $r \times n$;
- $L$ has dimensions $s \times n$;
- $t$ is the LSB-first bit vector of the Target ID;
- $\ell$ is the LSB-first bit vector of the LA line.

The bit vectors are converted to integers as follows:

$$
\operatorname{Target}(a)
= \sum_{i=0}^{r-1} t_i 2^i
$$

$$
\operatorname{LA\_line}(a)
= \sum_{i=0}^{s-1} \ell_i 2^i
$$

The final byte address is:

$$
\operatorname{LA\_byte}(a)
= G \cdot \operatorname{LA\_line}(a) + o
$$

### 2.4 Three Levels of Correctness

The three levels must be checked and reported separately.

#### 2.4.1 Target Reachability

Every Target must be reachable by at least one input address.

The criterion is:

$$
\operatorname{rank}_{GF(2)}(M)=r
$$

If the rank is less than $r$, some Targets can never be selected and the Mapping fails.

#### 2.4.2 Mapping Bijectivity

Define the combined matrix:

$$
F =
\begin{bmatrix}
M \\
L
\end{bmatrix}
$$

Because $r+s=n$, $F$ is an $n \times n$ square matrix.

The Mapping is bijective if and only if:

$$
\operatorname{rank}_{GF(2)}(F)=n
$$

This condition guarantees that every input line address maps to a unique `(Target, LA line)` pair and that the valid output space contains neither collisions nor holes.

#### 2.4.3 Natural LA Order Within Each Target

Split the line address into low and high bits:

$$
p =
\begin{bmatrix}
x_0 & \cdots & x_{r-1}
\end{bmatrix}^{\mathsf T},
\qquad
u =
\begin{bmatrix}
x_r & \cdots & x_{n-1}
\end{bmatrix}^{\mathsf T}
$$

Here, $p$ contains $r$ bits and $u$ contains $s$ bits. In column order:

$$
x =
\begin{bmatrix}
p \\
u
\end{bmatrix},
\qquad
M =
\begin{bmatrix}
M_p & M_u
\end{bmatrix}
$$

This specification gives “natural LA order” a strict meaning: after fixing any Target, the LA line must equal the high $s$ bits $u$ of the input line address and must cover the natural binary sequence `0, 1, 2, ...` in full.

Both of the following conditions must therefore hold:

$$
\operatorname{rank}_{GF(2)}(M_p)=r
$$

and:

$$
L =
\begin{bmatrix}
0_{s\times r} & I_s
\end{bmatrix}
$$

The first condition guarantees that, for a fixed Target and any $u$, there is exactly one corresponding low-bit vector $p$. The second guarantees that the LA line is exactly $u$, without bit permutation, XOR reordering, or a Target-dependent offset.

Even if the Mapping as a whole is bijective, failure of either condition must be reported as “the Mapping is valid, but the LA does not have natural order.” It must not be misreported as a bijectivity failure.

LA-bit permutation, XOR reordering, or a Target-dependent offset may harm continuity and locality within each Target. The warning must therefore be displayed prominently and retained in `validate`, `map`, and `run` output. It does not, however, change the fact that the Mapping is bijective or prevent further analysis.

When $r=0$, $M_p$ is an empty matrix whose rank is defined as 0. When $s=0$, $I_s$ is an empty identity matrix. These degenerate cases still use the same formulas.

### 2.5 Final Classification

| Classification | Condition | Command result |
| --- | --- | --- |
| `valid_natural` | Targets reachable, Mapping bijective, and LA naturally ordered | Success |
| `valid_non_natural` | Targets reachable and Mapping bijective, but LA not naturally ordered | Success with a warning |
| `invalid_target_unreachable` | $\operatorname{rank}(M)<r$ | Failure |
| `invalid_non_bijective` | $\operatorname{rank}(M)=r$, but $\operatorname{rank}(F)<n$ | Failure |

If the input structure itself is invalid, for example because a matrix has the wrong dimensions, no final classification is produced. The specific input error is reported instead.

## 3. Stimuli and Performance Metrics

### 3.1 From Stimulus to Statistics

A performance test begins with a deterministic, ordered sequence of byte-address stimuli:

$$
a_0,a_1,\ldots,a_{Q-1}
$$

The stimuli may come from a single linear access stream or from multiple streams merged in a deterministic order. Regardless of their source, each expanded concrete test must produce exactly one address sequence.

The Mapping converts each address into a Target:

$$
y_i=\operatorname{Target}(a_i),
\qquad
0\le i<Q
$$

The analyzer therefore operates on the ordered Target sequence:

$$
y_0,y_1,\ldots,y_{Q-1}
$$

The same set of addresses in a different order may have the same long-term access totals but different short-term congestion and run behavior. The order must therefore be preserved.

For each concrete test, the tool independently calculates four categories of results:

| Category | Metric | Question answered |
| --- | --- | --- |
| A | Count and share for each Target | Is the total traffic distributed evenly? |
| B | Maximum load ratio $R_{\max}$ | How far does the busiest Target deviate from the ideal long-term load? |
| C | Short-term load ratio $R_{\mathrm{window}}(W)$ | Does local congestion occur in any contiguous window? |
| D | Longest run $L_{\mathrm{run}}$ | Do consecutive requests cluster on the same Target? |

The four categories are defined formally below. Let:

- $Q>0$ be the total number of accesses in the concrete test;
- $N$ be the number of Targets;
- $y_i\in\{0,\ldots,N-1\}$ be the Target selected by access $i$.

### 3.2 A: Accesses per Target

The number of accesses to Target $j$ is:

$$
C_j =
\sum_{i=0}^{Q-1}
\mathbf{1}[y_i=j]
$$

Its share of all accesses is:

$$
S_j=\frac{C_j}{Q}
$$

The counts must satisfy:

$$
\sum_{j=0}^{N-1}C_j=Q
$$

### 3.3 B: Maximum Load Ratio

The ideal average number of accesses per Target is $Q/N$.

$$
R_{\max}
=
\max_{0\le j<N}
\frac{C_j}{Q/N}
=
\frac{N\cdot\max_j C_j}{Q}
$$

$R_{\max}=1$ means that the long-term distribution is perfectly balanced. Larger values indicate a greater deviation between the busiest Target and the ideal average.

If several Targets tie for the largest $C_j$, the report uses the smallest Target ID as the representative. This tie-breaking rule does not affect the value of $R_{\max}$.

### 3.4 C: Short-Term Congestion

The window size $W$ is measured in **number of accesses**, not bytes, time, or cycles. Because short-term congestion depends on the observation scale, a concrete test may specify multiple values of $W$.

For example, `window_sizes: [4, 16, 64]` calculates the following independently over the same Target sequence:

- the worst distribution within any 4 consecutive accesses;
- the worst distribution within any 16 consecutive accesses;
- the worst distribution within any 64 consecutive accesses.

Each of the three window sizes produces one short-term-congestion result. They do not expand the case into three concrete tests or change the original address or Target sequence.

Every window size $W$ must satisfy $1\le W\le Q$.

Within the window that starts at index $k$, the number of accesses to Target $j$ is:

$$
C_{j,k}^{(W)}
=
\sum_{i=k}^{k+W-1}
\mathbf{1}[y_i=j],
\qquad
0\le k\le Q-W
$$

The worst load ratio for window size $W$ is:

$$
R_{\mathrm{window}}(W)
=
\max_{\substack{0\le k\le Q-W\\0\le j<N}}
\frac{C_{j,k}^{(W)}}{W/N}
=
\frac{N}{W}
\max_{k,j} C_{j,k}^{(W)}
$$

Even when $W<N$, the real-valued ideal load $W/N$ is used without rounding.

If multiple `(k, j)` pairs tie for the worst value, the report chooses a representative using the following order:

1. the smallest window start index $k$;
2. for the same start index, the smallest Target ID $j$.

### 3.5 D: Longest Consecutive Run

The longest run of consecutive accesses to the same Target is defined as:

$$
L_{\mathrm{run}}
=
\max
\left\{
d\ \middle|\
d\ge1,\quad
\exists k,j,\quad
0\le k,\quad
k+d\le Q,\quad
0\le j<N,\quad
y_k=y_{k+1}=\cdots=y_{k+d-1}=j
\right\}
$$

The report includes:

- the length $L_{\mathrm{run}}$;
- the Target ID;
- the starting access index.

If multiple longest runs have the same length, the report chooses the one with the smallest start index.

### 3.6 Metric Boundaries for sweep

Each `(base, stride)` combination in a `sweep` is an independent concrete test. It has its own $Q$, $C_j$, $R_{\max}$, $R_{\mathrm{window}}$, and $L_{\mathrm{run}}$.

v1 defines no aggregate metrics across combinations.

### 3.7 Operational Limits and Deterministic Completion

v1 accepts work only within the following inclusive limits:

| Resource | Limit |
| --- | --- |
| Raw bytes in each Mapping or Scenario source, whether a file or standard input | `16 MiB` |
| Address operands in one `map` command | `1,000,000` |
| `targets.count` ($N$) | `65,536` |
| `address.granule_bytes` ($G$) | $2^{52}$ |
| Accesses ($Q$) in one concrete test | `10,000,000` |
| Concrete tests expanded by one `run` | `10,000` |
| Total generated accesses $\sum Q$ in one `run` | `100,000,000` |
| Streams in one `multi_stream` case | `4,096` |
| Effective window sizes in one case | `1,024` |
| UTF-8 bytes in each Mapping name, Scenario case name, or stream name after YAML decoding | `128` |
| Target rows in one complete report, summed across concrete tests | `1,000,000` |
| Window rows in one complete report, summed across concrete tests | `1,000,000` |
| Exact window work $\sum(Q\cdot K_{\mathrm{effective}})$ in one `run` | `100,000,000` |
| Bytes in one complete rendered text or JSON report, including its single trailing LF | `268,435,456` |

Every product, sum, $2^A$ bound, sweep cardinality, stream total, and last-address expression used in preflight is evaluated with checked `u128` arithmetic before any proven narrowing. A checked-arithmetic failure is the same deterministic limit failure as exceeding the corresponding bound; it is never allowed to wrap, truncate, or begin partial analysis.

A Mapping limit violation is `mapping.unsupported` and exits 2. A Scenario or expanded-run limit violation is `scenario.invalid` and exits 3. Name-length failures use the owning schema code. An oversized raw input uses `input.invalid_value`, with exit 2 for a Mapping source and exit 3 for a Scenario source. Exceeding the `map` operand count also uses `input.invalid_value` and exits 3. A rendered-report limit violation uses `output.too_large` and exits 1. Only an unexpected failure after every applicable input, range, address, resource, and rendered-size gate has passed uses `analysis.failed` and exits 4.

These limits are part of the input/output contract, not best-effort targets. Inputs and reports at or below every applicable limit complete deterministically unless an I/O failure occurs or the below-limit exceptional `analysis.failed` condition is reported. No partial Mapping query, concrete test, or report is emitted.

## 4. User Input Format

### 4.1 Format Selection

v1 accepts YAML 1.2 configuration files only:

- a Mapping uses one YAML file;
- a Scenario uses a separate YAML file;
- JSON is used only for structured output;
- TOML and JSON input are outside the scope of v1.

YAML was chosen because XOR tap lists, scenario lists, and comments are easier for people to write and review in YAML.

Each source must be UTF-8 and contain exactly one YAML document whose root is a block-style mapping. A flow-style root is rejected even when its values would otherwise match the schema; therefore a JSON object is not accepted as configuration regardless of filename extension or use of standard input. Nested flow-style sequences, such as the tap and window lists in this specification, remain valid where the schema permits a sequence.

One UTF-8 BOM is accepted only as the first three bytes of a source. UTF-16 and UTF-32 BOMs are rejected, as are a second BOM and U+FEFF anywhere after byte zero. YAML anchors, aliases, merge keys, every explicit tag, duplicate keys, non-string mapping keys, multiple documents, and unknown fields are rejected. Keys are case-sensitive, and `schema_version` must be the integer `1`.

Scalar resolution follows YAML 1.2. In particular, plain `yes` and `on` are strings: they are valid in a string field such as `name`, but are type errors in a boolean field such as `enabled`. Only plain `true` and `false` satisfy a boolean field.

An address field represented by a plain YAML scalar, and every command-line address operand, accepts exactly one of these lexemes:

```text
decimal: 0|[1-9][0-9]*(?:_[0-9]+)*
hex:     0x[0-9A-Fa-f]+(?:_[0-9A-Fa-f]+)*
```

An underscore is allowed only between digits of the same numeral. Leading, trailing, doubled, and prefix-adjacent underscores are invalid. Canonical output is lowercase hexadecimal with no underscore.

A generic integer field represented by a plain YAML scalar accepts exactly:

```text
0|[1-9][0-9]*|0x[0-9A-Fa-f]+
```

Generic integers therefore accept no sign, leading decimal zero, or underscore. Quoted numeric text is a type error for both address and generic integer fields.

Validation follows this fixed ladder:

1. validate command-line grammar;
2. bounded-read each input in command order until EOF or byte `16 MiB + 1`;
3. on byte `16 MiB + 1`, stop reading immediately and reject the source before UTF-8 or YAML inspection;
4. preflight the output destination;
5. validate UTF-8 and YAML syntax/document rules;
6. decode and validate the whole document schema, including unselected Scenario cases;
7. validate Mapping scalars, relationships, and v1 caps;
8. validate matrix dimensions and taps;
9. calculate all three mathematical checks;
10. select Scenario cases;
11. resolve inheritance and validate semantics for selected cases;
12. preflight checked expansion, resources, and every query or generated address;
13. perform analysis;
14. render the complete report;
15. atomically commit file output.

“Read fully” means reaching EOF only within the 16 MiB envelope. One bounded reader is used for regular files, standard input, FIFOs, and device-like named inputs; it retains at most `16 MiB + 1` bytes. An in-envelope source is complete only when EOF is observed. On byte `16 MiB + 1`, the command stops that read immediately, does not read a later input, and reports the size issue before considering malformed UTF-8, a BOM, YAML, or the output destination. Mapping input always precedes Scenario input. For accepted-size snapshots, every input is acquired before output preflight or content parsing; a destination error then exits 1 before content parsing.

After the raw-size check, syntax, encoding, document, and prohibited-YAML failures produce exactly one `input.yaml_parse` issue. Choose the violation with the earliest source byte position. Failures beginning at the same byte use this total priority, from highest to lowest: invalid encoding/BOM; scanner/parser syntax; flow-style root; explicit tag; anchor; alias; merge key; non-string key; duplicate key; second-document start. A non-string key is rejected before duplicate handling, is not inserted into the duplicate-key set, and is never compared for duplicate equality. A missing document is positioned at EOF. No later or lower-priority violation is reported.

For schema decoding, walk each mapping’s present entries in source order. A present field or sequence entry emits at most its first failing constraint under Section 7.2’s order. Recurse into a valid container before moving to the next present sibling; a missing or wrong-typed parent suppresses synthetic descendant issues. After all present entries in a mapping, emit absent required fields in the canonical order below:

- Mapping root: `schema_version`, `name`, `address`, `targets`, `mapping`;
- `address`: `width_bits`, `granule_bytes`;
- `targets`: `count`;
- `mapping`: `m`, `l`;
- `m`: `rows`;
- `l`: `mode`, `rows`, where `rows` is required only when `mode` is `explicit`;
- Scenario root: `schema_version`, `defaults`, `cases`;
- `defaults`: `accesses`, `window_sizes`;
- `stride` or `sweep` case: `name`, `enabled`, `kind`, `window_sizes`, `base_bytes`, `stride_bytes`, `accesses`; only `name`, `kind`, `base_bytes`, and `stride_bytes` are intrinsically required, while `accesses` may inherit;
- `multi_stream` case: `name`, `enabled`, `kind`, `window_sizes`, `schedule`, `streams`;
- stream: `name`, `base_bytes`, `stride_bytes`, `accesses`.

Optional and conditional fields never produce a missing issue unless their stated condition makes them required. Missing container fields use the container’s own path at its listed position and suppress every descendant. Array paths are zero-based, for example `cases[2].streams[1].accesses`, `mapping.m.rows[2][1]`, and `addresses[3]`; a command or command-line-option issue uses the empty path `""`.

Known schema-field path segments are identifiers joined by `.`, and sequence positions use zero-based `[n]`. A raw user-supplied mapping key used in an unknown-key or duplicate-key path is never treated as an identifier: append it as `[<canonical-JSON-string>]`. Thus root key `bad.key` has path `["bad.key"]`, while unknown key `x` under `address` has path `address["x"]`. Quote, backslash, control, line-separator, dot, and bracket characters use the canonical JSON escaping from Section 7.2. Text output preserves those escape bytes exactly and never decodes them before terminal rendering. A duplicate-key issue uses the encoded full path and source position of the second occurrence.

For every Scenario case, process the common fields `name`, `enabled`, `kind`, and `window_sizes` in source order even when `kind` is missing, has the wrong type, or is not a supported literal. Until `kind` passes both its string and allowed-value gates, the recognized-key set is the union of the common names and all declared kind-specific names: `base_bytes`, `stride_bytes`, `accesses`, `schedule`, and `streams`. This union is used only to decide whether a key is unknown. Every required, forbidden, type, value, shape, uniqueness, inheritance, and resource check that depends on a case kind is suppressed. A key outside the common-plus-union set still emits `input.unknown_field`. Once `kind` is valid, use only that kind’s exact allowed fields and constraints. Consequently, a missing, wrong-typed, or unsupported `kind` never creates synthetic `base_bytes`, `stride_bytes`, `schedule`, `streams`, or forbidden-`accesses` issues.

### 4.2 Mapping File

#### 4.2.1 Complete Example

```yaml
schema_version: 1
name: example-4-target

address:
  width_bits: 20
  granule_bytes: 64

targets:
  count: 4

mapping:
  m:
    rows:
      - [0, 4, 8]
      - [1, 5, 9]

  l:
    mode: preserve_high
```

#### 4.2.2 Field Definitions

| Path | Type | Required | Meaning |
| --- | --- | --- | --- |
| `schema_version` | integer | Yes | Fixed at `1` |
| `name` | string | Yes | Non-empty, human-readable Mapping name, at most 128 UTF-8 bytes after decoding |
| `address.width_bits` | integer | Yes | $A$, in the range `1..64` |
| `address.granule_bytes` | integer | Yes | $G$, which must be a power of two |
| `targets.count` | integer | Yes | $N$, which must be a power of two |
| `mapping.m.rows` | array of integer arrays | Yes | XOR taps for each Target output bit |
| `mapping.l.mode` | string | Yes | `preserve_high` or `explicit` |
| `mapping.l.rows` | array of integer arrays | See below | XOR taps for each explicit LA output bit |

`mapping.m.rows[i]` generates Target bit $t_i$. For example, `[0, 4, 8]` means:

$$
t_i=x_0\oplus x_4\oplus x_8
$$

Each tap is the number of a bit in the input vector $x$ and must satisfy `0 <= tap < n`.

Similarly, `mapping.l.rows[i]` generates LA-line bit $\ell_i$. In both sets of rows, output bits are listed from least significant to most significant.

The row-count requirements are:

- `mapping.m.rows` must contain exactly $r$ rows;
- in `explicit` mode, `mapping.l.rows` must contain exactly $s$ rows;
- a row must not contain the same tap more than once;
- tap order within a row has no semantic effect;
- an empty row represents the constant 0 and is syntactically valid, although it may cause a rank check to fail.

#### 4.2.3 LA Modes

`preserve_high` means:

$$
L =
\begin{bmatrix}
0_{s\times r} & I_s
\end{bmatrix}
$$

In this mode, `mapping.l.rows` must not also be present.

`explicit` means that the user supplies $L$ one row at a time. Each row still defines an XOR operation over GF(2), not ordinary integer addition.

The configuration format allows an explicit $L$ to use any syntactically valid taps. Whether it is acceptable is determined by the rank and natural-order checks in Chapter 2.

The following expands `preserve_high` into equivalent explicit rows, so it still passes the natural LA check:

```yaml
mapping:
  m:
    rows:
      - [0, 4, 8]
      - [1, 5, 9]
  l:
    mode: explicit
    rows:
      - [2]
      - [3]
      - [4]
      - [5]
      - [6]
      - [7]
      - [8]
      - [9]
      - [10]
      - [11]
      - [12]
      - [13]
```

`mapping.l.rows` is required in `explicit` mode. If an explicit matrix is exactly equal to the preserve-high matrix, it is still classified as having natural LA order.

The following explicit $L$ applies an XOR transformation while remaining bijective:

```yaml
mapping:
  m:
    rows:
      - [0, 4, 8]
      - [1, 5, 9]
  l:
    mode: explicit
    rows:
      - [2, 3]  # LA bit 0 = x[2] XOR x[3]
      - [3]     # LA bit 1 = x[3]
      - [4]
      - [5]
      - [6]
      - [7]
      - [8]
      - [9]
      - [10]
      - [11]
      - [12]
      - [13]
```

This example changes only the two least-significant LA bits:

$$
\ell_0=x_2\oplus x_3,\qquad
\ell_1=x_3
$$

The remaining LA bits still preserve their corresponding high input bits directly. The transformation matrix over the high LA bits is invertible, so the combined matrix $F$ remains full rank:

- the Target reachability check passes;
- the Mapping bijectivity check passes;
- because $L\ne[0\ I_s]$, the natural LA order check produces a warning;
- the final classification is `valid_non_natural`; `validate` exits with code 0, and `map` and `run` may continue, but they must retain the warning.

An explicit Mapping can therefore describe and use a custom XOR transformation. Any $L$ that differs from $[0\ I_s]$ loses natural LA order, but the Mapping remains valid as long as the Targets are reachable and $F$ is full rank.

### 4.3 Scenario File

A Scenario file describes how to generate the concrete tests defined in Section 3.1. A `case` in the file is a scenario declared by the user. After default inheritance and any required combination expansion, a case produces one or more concrete tests.

A concrete test must contain exactly one deterministic, ordered sequence of byte addresses. Every performance metric in Chapter 3 is calculated independently over that sequence and is never merged across tests.

#### 4.3.1 What a Concrete Test Looks Like

For example, consider “start at `0x0`, advance by 64 bytes each time, and perform 4 accesses.” The test proceeds as follows:

```text
Scenario parameters
  base = 0x0
  stride = 64
  accesses = 4

Generate the ordered address sequence
  [0x0, 0x40, 0x80, 0xc0]

Apply the Mapping to produce the ordered Target sequence
  [Target(0x0), Target(0x40), Target(0x80), Target(0xc0)]

Calculate over that Target sequence
  per-Target counts, R_max, R_window for each window, and the longest run
```

Address order is part of the test. The same addresses in a different order may produce the same long-term counts but different short-term windows and longest runs.

The tool processes a Scenario in the following fixed order:

1. select cases according to `enabled` and `--case`;
2. resolve the defaults for each case;
3. expand each case into one or more concrete tests;
4. generate an ordered byte-address sequence for each concrete test;
5. use the Mapping to produce the corresponding Target sequence;
6. calculate and report metrics for each concrete test independently.

#### 4.3.2 How the Three Scenario Kinds Expand

| kind | What the user describes | Expansion result | Primary use |
| --- | --- | --- | --- |
| `stride` | One base, one stride, and an access count | 1 concrete test | Observe one fixed linear access pattern |
| `sweep` | Multiple bases and multiple strides | 1 concrete test for each `(base, stride)` combination | Compare phase and stride changes |
| `multi_stream` | Multiple streams and a merge order | 1 merged concrete test | Observe combined traffic from multiple masters or request sources |

For `stride` and `multi_stream`, `case_id` equals the case name. Each combination in a `sweep` has its own `case_id`, as defined in Section 4.3.5.

#### 4.3.3 Common Fields

| Path | Type | Required | Meaning |
| --- | --- | --- | --- |
| `schema_version` | integer | Yes | Fixed at `1` |
| `defaults.accesses` | integer | Yes | Default access count for `stride` and `sweep`; must be greater than 0 |
| `defaults.window_sizes` | integer array | Yes | Default list of short-term-congestion windows, measured in accesses |
| `cases` | case array | Yes | At least one scenario |
| `cases[].name` | string | Yes | Unique within the file; format defined below |
| `cases[].enabled` | boolean | No | Defaults to `true` |
| `cases[].kind` | string | Yes | `stride`, `sweep`, or `multi_stream` |
| `cases[].window_sizes` | integer array | No | Overrides the default list of short-term-congestion windows |

The window list must be non-empty, contain no duplicates, and contain only values greater than 0. Each value is an independent $W$ in the formula from Section 3.4. The list does not increase the number of concrete tests.

The effective access count for `stride` and `sweep` is the case's own `accesses`, or `defaults.accesses` when omitted. The effective window list for every case is the case's own `window_sizes`, or `defaults.window_sizes` when omitted. Window validity is checked after inheritance, against the final list.

A case name must match:

```text
[A-Za-z0-9][A-Za-z0-9._-]*
```

This restriction allows a name to be passed directly to `--case` and prevents collisions with automatically generated `sweep` combination IDs. Each case name is at most 128 UTF-8 bytes after YAML decoding.

#### 4.3.4 stride

Fields:

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `base_bytes` | address scalar | Yes | First byte address |
| `stride_bytes` | address scalar | Yes | Byte distance between adjacent accesses; may be 0 |
| `accesses` | integer | No | Number of accesses; inherits `defaults.accesses` when omitted |

For an access count $Q$, the generated sequence is:

$$
a_i=\operatorname{base}+i\cdot\operatorname{stride},
\qquad 0\le i<Q
$$

A base or stride that is not aligned to $G$ is valid. The Mapping operates independently on each generated byte address and preserves its byte offset.

A `stride` case produces exactly one concrete test, whose `case_id` equals `name`.

#### 4.3.5 sweep

Fields:

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `base_bytes` | address scalar array | Yes | Non-empty list of unique bases |
| `stride_bytes` | address scalar array | Yes | Non-empty list of unique strides |
| `accesses` | integer | No | Number of accesses per combination; inherits the default when omitted |

A `sweep` takes the Cartesian product of its bases and strides. The order is fixed:

1. take each base in `base_bytes` declaration order;
2. for each base, take each stride in `stride_bytes` declaration order.

Each combination has its own independent metrics. Combinations are neither concatenated nor aggregated.

The combination ID in output has the fixed form:

```text
<case-name>[base=<canonical-hex>,stride=<canonical-hex>]
```

For example:

```text
stride-and-phase-sweep[base=0x40,stride=0x100]
```

#### 4.3.6 multi_stream

`schedule` defines how addresses from multiple streams are merged into one final stimulus sequence. The scheduling order directly affects short-term congestion and longest runs, so it must be explicit and reproducible.

Fields:

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `schedule` | string | Yes | Stream merge policy; v1 supports only `round_robin` |
| `streams` | stream array | Yes | At least one stream |
| `streams[].name` | string | Yes | Unique within the case, subject to the case-name character rules, and at most 128 UTF-8 bytes after decoding |
| `streams[].base_bytes` | address scalar | Yes | First byte address in the stream |
| `streams[].stride_bytes` | address scalar | Yes | Stream stride; may be 0 |
| `streams[].accesses` | integer | Yes | Number of accesses in the stream; must be greater than 0 |

Each stream first generates its own address sequence using the `stride` formula.

`round_robin` merges the streams as follows:

1. in `streams` declaration order, take one address from each stream that has not ended;
2. skip streams that have ended;
3. repeat until all streams have ended.

For example:

```text
master0: [A0, A1, A2]
master1: [B0, B1]

round_robin result:
[A0, B0, A1, B1, A2]
```

The total number of accesses in a multi-stream scenario is therefore:

$$
Q=\sum_h Q_h
$$

where $Q_h$ is the number of accesses in stream $h$.

A multi-stream case does not accept the case-level `accesses` field.

A multi-stream case produces exactly one merged concrete test, whose `case_id` equals `name`.

#### 4.3.7 Complete Example

With the test model above in mind, the following complete file contains all three scenario kinds:

```yaml
schema_version: 1

defaults:
  accesses: 4096
  window_sizes: [4, 16, 64]

cases:
  - name: sequential
    enabled: true
    kind: stride
    base_bytes: 0x0
    stride_bytes: 64

  - name: stride-and-phase-sweep
    enabled: true
    kind: sweep
    base_bytes: [0x0, 0x40, 0x80, 0xc0]
    stride_bytes: [64, 128, 256]
    accesses: 4096
    window_sizes: [4, 16, 64, 256]

  - name: two-master
    enabled: true
    kind: multi_stream
    schedule: round_robin  # Take one address from each stream in declaration order per round
    window_sizes: [4, 16, 64]
    streams:
      - name: master0
        base_bytes: 0x0
        stride_bytes: 256
        accesses: 2048
      - name: master1
        base_bytes: 0x40
        stride_bytes: 256
        accesses: 2048
```

When all three cases are enabled, this file expands as follows:

- `sequential`: 1 concrete test;
- `stride-and-phase-sweep`: $4\times3=12$ concrete tests;
- `two-master`: 2 streams merged into 1 concrete test.

The tool therefore runs and reports 14 independent concrete tests in deterministic order.

## 5. Command-Line Interface

The binary command name is fixed as `interleave`.

### 5.1 General Conventions

- `--help` displays help and exits with code 0;
- `--version` displays exactly `interleave 0.1.0` and exits with code 0;
- the initial v1 package version is `0.1.0`;
- an input path of `-` reads from standard input;
- an output path of `-` writes to standard output;
- output is written to standard output when `--output` is omitted;
- `text` is used when `--format` is omitted;
- `--format` accepts only `text` or `json`;
- an existing output file is not overwritten by default; `--force` is required to overwrite it;
- `--force` requires a path-valued `--output`, not an omitted output or `-`; the path may be nonexistent, while an existing target must be a regular file and must not be a symlink;
- at most one input file for a command may come from standard input.

### 5.2 Generate Templates

```text
interleave template mapping  --output <FILE> [--force]
interleave template scenario --output <FILE> [--force]
```

Behavior:

- generate commented YAML;
- generated output must be accepted directly by the corresponding command;
- the `template` command does not support `--format`;
- `--output` is required.

The Mapping template body is exactly the following:

```yaml
# Interleave Mapping template (schema v1).
# XOR tap indices refer to line-address bits x0, x1, ... from least significant upward.
schema_version: 1
name: example-4-target

address:
  width_bits: 20
  granule_bytes: 64

targets:
  count: 4

mapping:
  m:
    rows:
      - [0, 4, 8]
      - [1, 5, 9]
  l:
    # preserve_high keeps naturally ordered local-address bits.
    mode: preserve_high
```

The Scenario template body is exactly the following:

```yaml
# Interleave Scenario template (schema v1).
# Every effective window size is measured in accesses and must not exceed its test length.
schema_version: 1

defaults:
  accesses: 4096
  window_sizes: [4, 16, 64]

cases:
  - name: sequential
    enabled: true
    kind: stride
    base_bytes: 0x0
    stride_bytes: 64

  - name: stride-and-phase-sweep
    enabled: true
    kind: sweep
    base_bytes: [0x0, 0x40, 0x80, 0xc0]
    stride_bytes: [64, 128, 256]
    accesses: 4096
    window_sizes: [4, 16, 64, 256]

  - name: two-master
    enabled: true
    kind: multi_stream
    # round_robin takes one address from each active stream in declaration order.
    schedule: round_robin
    window_sizes: [4, 16, 64]
    streams:
      - name: master0
        base_bytes: 0x0
        stride_bytes: 256
        accesses: 2048
      - name: master1
        base_bytes: 0x40
        stride_bytes: 256
        accesses: 2048
```

Each template is emitted as exactly the UTF-8 body shown inside its fence, using LF line endings, no BOM, and exactly one trailing newline.

### 5.3 Validate a Mapping

```text
interleave validate
  --spec <MAPPING_YAML>
  [--format text|json]
  [--output <FILE|->]
  [--force]
  [--verbose]
```

Behavior:

- check the input structure and all three correctness levels from Chapter 2;
- in text output, `--verbose` appends the complete $M$, $L$, $F$, and $M_p$ binary matrices;
- `--verbose` and `--format json` are mutually exclusive.

### 5.4 Map Addresses

```text
interleave map
  --spec <MAPPING_YAML>
  <ADDRESS>...
  [--format text|json]
  [--output <FILE|->]
  [--force]
```

Behavior:

- require at least one address;
- accept non-negative decimal or `0x`-prefixed hexadecimal addresses;
- preserve command-line address order;
- validate the Mapping before processing addresses;
- allow queries against both `valid_natural` and `valid_non_natural` Mappings;
- retain the `mapping.non_natural` warning in query reports for a `valid_non_natural` Mapping;
- produce no partial query results if the Mapping is invalid or any address is out of range.

### 5.5 Run Scenarios

```text
interleave run
  --spec <MAPPING_YAML>
  --scenario <SCENARIO_YAML>
  [--case <NAME>]...
  [--format text|json]
  [--output <FILE|->]
  [--force]
```

Behavior:

- when no `--case` is specified, run every case with `enabled: true`;
- when `--case` is specified, match case names exactly and ignore their `enabled` values;
- allow `--case` to be repeated to select multiple cases;
- if the same name is selected more than once, run it only once;
- always run cases in their declaration order in the Scenario file;
- fail if a requested name is not found or no case is ultimately selected;
- allow both `valid_natural` and `valid_non_natural` Mappings;
- retain the `mapping.non_natural` warning in scenario reports for a `valid_non_natural` Mapping;
- validate every selected scenario before execution and produce no partial analysis if any scenario is invalid.

Each distinct requested name that is absent produces one `scenario.case_not_found` issue in first command-line occurrence order, with path `""`; repeated absent names are deduplicated. If any requested name is absent, `scenario.no_case_selected` is not also emitted. Otherwise, an empty final selection produces exactly one `scenario.no_case_selected`. Issues in selected cases are ordered by Scenario declaration order and then by field/source order.

## 6. Output Format

### 6.1 Output Channels

When using `text`:

- successful reports are written to standard output or to the file specified by `--output`;
- input errors and failure reasons are written to standard error;
- text output is intended only for human readers and is not a stable machine-readable interface.

When using `json`:

- a complete JSON document is emitted for both success and business-logic failure;
- JSON is written to standard output or to the file specified by `--output`;
- no ordinary text is mixed before or after the JSON document;
- standard error is used only for command-line syntax errors, inaccessible input or output files, or failure to generate the JSON document itself.

The complete destination rule is:

| Outcome | Text | JSON |
| --- | --- | --- |
| Command-line or filesystem failure | Diagnostic on standard error; no report | Diagnostic on standard error; no report |
| Business success | Complete report at the chosen report destination | One complete envelope at the chosen report destination |
| Business failure during parse, schema, mathematics, preflight, or analysis | Complete failure report on standard error; standard output is empty and an output file is untouched | One complete failure envelope at the chosen report destination |
| `output.exists`, invalid output target, atomic-output failure, or `output.too_large` | Diagnostic on standard error, exit 1; the refused destination is untouched | Diagnostic on standard error, exit 1; no envelope can be written to the refused destination |

For this matrix, the chosen report destination is standard output when `--output` is omitted or is `-`, and otherwise is the named file. Every text or JSON report, including a success, validation failure, business-error envelope, or verbose matrix report, includes exactly one trailing LF and is limited to `268435456` bytes including that LF. Rendering uses a counting bounded sink. On attempted byte `268435457`, stop immediately, discard every partial buffer or temporary file, emit exactly `report exceeds v1 limit 268435456 bytes` with `output.too_large`, path `""`, and exit 1 on standard error, and leave the destination untouched. This failure is never wrapped in a JSON envelope. A render or report-write failure is a filesystem/output failure, never a partial business report.

### 6.2 Text Reports

#### validate

The report must present the following items in order:

1. Mapping name and input file;
2. derived parameters $A,G,g,n,N,r,s$;
3. input-structure check;
4. `rank(M)` and the Target-reachability conclusion;
5. `rank(F)` and the bijectivity conclusion;
6. `rank(M_p)`, the $L$ comparison, and the natural-LA-order conclusion;
7. final classification and any warning or error.

With `--verbose`, the complete matrices are inserted between the derived parameters and the check results. Matrix columns are always ordered as `x0, x1, ..., x(n-1)`. The rows of $F$ consist of all rows of $M$ followed by all rows of $L$. $M_p$ contains only the first $r$ columns of $M$.

The following is the complete `--verbose` text output when the Mapping from Section 4.2.1 passes validation:

```text
Mapping: example-4-target
Input: mapping.yaml
Derived: A=20, G=64, g=6, n=14, N=4, r=2, s=12

M (2 x 14; columns x0..x13)
  t0   1 0 0 0 1 0 0 0 1 0 0 0 0 0
  t1   0 1 0 0 0 1 0 0 0 1 0 0 0 0

L (12 x 14; columns x0..x13)
  l0   0 0 1 0 0 0 0 0 0 0 0 0 0 0
  l1   0 0 0 1 0 0 0 0 0 0 0 0 0 0
  l2   0 0 0 0 1 0 0 0 0 0 0 0 0 0
  l3   0 0 0 0 0 1 0 0 0 0 0 0 0 0
  l4   0 0 0 0 0 0 1 0 0 0 0 0 0 0
  l5   0 0 0 0 0 0 0 1 0 0 0 0 0 0
  l6   0 0 0 0 0 0 0 0 1 0 0 0 0 0
  l7   0 0 0 0 0 0 0 0 0 1 0 0 0 0
  l8   0 0 0 0 0 0 0 0 0 0 1 0 0 0
  l9   0 0 0 0 0 0 0 0 0 0 0 1 0 0
  l10  0 0 0 0 0 0 0 0 0 0 0 0 1 0
  l11  0 0 0 0 0 0 0 0 0 0 0 0 0 1

F (14 x 14; rows t0,t1,l0..l11; columns x0..x13)
  1 0 0 0 1 0 0 0 1 0 0 0 0 0
  0 1 0 0 0 1 0 0 0 1 0 0 0 0
  0 0 1 0 0 0 0 0 0 0 0 0 0 0
  0 0 0 1 0 0 0 0 0 0 0 0 0 0
  0 0 0 0 1 0 0 0 0 0 0 0 0 0
  0 0 0 0 0 1 0 0 0 0 0 0 0 0
  0 0 0 0 0 0 1 0 0 0 0 0 0 0
  0 0 0 0 0 0 0 1 0 0 0 0 0 0
  0 0 0 0 0 0 0 0 1 0 0 0 0 0
  0 0 0 0 0 0 0 0 0 1 0 0 0 0
  0 0 0 0 0 0 0 0 0 0 1 0 0 0
  0 0 0 0 0 0 0 0 0 0 0 1 0 0
  0 0 0 0 0 0 0 0 0 0 0 0 1 0
  0 0 0 0 0 0 0 0 0 0 0 0 0 1

Mp (2 x 2; columns x0..x1)
  t0   1 0
  t1   0 1

PASS  input structure
PASS  target reachable: rank(M)=2, expected 2
PASS  bijective: rank(F)=14, expected 14
PASS  natural LA: rank(Mp)=2 and L=[0 I]

Result: valid_natural
```

If validation fails, the complete failure report is written to standard error and standard output remains empty. The command must not create, truncate, or replace the `--output` file. All three mathematical levels are still shown separately so that users can see exactly where validation failed. The following example has reachable Targets, but an error in $L$ makes the combined matrix rank-deficient and also destroys natural LA order:

```text
Mapping: broken-4-target
Input: broken-mapping.yaml
Derived: A=20, G=64, g=6, n=14, N=4, r=2, s=12

PASS  input structure
PASS  target reachable: rank(M)=2, expected 2
FAIL  bijective: rank(F)=13, expected 14
FAIL  natural LA: rank(Mp)=2, but L != [0 I]

Result: invalid_non_bijective
ERROR [mapping.non_bijective] mapping.l.rows: rank(F)=13, expected 14
```

#### map

The report first shows the Mapping information and validation classification, then emits the address table in command-line input order. Each input address corresponds to exactly one row. `Offset` is the byte offset within the original address's granule, and `LA byte` equals `G * LA line + Offset`.

For example:

```text
Mapping: example-4-target
Input: mapping.yaml
Result: valid_natural

Address  Line address  Offset  Target  LA line  LA byte
0x0      0x0           0x0     0       0x0      0x0
0x40     0x1           0x0     1       0x0      0x0
0x80     0x2           0x0     2       0x0      0x0
0xc0     0x3           0x0     3       0x0      0x0
0x1234   0x48          0x34    0       0x12     0x4b4
```

By default, every address uses canonical lowercase hexadecimal: a lowercase `0x` prefix, lowercase hexadecimal digits, and no leading zeros except in `0x0`.

#### run

Each expanded scenario has its own section containing at least:

- the case ID;
- the total number of accesses;
- count and share for each Target;
- $R_{\max}$;
- $R_{\mathrm{window}}$, Target, start index, and count for each window;
- the length, Target, and start index of the longest run;
- any Mapping or Scenario warning.

The beginning of the report shows the Mapping information and validation classification. Within each scenario section, the Target table is ordered by ascending Target ID and includes Targets with a count of 0. The window table follows the declaration order of the effective `window_sizes`.

The following text output represents the same `sequential` analysis result as the JSON example in Section 6.6:

```text
Mapping: example-4-target
Input: mapping.yaml
Result: valid_natural

Case: sequential
Source case: sequential
Accesses: 4096

Targets
Target  Count  Share
0       1024   0.250000
1       1024   0.250000
2       1024   0.250000
3       1024   0.250000

Max load
Target  Count  Ratio
0       1024   1.000000

Short-term windows
Size  Target  Start index  Count  Ratio
4     1       13           2      2.000000
16    1       1            5      1.250000
64    1       193          17     1.062500

Longest run
Length  Target  Start index
2       2       31

Warnings: none
```

All ratios in text output are shown with 6 digits after the decimal point.

### 6.3 Common JSON Envelope

The top-level JSON structure is fixed:

```json
{
  "schema_version": 1,
  "command": "validate",
  "status": "pass",
  "warnings": [],
  "errors": [],
  "result": {}
}
```

Field definitions:

| Field | Type | Meaning |
| --- | --- | --- |
| `schema_version` | integer | Fixed at `1` |
| `command` | string | `validate`, `map`, or `run` |
| `status` | string | `pass`, `warning`, or `fail` |
| `warnings` | issue array | List of warnings |
| `errors` | issue array | List of errors |
| `result` | object or null | Command result; `null` when no result can be produced |

The top-level status obeys the following invariants:

- `pass`: both `warnings` and `errors` are empty;
- `warning`: `warnings` is non-empty and `errors` is empty;
- `fail`: `errors` is non-empty;
- `result` is `null` when YAML parsing or structural validation fails;
- when a mathematical check in `validate` fails, `result` still contains all completed checks and the failure classification;
- when preflight validation for `map` or `run` fails, `result` is `null`.

The issue structure is fixed:

```json
{
  "code": "mapping.non_bijective",
  "path": "mapping.l.rows",
  "message": "rank(F)=13, expected 14"
}
```

`path` uses the zero-based field-path syntax defined in Section 4.1. When an issue does not correspond to an input field, it is an empty string.

JSON object key order is not part of the contract. Array order explicitly defined by this specification is part of the contract.

Issue arrays are ordered first by validation phase. Schema issues then follow Section 4.1’s recursive present-entry order and documented-table fallback order for missing fields. Selected-case semantic issues follow Scenario declaration order and then field/source order.

Every address in JSON is a canonical hexadecimal string:

- lowercase `0x` prefix;
- lowercase hexadecimal digits;
- no leading zeros except in `0x0`.

Every ratio contains an exact fraction and a decimal for display:

```json
{
  "numerator": 4,
  "denominator": 4,
  "decimal": "1.000000"
}
```

`numerator/denominator` is authoritative and is not reduced. `decimal` is rounded to 6 digits after the decimal point using exact integer round-half-up; no ratio calculation uses floating point.

Every JSON numeric field is bounded as follows:

| Numeric fields | Bound |
| --- | --- |
| Top-level `schema_version` | exactly `1` |
| `derived.address_width_bits`, `derived.offset_bits`, `derived.line_bits`, `derived.target_bits`, `derived.local_address_bits`; all `rank_m`, `rank_f`, and `rank_m_low` observations and expectations | at most `64` |
| `derived.granule_bytes` | at most $2^{52}$ |
| `derived.target_count` | at most `65,536` |
| Every Target ID in `target`, including map rows, Target rows, max-load rows, windows, and longest runs | at most `65,535` |
| Per-case `accesses`; Target/window `count`; window `size` and `start_index`; longest-run `length` and `start_index`; share and ratio denominators | at most `10,000,000` |
| Target-share numerators | at most `10,000,000` |
| Maximum-load and window-ratio numerators | at most `65,536 * 10,000,000 = 655,360,000,000` |

These are every numeric field in the envelope, validate result, map result, and run result. Report row counts, test counts, and total generated-access counts are bounded by Section 3.7 before rendering. Addresses, line addresses, offsets, and local addresses remain canonical strings and are never JSON numbers. Consequently every JSON integer is at most $2^{53}-1$.

### 6.4 validate JSON Result

```json
{
  "mapping_name": "example-4-target",
  "input": "mapping.yaml",
  "derived": {
    "address_width_bits": 20,
    "granule_bytes": 64,
    "offset_bits": 6,
    "line_bits": 14,
    "target_count": 4,
    "target_bits": 2,
    "local_address_bits": 12
  },
  "checks": [
    {
      "id": "target_reachable",
      "status": "pass",
      "observed": {"rank_m": 2},
      "expected": {"rank_m": 2},
      "message": "all targets are reachable"
    },
    {
      "id": "bijective",
      "status": "pass",
      "observed": {"rank_f": 14},
      "expected": {"rank_f": 14},
      "message": "mapping is bijective"
    },
    {
      "id": "natural_local_address",
      "status": "pass",
      "observed": {
        "rank_m_low": 2,
        "l_matches_preserve_high": true
      },
      "expected": {
        "rank_m_low": 2,
        "l_matches_preserve_high": true
      },
      "message": "local address is naturally ordered"
    }
  ],
  "classification": "valid_natural"
}
```

The order of `checks` is fixed to the three checks shown in the example. Input-structure errors belong in the top-level `errors` array and must not be presented as mathematical checks.

The exact mathematical-check contract is:

| `id` | `observed` | `expected` | Pass message | Failure message |
| --- | --- | --- | --- | --- |
| `target_reachable` | `{"rank_m":actual}` | `{"rank_m":r}` | `all targets are reachable` | `rank(M)=<actual>, expected <r>` |
| `bijective` | `{"rank_f":actual}` | `{"rank_f":n}` | `mapping is bijective` | `rank(F)=<actual>, expected <n>` |
| `natural_local_address` | `{"rank_m_low":actual,"l_matches_preserve_high":bool}` | `{"rank_m_low":r,"l_matches_preserve_high":true}` | `local address is naturally ordered` | See below |

For `natural_local_address`, the failure message is exactly `rank(Mp)=<actual>, expected <r>` when only the rank predicate fails, `rank(Mp)=<actual>; L != [0 I]` when only the $L$ predicate fails, or `rank(Mp)=<actual>, expected <r>; L != [0 I]` when both fail. Its status is `pass` when its predicate passes; when the predicate fails it is `warning` only if `target_reachable` and `bijective` both pass, and otherwise is `fail`.

An invalid Mapping retains all three check objects but emits exactly one primary error. `mapping.target_unreachable` takes precedence over `mapping.non_bijective`; its path is `mapping.m.rows`. A primary `mapping.non_bijective` issue uses `mapping.l.rows` for explicit $L$ and `mapping.m.rows` for `preserve_high`.

A valid non-natural Mapping emits exactly one `mapping.non_natural` warning. Its path is `mapping.m.rows` when only low $M_p$ rank fails, `mapping.l.rows` when only $L$ differs from preserve-high, and `mapping` when both predicates fail. An invalid Mapping emits no separate non-natural warning.

### 6.5 map JSON Result

```json
{
  "mapping_name": "example-4-target",
  "mapping_classification": "valid_natural",
  "addresses": [
    {
      "input_address": "0x1234",
      "line_address": "0x48",
      "byte_offset": "0x34",
      "target": 0,
      "local_line_address": "0x12",
      "local_byte_address": "0x4b4"
    }
  ]
}
```

The order of `addresses` matches the command-line input order.

### 6.6 run JSON Result

```json
{
  "mapping_name": "example-4-target",
  "mapping_classification": "valid_natural",
  "cases": [
    {
      "case_id": "sequential",
      "source_case": "sequential",
      "accesses": 4096,
      "targets": [
        {
          "target": 0,
          "count": 1024,
          "share": {
            "numerator": 1024,
            "denominator": 4096,
            "decimal": "0.250000"
          }
        },
        {
          "target": 1,
          "count": 1024,
          "share": {
            "numerator": 1024,
            "denominator": 4096,
            "decimal": "0.250000"
          }
        },
        {
          "target": 2,
          "count": 1024,
          "share": {
            "numerator": 1024,
            "denominator": 4096,
            "decimal": "0.250000"
          }
        },
        {
          "target": 3,
          "count": 1024,
          "share": {
            "numerator": 1024,
            "denominator": 4096,
            "decimal": "0.250000"
          }
        }
      ],
      "max_load": {
        "target": 0,
        "count": 1024,
        "ratio": {
          "numerator": 4096,
          "denominator": 4096,
          "decimal": "1.000000"
        }
      },
      "windows": [
        {
          "size": 4,
          "target": 1,
          "start_index": 13,
          "count": 2,
          "ratio": {
            "numerator": 8,
            "denominator": 4,
            "decimal": "2.000000"
          }
        },
        {
          "size": 16,
          "target": 1,
          "start_index": 1,
          "count": 5,
          "ratio": {
            "numerator": 20,
            "denominator": 16,
            "decimal": "1.250000"
          }
        },
        {
          "size": 64,
          "target": 1,
          "start_index": 193,
          "count": 17,
          "ratio": {
            "numerator": 68,
            "denominator": 64,
            "decimal": "1.062500"
          }
        }
      ],
      "longest_run": {
        "length": 2,
        "target": 2,
        "start_index": 31
      }
    }
  ]
}
```

`targets` must include every Target from 0 through $N-1$ in ascending ID order, including Targets whose count is 0.

`windows` follows the declaration order of `window_sizes` in the Scenario.

For a `sweep`, `case_id` uses the combination ID defined in Section 4.3.5, while `source_case` retains the original case name.

The exact fraction fields for ratios in `run` JSON are fixed as follows:

- Target share: `numerator = C_j`, `denominator = Q`;
- maximum-load ratio: `numerator = N * max(C_j)`, `denominator = Q`;
- window ratio: `numerator = N * C_{j,k}^{(W)}`, `denominator = W`.

## 7. Errors, Exit Codes, and Atomicity

### 7.1 Exit Codes

| Exit code | Meaning |
| --- | --- |
| `0` | Command succeeded; warnings do not change the exit code |
| `1` | Command-line usage error, inaccessible input or output file, or refusal to overwrite a file |
| `2` | Mapping YAML could not be parsed, is structurally invalid, is unsupported, or failed mathematical validation |
| `3` | Scenario YAML could not be parsed, is structurally invalid, has a case-selection error, or contains an out-of-range address |
| `4` | Input was valid, but analysis could not be completed |

An invalid or out-of-range query address passed to `map` also uses exit code `3`.

The limit classifications in Section 3.7 are exhaustive: Mapping caps use `mapping.unsupported`/2; Scenario and expanded-run caps use `scenario.invalid`/3; raw-input and `map` operand-count caps use `input.invalid_value` with the command-appropriate exit 2 or 3; rendered-report size uses `output.too_large`/1. `analysis.failed`/4 is reserved solely for an unexpected below-limit failure after complete preflight.

### 7.2 Stable Issue Codes

`warnings[].code` and `errors[].code` define at least the following stable values:

| code | Severity | Meaning |
| --- | --- | --- |
| `input.yaml_parse` | error | YAML syntax, duplicate-key, or document-count error |
| `input.unknown_field` | error | An undefined field is present |
| `input.invalid_value` | error | A field has an invalid type, range, or combination |
| `mapping.unsupported` | error | The Mapping is meaningful but outside the scope of v1 |
| `mapping.target_unreachable` | error | $\operatorname{rank}(M)<r$ |
| `mapping.non_bijective` | error | $\operatorname{rank}(F)<n$ |
| `mapping.non_natural` | warning | The Mapping is bijective, but the LA is not naturally ordered |
| `scenario.invalid` | error | Scenario fields or inherited values are invalid |
| `scenario.case_not_found` | error | `--case` did not match a name |
| `scenario.no_case_selected` | error | No case was ultimately selected |
| `address.invalid` | error | Address text is not an allowed non-negative integer |
| `address.out_of_range` | error | A query or generated address exceeds the $A$-bit range |
| `output.exists` | error | The output file already exists and `--force` was not specified |
| `output.invalid_target` | error | The output path exists but is not an acceptable regular-file target |
| `output.atomic_unsupported` | error | Atomic no-clobber rename is unavailable |
| `output.io` | error | Report output could not be completed |
| `output.too_large` | error | The complete rendered report exceeds the v1 byte limit |
| `analysis.failed` | error | Input was valid, but analysis could not be completed |

An implementation may add more specific issue codes, but it must not change the meaning of the codes above.

The following issue messages are stable templates; angle-bracket terms are replaced with the canonical representation defined below:

```text
invalid YAML syntax
duplicate key <quoted-key>
expected exactly one YAML document, found <count>
unknown field <quoted-key>
expected <constraint>, observed <canonical-value>
unsupported <field>: <reason>
case <quoted-name> was not found
no scenario case was selected
invalid address <quoted-lexeme>
address <canonical> is outside the <A>-bit range
output path already exists; use --force to replace it
output target must be a regular file
atomic no-clobber rename is unsupported
report exceeds v1 limit 268435456 bytes
analysis could not be completed
```

`<quoted-key>`, `<quoted-name>`, and `<quoted-lexeme>` are complete JSON string literals, including the opening and closing double quotes. Their canonical escaping is: `"` becomes `\"`, `\` becomes `\\`, backspace/form-feed/newline/carriage-return/tab become `\b`, `\f`, `\n`, `\r`, `\t`, and every other U+0000 through U+001F scalar becomes lowercase `\u00xx`. Every other Unicode scalar is emitted literally as UTF-8; `/` is not escaped.

`<canonical-value>` is total over YAML 1.2 values. A sequence is exactly `sequence`; a mapping is exactly `mapping`; a resolved integer is ungrouped decimal with no leading zeros and exactly one leading `-` when negative; a resolved boolean or null is `true`, `false`, or `null`; and every other scalar is its decoded content as a complete JSON string literal using the preceding escaping. The last category includes quoted numeric text and every float or non-finite-looking scalar, including `1.5`, `.inf`, and `.nan`. Absence is `missing`. `<count>` and `<A>` are non-negative ungrouped decimal integers. `<canonical>` is the canonical lowercase hexadecimal address from Section 6.3. No placeholder receives locale-dependent formatting or additional quotes.

In the `unsupported` message, `<field>` is the exact issue `path` without quoting. For `mapping.unsupported`, it is therefore exactly `targets.count` or `address.granule_bytes`. Inside a conditional constraint, `<field>` is the controlling field’s full zero-based path without quoting. A missing field’s issue path is the missing field’s full path, while the observed `<canonical-value>` is `missing`.

`<constraint>` is finite and must be exactly one of the following forms:

```text
integer
boolean
string
mapping
sequence
plain integer
plain address
required field
non-empty sequence
power of two
unique values
one of <compact-JSON-string-array>
integer in [<min>,<max>]
integer <= <max>
sequence length <n>
UTF-8 byte length <= <max>
string matching <quoted-regex>
non-empty string without control or line-separator characters
field absent when <field>=<canonical-value>
field present when <field>=<canonical-value>
at most <n> raw bytes
at most <n> query addresses
sum(Q*K) <= 100000000
```

All numbers substituted into a constraint are ungrouped decimal. `<compact-JSON-string-array>` has no spaces and uses the canonical JSON string form for every element, for example `["preserve_high","explicit"]`, `["stride","sweep","multi_stream"]`, or `["round_robin"]`. `<quoted-regex>` is a complete canonical JSON string literal, for example `"[A-Za-z0-9][A-Za-z0-9._-]*"`.

For each present field or sequence entry, the normative emitter matrix below, rather than the textual order of the vocabulary, selects the applicable constraints and their order. Emit only the first failure for that field after its parent and type gates. Missing intrinsically required fields use `required field`; a conditionally forbidden or required field uses the matrix’s exact `field absent when ...` or `field present when ...` constraint. Sequence elements are processed in source order only after their container gates pass.

`targets.count` and `address.granule_bytes` have one cross-family order that overrides every other constraint/reason ordering. After `integer` and `plain integer` succeed, evaluate: (1) non-power-of-two, which emits `mapping.unsupported` with the field-specific not-a-power-of-two reason; (2) the intrinsic relation, `targets.count <= 2^n` or `address.granule_bytes <= 2^A`, which emits `input.invalid_value` with `integer <= <max>`; and (3) the v1 cap, which emits `mapping.unsupported` with the field-specific exceeds-limit reason. Stop at the first result. The `expected power of two, observed ...` branch is not applicable to either field.

`<reason>` is finite and must be exactly one of:

```text
target count is not a power of two
granule size is not a power of two
target count exceeds v1 limit 65536
granule size exceeds v1 limit 4503599627370496
```

The two target-count reasons always use `<field>` `targets.count`; the two granule-size reasons always use `<field>` `address.granule_bytes`. If more than one reason applies to one field, the order in the reason list above selects the first. No constraint or reason contains user-supplied text.

#### Normative Validation Emitter Matrix

This matrix is exhaustive for v1 input and validation diagnostics. Each row is one emitter rule; when a scope cell lists several exact paths or a path pattern, the row applies independently to each match. For rows sharing a path, the gate/order column is the only order. A failed prerequisite suppresses every dependent row. Every cell saying “actual canonical value” uses the total rule above: wrong collections become `sequence` or `mapping`, integers/booleans/null retain their exact canonical primitive, and every other scalar becomes the decoded-content JSON string. Collection-content, length, and uniqueness failures use `sequence`; absence uses `missing`. A derived-count row uses its named ungrouped decimal count. Except for the separately ordered five global totals below, a checked-arithmetic failure uses the corresponding limit plus one as the observed count and follows the same row. `2^A`, `2^n`, `r`, `s`, `n-1`, and every other dynamic substitution use already validated ungrouped decimal values.

YAML and common input emitters:

| scope/path pattern | gate/order | failure | code | message template | constraint or reason | observed value |
| --- | --- | --- | --- | --- | --- | --- |
| Mapping or Scenario source, `""` | raw-size gate, first | byte `16777217` is read | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `at most 16777216 raw bytes` | `16777217` |
| source, `""` | earliest byte, priority 1 | invalid encoding or prohibited BOM | `input.yaml_parse` | `invalid YAML syntax` | — | — |
| source, `""` | earliest byte, priority 2 | scanner/parser syntax | `input.yaml_parse` | `invalid YAML syntax` | — | — |
| source, `""` | earliest byte, priority 3 | flow-style root | `input.yaml_parse` | `invalid YAML syntax` | — | — |
| source, `""` | earliest byte, priority 4 | explicit tag | `input.yaml_parse` | `invalid YAML syntax` | — | — |
| source, `""` | earliest byte, priority 5 | anchor | `input.yaml_parse` | `invalid YAML syntax` | — | — |
| source, `""` | earliest byte, priority 6 | alias | `input.yaml_parse` | `invalid YAML syntax` | — | — |
| source, `""` | earliest byte, priority 7 | merge key | `input.yaml_parse` | `invalid YAML syntax` | — | — |
| source, `""` | earliest byte, priority 8 | non-string key | `input.yaml_parse` | `invalid YAML syntax` | — | — |
| second occurrence’s encoded full path | earliest byte, priority 9; use second occurrence’s source position | duplicate string key | `input.yaml_parse` | `duplicate key <quoted-key>` | — | quoted raw duplicate key |
| source, `""` | earliest byte, priority 10 | second or later document | `input.yaml_parse` | `expected exactly one YAML document, found <count>` | — | document count |
| source, `""` | EOF position | no document | `input.yaml_parse` | `expected exactly one YAML document, found <count>` | — | `0` |
| exact encoded full path of an unrecognized key | YAML gates, then containing mapping in source order | key is outside the allowed set | `input.unknown_field` | `unknown field <quoted-key>` | — | quoted raw key |

Mapping schema emitters:

| scope/path pattern | gate/order | failure | code | message template | constraint or reason | observed value |
| --- | --- | --- | --- | --- | --- | --- |
| `""` | Mapping schema, 1 | document root is not a mapping | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `mapping` | actual canonical value |
| `schema_version`, `name`, `address`, `targets`, `mapping`, `mapping.m`, `mapping.l`, `mapping.m.rows` | parent gate, canonical missing order | intrinsically required field absent | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `required field` | `missing` |
| `address.width_bits`, `address.granule_bytes`, `targets.count`, `mapping.l.mode` | parent gate, canonical missing order | intrinsically required field absent | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `required field` | `missing` |
| `address`, `targets`, `mapping`, `mapping.m`, `mapping.l` | after presence | value is not a mapping | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `mapping` | actual canonical value |
| `schema_version` | 1 | value is not an integer | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `integer` | actual canonical value |
| `schema_version` | 2 | value is not a plain generic-integer lexeme | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `plain integer` | actual canonical value |
| `schema_version` | 3 | value is not `1` | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `integer in [1,1]` | actual integer |
| `name` | 1 | value is not a string | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `string` | actual canonical value |
| `name` | 2 | value is empty or contains a control or line-separator character | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `non-empty string without control or line-separator characters` | actual JSON string |
| `name` | 3 | decoded name exceeds 128 UTF-8 bytes | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `UTF-8 byte length <= 128` | actual UTF-8 byte count |
| `address.width_bits` | 1 | value is not an integer | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `integer` | actual canonical value |
| `address.width_bits` | 2 | value is not a plain generic-integer lexeme | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `plain integer` | actual canonical value |
| `address.width_bits` | 3 | value is outside `1..64` | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `integer in [1,64]` | actual integer |
| `address.granule_bytes` | scalar gate 1 | value is not an integer | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `integer` | actual canonical value |
| `address.granule_bytes` | scalar gate 2 | value is not a plain generic-integer lexeme | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `plain integer` | actual canonical value |
| `address.granule_bytes` | cross-family 1 | integer is not a power of two | `mapping.unsupported` | `unsupported <field>: <reason>` | `granule size is not a power of two` | actual integer |
| `address.granule_bytes` | cross-family 2; valid `address.width_bits` required | $G>2^A$ | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `integer <= <2^A>` | actual integer |
| `address.granule_bytes` | cross-family 3 | $G>4503599627370496$ | `mapping.unsupported` | `unsupported <field>: <reason>` | `granule size exceeds v1 limit 4503599627370496` | actual integer |
| `targets.count` | scalar gate 1 | value is not an integer | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `integer` | actual canonical value |
| `targets.count` | scalar gate 2 | value is not a plain generic-integer lexeme | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `plain integer` | actual canonical value |
| `targets.count` | cross-family 1 | integer is not a power of two | `mapping.unsupported` | `unsupported <field>: <reason>` | `target count is not a power of two` | actual integer |
| `targets.count` | cross-family 2; valid $n$ required | $N>2^n$ | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `integer <= <2^n>` | actual integer |
| `targets.count` | cross-family 3 | $N>65536$ | `mapping.unsupported` | `unsupported <field>: <reason>` | `target count exceeds v1 limit 65536` | actual integer |
| `mapping.l.mode` | 1 | value is not a string | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `string` | actual canonical value |
| `mapping.l.mode` | 2 | value is not a supported mode | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `one of ["preserve_high","explicit"]` | actual JSON string |
| `mapping.l.rows` | valid mode, conditional 1 | present when mode is `preserve_high` | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `field absent when mapping.l.mode="preserve_high"` | actual canonical value |
| `mapping.l.rows` | valid mode, conditional 1 | absent when mode is `explicit` | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `field present when mapping.l.mode="explicit"` | `missing` |
| `mapping.m.rows`, `mapping.l.rows` | after presence/conditional gate | value is not a sequence | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `sequence` | actual canonical value |
| `mapping.m.rows` | valid $r$, after sequence gate | row count is not $r$ | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `sequence length <r>` | `sequence` |
| `mapping.l.rows` | valid $s$, explicit mode, after sequence gate | row count is not $s$ | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `sequence length <s>` | `sequence` |
| `mapping.m.rows[i]`, `mapping.l.rows[i]` | row source order, 1 | row is not a sequence | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `sequence` | actual canonical value |
| `mapping.m.rows[i]`, `mapping.l.rows[i]` | row source order, 2 | row repeats a tap | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `unique values` | `sequence` |
| `mapping.m.rows[i][j]`, `mapping.l.rows[i][j]` | tap source order, 1 | tap is not an integer | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `integer` | actual canonical value |
| `mapping.m.rows[i][j]`, `mapping.l.rows[i][j]` | tap source order, 2 | tap is not a plain generic-integer lexeme | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `plain integer` | actual canonical value |
| `mapping.m.rows[i][j]`, `mapping.l.rows[i][j]` | valid $n$, tap source order, 3 | tap is outside `0..n-1` | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `integer in [0,<n-1>]` | actual integer |

Scenario schema emitters:

| scope/path pattern | gate/order | failure | code | message template | constraint or reason | observed value |
| --- | --- | --- | --- | --- | --- | --- |
| `""` | Scenario schema, 1 | document root is not a mapping | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `mapping` | actual canonical value |
| `schema_version`, `defaults`, `cases`, `defaults.accesses`, `defaults.window_sizes` | parent gate, canonical missing order | intrinsically required field absent | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `required field` | `missing` |
| `cases[i].name`, `cases[i].kind` | common-field canonical missing order | intrinsically required field absent | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `required field` | `missing` |
| `cases[i].base_bytes`, `cases[i].stride_bytes` | valid `stride` or `sweep`, kind-specific missing order | intrinsically required field absent | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `required field` | `missing` |
| `cases[i].schedule`, `cases[i].streams` | valid `multi_stream`, kind-specific missing order | intrinsically required field absent | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `required field` | `missing` |
| `cases[i].streams[j].name`, `cases[i].streams[j].base_bytes`, `cases[i].streams[j].stride_bytes`, `cases[i].streams[j].accesses` | valid `multi_stream`, stream missing order | intrinsically required field absent | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `required field` | `missing` |
| `defaults`, `cases[i]`, `cases[i].streams[j]` | after presence and valid parent shape | value is not a mapping | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `mapping` | actual canonical value |
| `schema_version` | 1 | value is not an integer | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `integer` | actual canonical value |
| `schema_version` | 2 | value is not a plain generic-integer lexeme | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `plain integer` | actual canonical value |
| `schema_version` | 3 | value is not `1` | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `integer in [1,1]` | actual integer |
| `defaults.accesses`, `cases[i].accesses` for `stride`/`sweep`, `cases[i].streams[j].accesses` | 1 | value is not an integer | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `integer` | actual canonical value |
| same access paths | 2 | value is not a plain generic-integer lexeme | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `plain integer` | actual canonical value |
| same access paths | 3 | value is outside `1..10000000` | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `integer in [1,10000000]` | actual integer |
| `defaults.window_sizes`, `cases[i].window_sizes` | 1 | value is not a sequence | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `sequence` | actual canonical value |
| same window-list paths | 2 | sequence is empty | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `non-empty sequence` | `sequence` |
| same window-list paths | 3 | sequence repeats a value | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `unique values` | `sequence` |
| `defaults.window_sizes[j]`, `cases[i].window_sizes[j]` | 1 | entry is not an integer | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `integer` | actual canonical value |
| same window-entry paths | 2 | entry is not a plain generic-integer lexeme | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `plain integer` | actual canonical value |
| same window-entry paths | 3 | entry is outside `1..10000000` | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `integer in [1,10000000]` | actual integer |
| effective window-entry source path | selected case, after inheritance and valid effective $Q$ | $W>Q$ | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `integer <= <Q>` | actual $W$ |
| `cases` | 1 | value is not a sequence | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `sequence` | actual canonical value |
| `cases` | 2 | sequence is empty | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `non-empty sequence` | `sequence` |
| `cases[i]` | case source order | entry is not a mapping | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `mapping` | actual canonical value |
| `cases[i].name`, `cases[i].streams[j].name` | 1 | value is not a string | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `string` | actual canonical value |
| same name paths | 2 | value does not match the name grammar | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `string matching "[A-Za-z0-9][A-Za-z0-9._-]*"` | actual JSON string |
| same name paths | 3 | decoded name exceeds 128 UTF-8 bytes | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `UTF-8 byte length <= 128` | actual UTF-8 byte count |
| `cases` | after all valid case names | case names are not unique | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `unique values` | `sequence` |
| `cases[i].streams` | after all valid stream names | stream names are not unique | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `unique values` | `sequence` |
| `cases[i].enabled` | common field, if present | value is not a boolean | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `boolean` | actual canonical value |
| `cases[i].kind` | common field, 1 | value is not a string | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `string` | actual canonical value |
| `cases[i].kind` | common field, 2 | value is not a supported kind | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `one of ["stride","sweep","multi_stream"]` | actual JSON string |
| `cases[i].base_bytes`, `cases[i].stride_bytes` for valid `stride` | kind gate, field source order | value is not a plain address | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `plain address` | actual canonical value |
| `cases[i].base_bytes`, `cases[i].stride_bytes` for valid `sweep` | kind gate, 1 | value is not a sequence | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `sequence` | actual canonical value |
| same sweep paths | kind gate, 2 | sequence is empty | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `non-empty sequence` | `sequence` |
| same sweep paths | kind gate, 3 | sequence repeats a value | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `unique values` | `sequence` |
| `cases[i].base_bytes[j]`, `cases[i].stride_bytes[j]` for valid `sweep` | entry source order | entry is not a plain address | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `plain address` | actual canonical value |
| `cases[i].schedule` | valid `multi_stream`, 1 | value is not a string | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `string` | actual canonical value |
| `cases[i].schedule` | valid `multi_stream`, 2 | value is not `round_robin` | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `one of ["round_robin"]` | actual JSON string |
| `cases[i].streams` | valid `multi_stream`, 1 | value is not a sequence | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `sequence` | actual canonical value |
| `cases[i].streams` | valid `multi_stream`, 2 | sequence is empty | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `non-empty sequence` | `sequence` |
| `cases[i].streams[j].base_bytes`, `cases[i].streams[j].stride_bytes` | valid stream, field source order | value is not a plain address | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `plain address` | actual canonical value |
| `cases[i].accesses` | valid `multi_stream`, field source order | forbidden case-level field is present | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `field absent when cases[i].kind="multi_stream"` | actual canonical value |

Command, semantic, and resource emitters:

| scope/path pattern | gate/order | failure | code | message template | constraint or reason | observed value |
| --- | --- | --- | --- | --- | --- | --- |
| `addresses[i]` | command operand source order, 1 | operand is not a plain address | `address.invalid` | `invalid address <quoted-lexeme>` | `plain address` | quoted original lexeme |
| `addresses[i]` | command operand source order, 2; valid $A$ | address is at least $2^A$ | `address.out_of_range` | `address <canonical> is outside the <A>-bit range` | `integer <= <2^A-1>` | canonical address |
| `""` | after CLI grammar | query-address count exceeds `1000000` | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `at most 1000000 query addresses` | actual query count |
| generated address from `cases[i]`, or `cases[i].streams[j]` | after checked expansion, source test/address order | address is at least $2^A$ | `address.out_of_range` | `address <canonical> is outside the <A>-bit range` | `integer <= <2^A-1>` | canonical address |
| effective-window source path | selected case, after inheritance | effective window count exceeds `1024` | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `integer <= 1024` | actual window count |
| `cases[i].streams` | selected case with valid `multi_stream` shape | stream count exceeds `4096` | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `integer <= 4096` | actual stream count |
| `cases[i]` | selected case, after inheritance/stream sum | concrete-test $Q$ exceeds `10000000` | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `integer <= 10000000` | actual $Q$ |
| `cases` | global total 1, after all selected per-case checks; stop on failure | concrete-test count exceeds `10000` | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `integer <= 10000` | actual count, or `10001` on overflow |
| `cases` | global total 2; only if 1 passes; stop on failure | $\sum Q$ exceeds `100000000` | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `integer <= 100000000` | actual sum, or `100000001` on overflow |
| `cases` | global total 3; only if 1–2 pass; stop on failure | Target report rows exceed `1000000` | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `integer <= 1000000` | actual count, or `1000001` on overflow |
| `cases` | global total 4; only if 1–3 pass; stop on failure | window report rows exceed `1000000` | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `integer <= 1000000` | actual count, or `1000001` on overflow |
| `cases` | global total 5; only if 1–4 pass; stop on failure | $\sum(Q\cdot K_{\mathrm{effective}})>100000000$ | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `sum(Q*K) <= 100000000` | actual sum, or `100000001` on overflow |
| `""` | rendering bounded sink, after business report selection | attempted byte `268435457` | `output.too_large` | `report exceeds v1 limit 268435456 bytes` | — | `268435457` |
| `mapping.m.rows` | after mathematical checks | $\operatorname{rank}(M)<r$ | `mapping.target_unreachable` | `rank(M)=<actual>, expected <r>` | — | exact Section 6.4 check object |
| `mapping.l.rows` for explicit L; `mapping.m.rows` for preserve-high | target reachable, then mathematical checks | $\operatorname{rank}(F)<n$ | `mapping.non_bijective` | `rank(F)=<actual>, expected <n>` | — | exact Section 6.4 check object |
| `mapping.m.rows`, `mapping.l.rows`, or `mapping` as assigned in Section 6.4 | first two checks pass | natural-order predicate fails | `mapping.non_natural` | exact applicable Section 6.4 natural failure message | — | exact Section 6.4 check object |
| `""` | requested names in first CLI occurrence order | distinct requested case name absent | `scenario.case_not_found` | `case <quoted-name> was not found` | — | quoted requested name |
| `""` | after case-name lookup | final selection empty and no missing-name issue exists | `scenario.no_case_selected` | `no scenario case was selected` | — | — |

For Scenario case unknown-key detection, before a valid `kind` the allowed-name union is exactly `name`, `enabled`, `kind`, `window_sizes`, `base_bytes`, `stride_bytes`, `accesses`, `schedule`, and `streams`, and kind-dependent matrix rows are suppressed. After a valid `kind`, `stride` and `sweep` allow the four common names plus `base_bytes`, `stride_bytes`, and `accesses`; `multi_stream` allows the common names plus `schedule` and `streams`, while recognizing `accesses` only for its explicit forbidden-field emitter. Every other key uses the common `input.unknown_field` row. After all selected per-case checks, the five `cases` global-total rows emit at most one issue: evaluate them in numbered order and do not compute a later aggregate after a failure.

Malformed YAML syntax and each prohibited syntax/document form compete under Section 4.1’s earliest-byte rule and produce one `input.yaml_parse`. Duplicate-key and document-count winners use their specific templates above; other syntax/document winners use `invalid YAML syntax`. Unknown fields use `input.unknown_field` and the `unknown field` template. The G/N cross-family rows use the `unsupported` template for both non-power-of-two and v1-cap failures; every other field, range, and constraint row uses the matrix’s assigned template.

### 7.3 Atomicity

Linux `x86_64-unknown-linux-gnu` is the v1 filesystem baseline. Input and output transactions obey all of the following:

1. Use Section 4.1’s bounded reader for every regular file, standard input, FIFO, or device-like named input. Stop on byte `16 MiB + 1`; the size error wins before UTF-8/YAML and before output preflight. Retain device and inode identity for opened regular-file inputs whose accepted snapshot reached EOF.
2. `--force` requires a path-valued output; an omitted output or `-` is a usage error. Inspect the final path without following a symlink. A nonexistent path is allowed. Refuse an existing symlink or non-regular file with `output.invalid_target`/1 regardless of `--force`. Compare an existing regular output with every regular-file input by device and inode, not path spelling; the same file or a hard-link alias is permitted only with `--force`, after its bounded input snapshot reached EOF.
3. Render the complete report through Section 6.1’s `268435456`-byte counting bounded sink before creating the destination transaction. The count includes the single trailing LF. Attempted byte `268435457` produces `output.too_large`/1 on standard error, discards the partial render, and creates no destination transaction. Only after a bounded render succeeds, create a regular temporary file in the output's directory with `O_CREAT|O_EXCL`, mode `0666 & umask`; write the complete bytes, flush userspace buffers, and close it.
4. Without `--force`, commit with `renameat2(RENAME_NOREPLACE)`. If the destination now exists, report `output.exists`/1. If the syscall or filesystem cannot provide atomic no-clobber rename, report `output.atomic_unsupported`/1; no link/unlink or other weaker fallback is allowed.
5. With `--force`, recheck the final path without following a symlink. If it is still nonexistent, use atomic rename to create it; if it is an existing regular file, use atomic rename to replace it. Refuse a symlink or non-regular target found by this recheck.
6. Remove the unique temporary file on every failure before commit. Every refused or failed operation leaves the prior destination byte-for-byte untouched and leaves no temporary residue.

New files receive the temporary file's `0666 & umask` mode. Replacement does not preserve the prior destination's permissions, ownership, or other metadata. v1 promises neither an `fsync` crash-durability guarantee nor correctness under hostile concurrent mutation of the output directory.

`map` validates the Mapping and all query addresses before calculation. `run` validates the Mapping, selected scenarios, expansion resources, and every address that will be generated before analysis. Any preflight failure produces no partial address or scenario result, and no failed file transaction leaves a truncated target.

## 8. Deterministic Corner-Case Behavior

### 8.1 Mapping

| Scenario | Behavior |
| --- | --- |
| `targets.count = 1` | Valid; $r=0$, and `mapping.m.rows` must be empty |
| $N=2^n$ | Valid; $s=0$, and the LA line is always 0 |
| `granule_bytes = 1` | Valid; $g=0$, and the byte offset is always 0 |
| `granule_bytes > 2^A` | Mapping input error |
| `granule_bytes > 2^52` while still satisfying the mathematical relation | `mapping.unsupported`, exit code 2 |
| Target count or granule is not a power of two | Report `mapping.unsupported` and exit with code 2 |
| `targets.count > 65,536` while otherwise meaningful | `mapping.unsupported`, exit code 2 |
| Target count exceeds the number of line-address combinations | Mapping input error |
| A tap is negative or `tap >= n` | Mapping input error with the exact field path |
| A row contains a duplicate tap | Mapping input error; duplicates are not canceled as XOR |
| `M` or explicit `L` has the wrong number of rows | Mapping input error |
| `preserve_high` also supplies rows | Mapping input error |
| `explicit` does not supply rows | Mapping input error |
| Matrices are structurally valid, but a Target is unreachable | `invalid_target_unreachable` |
| Targets are reachable, but $F$ is not full rank | `invalid_non_bijective` |
| $F$ is full rank, but the LA is not naturally ordered | `valid_non_natural` warning, exit code 0; `map` and `run` are allowed |

### 8.2 Addresses and Numeric Values

| Scenario | Behavior |
| --- | --- |
| Address is exactly $2^A-1$ | Valid |
| Address is equal to or greater than $2^A$ | Out-of-range error |
| Address is negative | Input error |
| Address text has a sign, leading decimal zero, or invalid underscore placement | `address.invalid`; no partial query result |
| A 64-bit address calculation produces an intermediate carry | Check using mathematical integers before narrowing; do not truncate or wrap |
| Base or stride is not aligned to the granule | Valid; calculate the byte offset from the actual address |
| `stride_bytes = 0` | Valid; repeatedly accesses the same address |

### 8.3 Scenario

| Scenario | Behavior |
| --- | --- |
| `accesses = 0` | Input error |
| A window is 0, duplicated, or the list is empty | Input error |
| A window exceeds $Q$ for the expanded scenario | Input error; do not skip that window |
| Duplicate case names | Input error |
| Duplicate stream names | Input error |
| Empty `sweep` base or stride list | Input error |
| Duplicate values in a `sweep` base or stride list | Input error |
| A multi-stream case has only one stream | Valid; equivalent to that stream's own order |
| Streams in a multi-stream case have different lengths | Valid; `round_robin` skips streams after they end |
| Every case is disabled and no `--case` is specified | Case-selection error, exit code 3 |
| A disabled case is selected explicitly | Run that case |
| The same `--case` appears repeatedly | Run it only once |
| Any generated address is out of range | Fail the entire `run` command; do not truncate, wrap, or emit partial results |
| Any per-case or run-wide resource cap in Section 3.7 is exceeded | `scenario.invalid`, exit code 3, before analysis |
| $Q=10,000,000$ and $K_{\mathrm{effective}}=1,024$ in one concrete test | Reject because $QK=10,240,000,000>100,000,000$ |

### 8.4 Metrics

| Scenario | Behavior |
| --- | --- |
| $Q$ is not divisible by $N$ | The ideal load still uses the real-valued $Q/N$ |
| $W<N$ | The ideal window load still uses the real-valued $W/N$ |
| Multiple Targets tie for the largest long-term load | Choose the smallest Target ID as the representative |
| Multiple worst windows tie | Choose the smallest start index, then the smallest Target ID |
| Multiple longest runs tie | Choose the smallest start index |
| A Target is never accessed | Still include it in output with a count of 0 |

## 9. Acceptance Criteria

### 9.1 Mathematical Correctness

- for small address spaces that can be exhaustively enumerated, tool results match direct per-address enumeration;
- `rank(M)=r` agrees with exhaustive reachability of every Target;
- `rank(F)=n` agrees with the absence of both duplicates and holes among `(Target, LA line)` pairs;
- when `rank(M_p)=r` and $L=[0\ I]$, fixing any Target yields LA lines in the exact order `0,1,2,...`;
- when a Mapping is bijective but its LA bits are permuted, it is classified as `valid_non_natural`, and `validate`, `map`, and `run` all retain the warning;
- the byte offset is preserved unchanged by every Mapping.

### 9.2 Input and Commands

- both template types produce output that the corresponding command can read directly;
- unknown fields, duplicate fields, and incorrect types in Mapping and Scenario files are never silently ignored;
- the options, selection order, and exit codes of `validate`, `map`, and `run` conform to Chapters 5 and 7;
- all corner cases behave as defined in Chapter 8;
- no failure leaves partial queries, partial scenarios, or a truncated output file.

### 9.3 Performance Metrics

- Target counts always sum to $Q$;
- $R_{\max}$, every $R_{\mathrm{window}}$, and $L_{\mathrm{run}}$ agree with their defining formulas;
- sequences with identical long-term counts but different access order can produce the same $R_{\max}$ and different $R_{\mathrm{window}}$;
- aligned and staggered multi-stream accesses produce distinct results that are deterministic and explainable;
- `sweep` combination order, case IDs, and independent metrics for each combination conform to this specification.

### 9.4 Output

- a reader can determine whether a Mapping is valid from the text report without reading source code;
- every warning and error explains its effect on the actual Mapping or its performance;
- text and JSON express the same conclusions;
- JSON fields, ordering rules, address encoding, and ratio structures conform to Chapter 6;
- identical input produces identical ordering and identical results.

## 10. Capabilities Outside the Current Scope

The following capabilities are outside the scope of v1:

- a Target count that is not a power of two;
- Targets with unequal capacities;
- holes in the address space;
- TOML or JSON configuration input;
- import of real hardware traces;
- multi-stream scheduling policies other than `round_robin`;
- aggregate metrics across `sweep` combinations.

These capabilities may be added in the future, but they must not affect the accuracy, determinism, or explainability of results within the current scope.
