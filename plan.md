# Hash Interleave 分析工具产品规格

## 1. 产品目标

Hash Interleave 分析工具用于回答两个问题：

1. 一套地址映射是否满足预期的数学性质；
2. 这套映射在典型访问模式下是否足够均衡。

工具面向设计、验证和性能分析人员。使用者通过可读的配置文件描述地址映射和访问场景，工具给出可解释、可复查的结论。

本规格定义：

- 地址映射的数学含义和正确性标准；
- 使用者提供的文件格式；
- 命令行接口；
- 性能指标及其计算公式；
- 人工报告和结构化报告的输出契约；
- 各类边界情况的确定行为。

本规格不规定编程语言、依赖库、内部数据结构或工程组织方式。

## 2. 数学模型与正确性标准

本章是 Mapping 正确性的唯一正式定义。后续输入、输出和验收章节均引用这里的定义，不再建立另一套验证规则。

公式使用 GitHub 兼容的 LaTeX 语法：行内公式使用 `$...$`，独立公式使用 `$$...$$`。即使阅读器不渲染 LaTeX，也可以结合紧邻公式的符号表和文字说明理解其含义。语法依据见 [GitHub 官方文档](https://docs.github.com/zh/get-started/writing-on-github/working-with-advanced-formatting/writing-mathematical-expressions)。

### 2.1 符号

| 符号 | 含义 |
| --- | --- |
| $A$ | byte address 的总位宽 |
| $G$ | 访问粒度，单位为 byte |
| $g$ | 粒度内部 byte offset 的位数 |
| $n$ | 参与 Mapping 的 line-address 位数 |
| $N$ | Target 数量 |
| $r$ | Target ID 的位数 |
| $s$ | 每个 Target 内 LA line 的位数 |
| $a$ | 输入 byte address |
| $o$ | 粒度内部的 byte offset |
| $x$ | line address 的 bit 向量 |
| $M$ | 从 line address 生成 Target ID 的矩阵 |
| $L$ | 从 line address 生成 LA line 的矩阵 |

当前范围要求 $G$ 和 $N$ 都是 2 的幂，因此：

$$
g = \log_2 G,\qquad
n = A-g,\qquad
r = \log_2 N,\qquad
s = n-r
$$

必须满足：

$$
1 \le A \le 64,\qquad
1 \le G \le 2^A,\qquad
1 \le N \le 2^n
$$

### 2.2 地址拆分

对于有效地址 $0 \le a < 2^A$：

$$
o = a \bmod G,\qquad
q = \left\lfloor\frac{a}{G}\right\rfloor
$$

将 $q$ 写成 LSB-first 的 bit 向量：

$$
x =
\begin{bmatrix}
x_0 & x_1 & \cdots & x_{n-1}
\end{bmatrix}^{\mathsf T}
$$

其中 $x_0$ 是 line address 的最低位。

Mapping 只作用于 $x$。byte offset $o$ 不参与 Target 或 LA line 的计算，并在最终 LA byte address 中原样保留。

### 2.3 Mapping

所有矩阵运算均在 GF(2) 上进行，加法等价于 XOR。

$$
t = Mx,\qquad
\ell = Lx
$$

其中：

- $M$ 的尺寸为 $r \times n$；
- $L$ 的尺寸为 $s \times n$；
- $t$ 是 Target ID 的 LSB-first bit 向量；
- $\ell$ 是 LA line 的 LSB-first bit 向量。

bit 向量到整数的转换为：

$$
\operatorname{Target}(a)
= \sum_{i=0}^{r-1} t_i 2^i
$$

$$
\operatorname{LA\_line}(a)
= \sum_{i=0}^{s-1} \ell_i 2^i
$$

最终的 byte address 为：

$$
\operatorname{LA\_byte}(a)
= G \cdot \operatorname{LA\_line}(a) + o
$$

### 2.4 正确性的三个层次

三个层次必须分别检查并分别报告。

#### 2.4.1 Target 可达

每个 Target 都必须存在至少一个输入地址能够访问到。

判定条件为：

$$
\operatorname{rank}_{GF(2)}(M)=r
$$

如果秩小于 $r$，某些 Target 永远无法被选中，Mapping 失败。

#### 2.4.2 Mapping 双射

定义组合矩阵：

$$
F =
\begin{bmatrix}
M \\
L
\end{bmatrix}
$$

因为 $r+s=n$，所以 $F$ 是 $n \times n$ 方阵。

Mapping 为双射的充要条件是：

$$
\operatorname{rank}_{GF(2)}(F)=n
$$

该条件保证每个输入 line address 都对应唯一的 `(Target, LA line)`，且有效输出空间中既没有碰撞，也没有空洞。

#### 2.4.3 每个 Target 内的 LA 自然有序

将 line address 按低位和高位拆分：

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

其中 $p$ 有 $r$ 位，$u$ 有 $s$ 位，并按列顺序写成：

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

“LA 自然有序”在本规格中有严格含义：固定任意 Target 后，LA line 必须等于输入 line address 的高 $s$ 位 $u$，并按 `0, 1, 2, ...` 的自然二进制顺序完整排列。

因此必须同时满足：

$$
\operatorname{rank}_{GF(2)}(M_p)=r
$$

以及：

$$
L =
\begin{bmatrix}
0_{s\times r} & I_s
\end{bmatrix}
$$

第一项保证固定 Target 和任意 $u$ 时，都存在唯一的低位 $p$。第二项保证 LA line 正好等于 $u$，没有 bit 置换、XOR 重排或依赖 Target 的偏移。

即使整体 Mapping 是双射，只要上述任一条件不成立，就必须报告“Mapping 有效，但 LA 不满足自然顺序”，不能将其误报为双射失败。

当 $r=0$ 时，$M_p$ 是空矩阵，其秩定义为 0；当 $s=0$ 时，$I_s$ 是空单位矩阵。这两个退化情况仍按同一公式处理。

### 2.5 最终分类

| 分类 | 条件 | 命令结果 |
| --- | --- | --- |
| `valid_natural` | Target 可达、Mapping 双射、LA 自然有序 | 成功 |
| `valid_non_natural` | Target 可达、Mapping 双射、LA 不自然 | 成功并给出 warning |
| `invalid_target_unreachable` | $\operatorname{rank}(M)<r$ | 失败 |
| `invalid_non_bijective` | $\operatorname{rank}(M)=r$，但 $\operatorname{rank}(F)<n$ | 失败 |

如果输入结构本身无效，例如矩阵尺寸错误，则不进行最终分类，而是报告具体输入错误。

## 3. 用户输入格式

### 3.1 格式选择

v1 只接受 YAML 1.2 配置文件：

- Mapping 使用一个 YAML 文件；
- Scenario 使用另一个 YAML 文件；
- JSON 只用于结构化输出；
- TOML 和 JSON 输入不属于 v1。

选择 YAML 是因为 XOR tap 列表、场景列表和注释在 YAML 中更便于人工编写和评审。

所有 YAML 文件必须满足：

- UTF-8 编码；
- 只包含一个 YAML document；
- key 区分大小写；
- 未定义的 key 直接报错，避免拼写错误被静默忽略；
- 重复 key 直接报错；
- YAML anchor、alias 和 merge key 不属于 v1；
- `schema_version` 必须为整数 `1`。

地址类数值可以写成非负十进制整数或 `0x` 开头的十六进制整数。工具生成的模板统一使用小写十六进制。

### 3.2 Mapping 文件

#### 3.2.1 完整示例

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

#### 3.2.2 字段定义

| 路径 | 类型 | 必填 | 含义 |
| --- | --- | --- | --- |
| `schema_version` | integer | 是 | 固定为 `1` |
| `name` | string | 是 | 非空的人类可读 Mapping 名称 |
| `address.width_bits` | integer | 是 | $A$，范围 `1..64` |
| `address.granule_bytes` | integer | 是 | $G$，必须为 2 的幂 |
| `targets.count` | integer | 是 | $N$，必须为 2 的幂 |
| `mapping.m.rows` | array of integer arrays | 是 | Target 各输出 bit 的 XOR tap |
| `mapping.l.mode` | string | 是 | `preserve_high` 或 `explicit` |
| `mapping.l.rows` | array of integer arrays | 见下文 | explicit LA 各输出 bit 的 XOR tap |

`mapping.m.rows[i]` 生成 Target bit $t_i$。例如 `[0, 4, 8]` 表示：

$$
t_i=x_0\oplus x_4\oplus x_8
$$

每个 tap 是输入向量 $x$ 的 bit 编号，必须满足 `0 <= tap < n`。

同理，`mapping.l.rows[i]` 生成 LA line bit $\ell_i$。两类 rows 都按输出 bit 从低到高排列。

行数要求：

- `mapping.m.rows` 必须恰好有 $r$ 行；
- `mapping.l.rows` 在 explicit 模式下必须恰好有 $s$ 行；
- 每行内不允许重复 tap；
- tap 顺序不影响语义；
- 空行表示常量 0，语法上允许，但可能导致秩检查失败。

#### 3.2.3 LA 模式

`preserve_high` 表示：

$$
L =
\begin{bmatrix}
0_{s\times r} & I_s
\end{bmatrix}
$$

该模式下不允许同时出现 `mapping.l.rows`。

`explicit` 表示使用者明确给出 $L$。例如：

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

explicit 模式必须出现 `mapping.l.rows`。如果 explicit 矩阵恰好等于 preserve-high 矩阵，它仍被判定为 LA 自然有序。

### 3.3 Scenario 文件

#### 3.3.1 完整示例

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

#### 3.3.2 公共字段

| 路径 | 类型 | 必填 | 含义 |
| --- | --- | --- | --- |
| `schema_version` | integer | 是 | 固定为 `1` |
| `defaults.accesses` | integer | 是 | stride 和 sweep 的默认访问次数，必须大于 0 |
| `defaults.window_sizes` | integer array | 是 | 默认窗口列表 |
| `cases` | case array | 是 | 至少包含一个场景 |
| `cases[].name` | string | 是 | 在文件内唯一，格式见下文 |
| `cases[].enabled` | boolean | 否 | 默认 `true` |
| `cases[].kind` | string | 是 | `stride`、`sweep` 或 `multi_stream` |
| `cases[].window_sizes` | integer array | 否 | 覆盖默认窗口 |

窗口列表必须非空、元素唯一且均大于 0。

stride 和 sweep 的有效访问次数为 case 自身的 `accesses`，若省略则使用 `defaults.accesses`。所有 case 的有效窗口列表为 case 自身的 `window_sizes`，若省略则使用 `defaults.window_sizes`。窗口合法性针对继承完成后的最终列表检查。

case 名称必须匹配：

```text
[A-Za-z0-9][A-Za-z0-9._-]*
```

该限制保证名称可以直接用于 `--case`，并且不会与 sweep 自动生成的组合 ID 冲突。

#### 3.3.3 stride

字段：

| 字段 | 类型 | 必填 | 含义 |
| --- | --- | --- | --- |
| `base_bytes` | address scalar | 是 | 首个 byte address |
| `stride_bytes` | address scalar | 是 | 相邻访问的 byte 间隔，允许为 0 |
| `accesses` | integer | 否 | 访问次数；省略时继承 `defaults.accesses` |

设访问次数为 $Q$，生成序列：

$$
a_i=\operatorname{base}+i\cdot\operatorname{stride},
\qquad 0\le i<Q
$$

未对齐到 $G$ 的 base 或 stride 是合法的；Mapping 对每个生成的 byte address 独立计算，并保留其 byte offset。

#### 3.3.4 sweep

字段：

| 字段 | 类型 | 必填 | 含义 |
| --- | --- | --- | --- |
| `base_bytes` | address scalar array | 是 | 非空且无重复的 base 列表 |
| `stride_bytes` | address scalar array | 是 | 非空且无重复的 stride 列表 |
| `accesses` | integer | 否 | 每个组合的访问次数；省略时继承默认值 |

sweep 对 base 与 stride 做笛卡尔积。顺序固定为：

1. 按 `base_bytes` 的声明顺序取 base；
2. 对每个 base，按 `stride_bytes` 的声明顺序取 stride。

每个组合独立计算一组指标，不把不同组合拼接或聚合。

输出中的组合 ID 固定为：

```text
<case-name>[base=<canonical-hex>,stride=<canonical-hex>]
```

例如：

```text
stride-and-phase-sweep[base=0x40,stride=0x100]
```

#### 3.3.5 multi_stream

字段：

| 字段 | 类型 | 必填 | 含义 |
| --- | --- | --- | --- |
| `schedule` | string | 是 | v1 固定为 `round_robin` |
| `streams` | stream array | 是 | 至少包含一个 stream |
| `streams[].name` | string | 是 | 在 case 内唯一，且遵循 case 名称的字符规则 |
| `streams[].base_bytes` | address scalar | 是 | stream 首个 byte address |
| `streams[].stride_bytes` | address scalar | 是 | stream 步长，允许为 0 |
| `streams[].accesses` | integer | 是 | stream 访问次数，必须大于 0 |

每个 stream 先按 stride 公式定义自己的地址序列。

`round_robin` 的合并规则为：

1. 按 `streams` 的声明顺序，每个尚未结束的 stream 取一个地址；
2. 已结束的 stream 被跳过；
3. 重复以上步骤，直到所有 stream 结束。

因此 multi-stream 场景的总访问数为：

$$
Q=\sum_h Q_h
$$

其中 $Q_h$ 是第 $h$ 个 stream 的访问次数。

multi-stream case 不接受 case 级 `accesses` 字段。

## 4. 命令行接口

二进制命令名固定为 `interleave`。

### 4.1 通用约定

- `--help` 显示帮助并以退出码 0 结束；
- `--version` 显示工具版本并以退出码 0 结束；
- 输入路径 `-` 表示从标准输入读取；
- 输出路径 `-` 表示写入标准输出；
- 未指定 `--output` 时写入标准输出；
- 未指定 `--format` 时使用 `text`；
- `--format` 只接受 `text` 或 `json`；
- 输出文件已存在时默认拒绝覆盖，使用 `--force` 才允许覆盖；
- `--force` 只有在 `--output` 指向普通文件时才允许使用；
- 同一条命令最多只能有一个输入文件来自标准输入。

### 4.2 生成模板

```text
interleave template mapping  --output <FILE> [--force]
interleave template scenario --output <FILE> [--force]
```

行为：

- 生成带注释的 YAML；
- 生成结果必须能直接被对应命令读取；
- template 命令不支持 `--format`；
- `--output` 必填。

### 4.3 验证 Mapping

```text
interleave validate
  --spec <MAPPING_YAML>
  [--format text|json]
  [--output <FILE|->]
  [--force]
  [--verbose]
```

行为：

- 检查输入结构和第 2 章的三个正确性层次；
- `--verbose` 在 text 输出中附加完整的 $M$、$L$、$F$ 和 $M_p$ 0/1 矩阵；
- `--verbose` 与 `--format json` 不能同时使用。

### 4.4 查询地址

```text
interleave map
  --spec <MAPPING_YAML>
  <ADDRESS>...
  [--format text|json]
  [--output <FILE|->]
  [--force]
```

行为：

- 至少提供一个地址；
- 地址接受非负十进制或 `0x` 十六进制；
- 保持命令行中的地址顺序；
- 在查询前验证 Mapping；
- `valid_natural` 和 `valid_non_natural` 可以查询；
- 无效 Mapping 或任一地址越界时不输出部分查询结果。

### 4.5 运行场景

```text
interleave run
  --spec <MAPPING_YAML>
  --scenario <SCENARIO_YAML>
  [--case <NAME>]...
  [--format text|json]
  [--output <FILE|->]
  [--force]
```

行为：

- 未指定 `--case` 时，运行所有 `enabled: true` 的 case；
- 指定 `--case` 时，精确匹配 case 名称，并忽略其 `enabled` 值；
- `--case` 可以重复，用于选择多个 case；
- 同一个名称被重复选择时只运行一次；
- 最终运行顺序始终按 Scenario 文件中的声明顺序；
- 找不到指定名称或最终没有选中任何 case 时失败；
- `valid_non_natural` Mapping 可以运行，但报告必须保留 warning；
- 运行前验证所有选中场景；任一场景无效时不产生部分分析结果。

## 5. 性能指标

对一个已展开的场景，设：

- $Q>0$ 为总访问数；
- $N$ 为 Target 数量；
- $y_i\in\{0,\ldots,N-1\}$ 为第 $i$ 次访问对应的 Target。

### 5.1 各 Target 访问量

Target $j$ 的访问次数：

$$
C_j =
\sum_{i=0}^{Q-1}
\mathbf{1}[y_i=j]
$$

访问占比：

$$
S_j=\frac{C_j}{Q}
$$

必须满足：

$$
\sum_{j=0}^{N-1}C_j=Q
$$

### 5.2 最大负载率

理想的每 Target 平均访问数为 $Q/N$。

$$
R_{\max}
=
\max_{0\le j<N}
\frac{C_j}{Q/N}
=
\frac{N\cdot\max_j C_j}{Q}
$$

$R_{\max}=1$ 表示长期分布完全均衡。数值越大，表示最繁忙 Target 偏离理想平均值越多。

如果多个 Target 的 $C_j$ 同为最大值，报告选择最小 Target ID 作为代表；$R_{\max}$ 的数值不受该选择影响。

### 5.3 短时拥塞

对于窗口大小 $W$，必须满足 $1\le W\le Q$。

从起点 $k$ 开始的窗口中，Target $j$ 的访问次数为：

$$
C_{j,k}^{(W)}
=
\sum_{i=k}^{k+W-1}
\mathbf{1}[y_i=j],
\qquad
0\le k\le Q-W
$$

窗口 $W$ 的最差负载率为：

$$
R_{\mathrm{window}}(W)
=
\max_{\substack{0\le k\le Q-W\\0\le j<N}}
\frac{C_{j,k}^{(W)}}{W/N}
=
\frac{N}{W}
\max_{k,j} C_{j,k}^{(W)}
$$

即使 $W<N$，仍使用实数理想值 $W/N$，不做取整。

如果多个 `(k, j)` 同为最差值，按以下顺序选择报告代表：

1. 最小窗口起点 $k$；
2. 在同一起点下选择最小 Target ID $j$。

### 5.4 最长连续访问

最长连续同 Target 访问定义为：

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

报告同时给出：

- 长度 $L_{\mathrm{run}}$；
- Target ID；
- 起始访问下标。

如果有多个同长度的最长 run，选择起始下标最小的一个。

### 5.5 sweep 的指标边界

sweep 的每个 `(base, stride)` 组合都有独立的 $Q$、$C_j$、$R_{\max}$、$R_{\mathrm{window}}$ 和 $L_{\mathrm{run}}$。

v1 不定义跨组合聚合指标。

## 6. 输出格式

### 6.1 输出通道

使用 `text` 时：

- 成功报告写入标准输出或 `--output` 指定文件；
- 输入错误和失败原因写入标准错误；
- text 只面向人阅读，不作为机器解析的稳定接口。

使用 `json` 时：

- 无论成功或业务失败，都输出一个完整 JSON document；
- JSON 写入标准输出或 `--output` 指定文件；
- 不在 JSON 前后混入普通文本；
- 只有命令行语法错误、输入输出文件无法访问或 JSON 本身无法生成时才写标准错误。

### 6.2 text 报告

#### validate

必须按以下顺序展示：

1. Mapping 名称和输入文件；
2. 派生参数 $A,G,g,n,N,r,s$；
3. 输入结构检查；
4. `rank(M)` 与 Target 可达结论；
5. `rank(F)` 与双射结论；
6. `rank(M_p)`、$L$ 比较结果与 LA 自然顺序结论；
7. 最终分类和 warning/error。

示例：

```text
Mapping: example-4-target
Address: 20 bits, granule 64 bytes
Derived: n=14, targets=4, r=2, s=12

PASS  target reachable: rank(M)=2, expected 2
PASS  bijective: rank(F)=14, expected 14
PASS  natural LA: rank(Mp)=2 and L=[0 I]

Result: valid_natural
```

#### map

每个输入地址一行，至少包含：

```text
Address  Line address  Byte offset  Target  LA line  LA byte
```

所有地址默认以小写十六进制展示。

#### run

每个展开后的场景单独成节，至少包含：

- case ID；
- 总访问数；
- 每个 Target 的 count 和 share；
- $R_{\max}$；
- 每个窗口的 $R_{\mathrm{window}}$、Target、起点和 count；
- 最长 run 的长度、Target 和起点；
- Mapping 或场景 warning。

所有比率在 text 中保留 6 位小数。

### 6.3 JSON 公共封装

JSON 顶层结构固定为：

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

字段定义：

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `schema_version` | integer | 固定为 `1` |
| `command` | string | `validate`、`map` 或 `run` |
| `status` | string | `pass`、`warning` 或 `fail` |
| `warnings` | issue array | warning 列表 |
| `errors` | issue array | error 列表 |
| `result` | object or null | 命令结果；无法产生结果时为 `null` |

顶层状态满足以下不变量：

- `pass`：`warnings` 和 `errors` 都为空；
- `warning`：`warnings` 非空且 `errors` 为空；
- `fail`：`errors` 非空；
- YAML 解析或结构检查失败时 `result` 为 `null`；
- validate 的数学检查失败时，`result` 仍包含已经完成的 checks 和失败分类；
- map 或 run 的预检失败时 `result` 为 `null`。

issue 结构固定为：

```json
{
  "code": "mapping.non_bijective",
  "path": "mapping.l.rows",
  "message": "rank(F)=13, expected 14"
}
```

`path` 使用 YAML 字段路径；问题不对应输入字段时使用空字符串。

JSON object 的 key 顺序不属于契约；本规格明确规定的 array 顺序属于契约。

所有地址在 JSON 中使用 canonical hex string：

- 小写 `0x` 前缀；
- 十六进制数字使用小写；
- 除 `0x0` 外没有前导零。

所有比例使用精确分数和展示用十进制：

```json
{
  "numerator": 4,
  "denominator": 4,
  "decimal": "1.000000"
}
```

`numerator/denominator` 是权威值且不约分；`decimal` 使用四舍五入保留 6 位小数。

### 6.4 validate JSON result

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

`checks` 的顺序固定为示例中的三个检查。输入结构错误放入顶层 `errors`，不伪装成数学检查。

### 6.5 map JSON result

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

`addresses` 顺序与命令行输入顺序一致。

### 6.6 run JSON result

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

`targets` 必须包含从 0 到 $N-1$ 的所有 Target，并按 ID 升序排列，包括 count 为 0 的 Target。

`windows` 按 Scenario 中 `window_sizes` 的声明顺序排列。

sweep 的 `case_id` 使用第 3.3.4 节定义的组合 ID，`source_case` 保留原始 case 名称。

run JSON 中各比例的精确字段固定为：

- Target share：`numerator = C_j`，`denominator = Q`；
- max-load ratio：`numerator = N * max(C_j)`，`denominator = Q`；
- window ratio：`numerator = N * C_{j,k}^{(W)}`，`denominator = W`。

## 7. 错误、退出码与原子性

### 7.1 退出码

| 退出码 | 含义 |
| --- | --- |
| `0` | 命令成功；warning 不改变退出码 |
| `1` | 命令行用法错误、输入输出文件不可访问或拒绝覆盖文件 |
| `2` | Mapping YAML 无法解析、结构无效、暂不支持或数学验证失败 |
| `3` | Scenario YAML 无法解析、结构无效、case 选择错误或地址越界 |
| `4` | 输入有效，但分析过程未能完成 |

`map` 中的查询地址无效或越界也使用退出码 `3`。

### 7.2 稳定问题码

`warnings[].code` 和 `errors[].code` 至少定义以下稳定值：

| code | 级别 | 含义 |
| --- | --- | --- |
| `input.yaml_parse` | error | YAML 语法、重复 key 或文档数量错误 |
| `input.unknown_field` | error | 出现未定义字段 |
| `input.invalid_value` | error | 字段类型、范围或组合无效 |
| `mapping.unsupported` | error | Mapping 合理但不属于 v1 范围 |
| `mapping.target_unreachable` | error | $\operatorname{rank}(M)<r$ |
| `mapping.non_bijective` | error | $\operatorname{rank}(F)<n$ |
| `mapping.non_natural` | warning | Mapping 双射，但 LA 不自然 |
| `scenario.invalid` | error | Scenario 字段或继承结果无效 |
| `scenario.case_not_found` | error | `--case` 未匹配到名称 |
| `scenario.no_case_selected` | error | 最终没有可运行 case |
| `address.invalid` | error | 地址文本不是允许的非负整数 |
| `address.out_of_range` | error | 查询或生成地址超出 $A$ 位范围 |
| `output.exists` | error | 输出文件已存在且未指定 `--force` |
| `analysis.failed` | error | 输入有效但分析未能完成 |

实现可以增加更具体的问题码，但不能改变上述 code 的含义。

### 7.3 原子性

- `map` 在计算前验证 Mapping 和全部查询地址；
- `run` 在分析前验证 Mapping、全部选中场景及所有将生成的地址；
- 任一预检失败时，不输出部分地址结果或部分场景结果；
- 输出到文件时，只有完整报告生成成功后才替换目标文件；
- 失败不能留下截断的目标文件。

## 8. Corner case 的确定行为

### 8.1 Mapping

| 场景 | 行为 |
| --- | --- |
| `targets.count = 1` | 合法；$r=0$，`mapping.m.rows` 必须为空 |
| $N=2^n$ | 合法；$s=0$，LA line 恒为 0 |
| `granule_bytes = 1` | 合法；$g=0$，byte offset 恒为 0 |
| `granule_bytes > 2^A` | Mapping 输入错误 |
| Target 数量、粒度不是 2 的幂 | 报告 `unsupported`，退出码 2 |
| Target 数量超过 line-address 组合数 | Mapping 输入错误 |
| tap 为负数或 `tap >= n` | Mapping 输入错误，并给出准确字段路径 |
| 同一行出现重复 tap | Mapping 输入错误，不按 XOR 抵消处理 |
| `M` 或 explicit `L` 行数错误 | Mapping 输入错误 |
| `preserve_high` 同时提供 rows | Mapping 输入错误 |
| `explicit` 未提供 rows | Mapping 输入错误 |
| 矩阵合法但 Target 不可达 | `invalid_target_unreachable` |
| Target 可达但 $F$ 不满秩 | `invalid_non_bijective` |
| $F$ 满秩但 LA 不自然 | `valid_non_natural` warning，允许 map 和 run |

### 8.2 地址和数值

| 场景 | 行为 |
| --- | --- |
| 地址恰好为 $2^A-1$ | 合法 |
| 地址等于或大于 $2^A$ | 越界错误 |
| 负地址 | 输入错误 |
| 64-bit 地址计算产生中间进位 | 先按数学整数检查，不能截断或回绕 |
| base 或 stride 未按粒度对齐 | 合法，byte offset 按实际地址计算 |
| `stride_bytes = 0` | 合法，表示重复访问同一地址 |

### 8.3 Scenario

| 场景 | 行为 |
| --- | --- |
| `accesses = 0` | 输入错误 |
| window 为 0、重复或列表为空 | 输入错误 |
| window 大于该展开场景的 $Q$ | 输入错误，不跳过该窗口 |
| case 名称重复 | 输入错误 |
| stream 名称重复 | 输入错误 |
| sweep base 或 stride 列表为空 | 输入错误 |
| sweep base 或 stride 有重复值 | 输入错误 |
| multi-stream 只有一个 stream | 合法，等价于单 stream 顺序 |
| multi-stream 各 stream 长度不同 | 合法；结束的 stream 被 round-robin 跳过 |
| 所有 case 均 disabled 且未指定 `--case` | case 选择错误，退出码 3 |
| 显式选择 disabled case | 运行该 case |
| 同一 `--case` 重复出现 | 只运行一次 |
| 任一生成地址越界 | 整条 run 命令失败，不截断、不回绕、不输出部分结果 |

### 8.4 指标

| 场景 | 行为 |
| --- | --- |
| $Q$ 不能被 $N$ 整除 | 理想负载仍使用实数 $Q/N$ |
| $W<N$ | 理想窗口负载仍使用实数 $W/N$ |
| 多个 Target 并列最大长期负载 | 选择最小 Target ID 作为代表 |
| 多个最差窗口并列 | 先选最小起点，再选最小 Target ID |
| 多个最长 run 并列 | 选择最小起点 |
| 某 Target 从未访问 | 仍在输出中列出，count 为 0 |

## 9. 验收标准

### 9.1 数学正确性

- 对可穷举的小地址空间，工具结果与逐地址枚举结果一致；
- `rank(M)=r` 与“所有 Target 可达”的枚举结果一致；
- `rank(F)=n` 与 `(Target, LA line)` 无重复、无空洞的枚举结果一致；
- `rank(M_p)=r` 且 $L=[0\ I]$ 时，固定每个 Target 后 LA line 严格为 `0,1,2,...`；
- 双射但 LA bit 被置换时，分类为 `valid_non_natural`，而不是 Mapping 失败；
- byte offset 在所有 Mapping 中原样保留。

### 9.2 输入与命令

- 两类 template 输出均可直接被对应命令读取；
- Mapping 和 Scenario 的未知字段、重复字段和错误类型不会被静默忽略；
- `validate`、`map`、`run` 的选项、选择顺序和退出码符合第 4、7 章；
- 所有 corner case 的行为符合第 8 章；
- 任何失败都不会留下部分查询、部分场景或截断输出文件。

### 9.3 性能指标

- Target counts 之和始终等于 $Q$；
- $R_{\max}$、每个 $R_{\mathrm{window}}$ 和 $L_{\mathrm{run}}$ 与定义公式一致；
- 长期 count 相同但访问顺序不同的序列，可以得到相同 $R_{\max}$ 和不同 $R_{\mathrm{window}}$；
- aligned 与 staggered 的多路访问产生确定且可解释的不同结果；
- sweep 的组合顺序、case ID 和每组独立指标符合本规格。

### 9.4 输出

- 不阅读源代码也能从 text 报告判断 Mapping 是否有效；
- warning 和 error 均说明对实际 Mapping 或性能的影响；
- text 与 JSON 表达相同结论；
- JSON 字段、顺序约定、地址编码和比例结构符合第 6 章；
- 相同输入产生相同顺序和相同结果。

## 10. 当前不包含的能力

以下能力不属于 v1：

- Target 数量不是 2 的幂；
- Target 容量不相等；
- 地址空间中存在空洞；
- TOML 或 JSON 配置输入；
- 导入真实硬件 trace；
- 除 `round_robin` 之外的多路调度策略；
- 跨 sweep 组合的聚合指标。

这些能力未来可以扩展，但不应影响当前范围内结果的准确性、确定性和可解释性。
