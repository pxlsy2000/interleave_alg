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

Formulas use GitHub-compatible LaTeX syntax: inline formulas use `$...$`, and display formulas use `$$...$$`. Even when a reader does not render LaTeX, the adjacent symbol table and prose should make each formula understandable. See the [official GitHub documentation](https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/writing-mathematical-expressions) for syntax details.

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

## 4. User Input Format

### 4.1 Format Selection

v1 accepts YAML 1.2 configuration files only:

- a Mapping uses one YAML file;
- a Scenario uses a separate YAML file;
- JSON is used only for structured output;
- TOML and JSON input are outside the scope of v1.

YAML was chosen because XOR tap lists, scenario lists, and comments are easier for people to write and review in YAML.

All YAML files must meet the following requirements:

- UTF-8 encoding;
- exactly one YAML document;
- case-sensitive keys;
- undefined keys are rejected so that misspellings cannot be silently ignored;
- duplicate keys are rejected;
- YAML anchors, aliases, and merge keys are outside the scope of v1;
- `schema_version` must be the integer `1`.

Numeric address values may be written as non-negative decimal integers or as hexadecimal integers prefixed with `0x`. Templates generated by the tool use lowercase hexadecimal.

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
| `name` | string | Yes | Non-empty, human-readable Mapping name |
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

This restriction allows a name to be passed directly to `--case` and prevents collisions with automatically generated `sweep` combination IDs.

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
| `streams[].name` | string | Yes | Unique within the case and subject to the same character rules as a case name |
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
- `--version` displays the tool version and exits with code 0;
- an input path of `-` reads from standard input;
- an output path of `-` writes to standard output;
- output is written to standard output when `--output` is omitted;
- `text` is used when `--format` is omitted;
- `--format` accepts only `text` or `json`;
- an existing output file is not overwritten by default; `--force` is required to overwrite it;
- `--force` is permitted only when `--output` refers to a regular file;
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

`path` uses a YAML field path. When an issue does not correspond to an input field, it is an empty string.

JSON object key order is not part of the contract. Array order explicitly defined by this specification is part of the contract.

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

`numerator/denominator` is authoritative and is not reduced. `decimal` is rounded to 6 digits after the decimal point.

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
| `analysis.failed` | error | Input was valid, but analysis could not be completed |

An implementation may add more specific issue codes, but it must not change the meaning of the codes above.

### 7.3 Atomicity

- `map` validates the Mapping and every query address before any calculation;
- `run` validates the Mapping, every selected scenario, and every address that will be generated before analysis;
- if any preflight check fails, no partial address results or scenario results are produced;
- when writing to a file, the target is replaced only after the complete report has been generated successfully;
- a failure must not leave behind a truncated target file.

## 8. Deterministic Corner-Case Behavior

### 8.1 Mapping

| Scenario | Behavior |
| --- | --- |
| `targets.count = 1` | Valid; $r=0$, and `mapping.m.rows` must be empty |
| $N=2^n$ | Valid; $s=0$, and the LA line is always 0 |
| `granule_bytes = 1` | Valid; $g=0$, and the byte offset is always 0 |
| `granule_bytes > 2^A` | Mapping input error |
| Target count or granule is not a power of two | Report `unsupported` and exit with code 2 |
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
