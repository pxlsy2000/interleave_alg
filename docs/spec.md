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

即使整体 Mapping 是双射，只要上述任一条件不成立，就必须报告“Mapping 有效，但 LA 不满足自然顺序”，不能将它误报为双射失败。

LA bit 置换、XOR 重排或依赖 Target 的偏移可能破坏每个 Target 内的连续性和局部性，因此该 warning 必须醒目展示，并保留在 validate、map 和 run 的输出中；但它不改变 Mapping 双射成立的事实，也不阻止后续分析。

当 $r=0$ 时，$M_p$ 是空矩阵，其秩定义为 0；当 $s=0$ 时，$I_s$ 是空单位矩阵。这两个退化情况仍按同一公式处理。

### 2.5 最终分类

| 分类 | 条件 | 命令结果 |
| --- | --- | --- |
| `valid_natural` | Target 可达、Mapping 双射、LA 自然有序 | 成功 |
| `valid_non_natural` | Target 可达、Mapping 双射、LA 不自然 | 成功并给出 warning |
| `invalid_target_unreachable` | $\operatorname{rank}(M)<r$ | 失败 |
| `invalid_non_bijective` | $\operatorname{rank}(M)=r$，但 $\operatorname{rank}(F)<n$ | 失败 |

如果输入结构本身无效，例如矩阵尺寸错误，则不进行最终分类，而是报告具体输入错误。

## 3. 激励与性能指标

### 3.1 从激励到统计结果

一次性能测试的起点是一条确定、有序的 byte-address 激励序列：

$$
a_0,a_1,\ldots,a_{Q-1}
$$

激励可以来自单个线性访问流，也可以由多个访问流按确定顺序合并。无论来源如何，展开后的一个具体测试都必须得到唯一的地址序列。

Mapping 将每个地址转换为 Target：

$$
y_i=\operatorname{Target}(a_i),
\qquad
0\le i<Q
$$

因此分析器真正统计的是有序 Target 序列：

$$
y_0,y_1,\ldots,y_{Q-1}
$$

相同地址集合以不同顺序出现时，长期访问总量可能相同，但短时拥塞和连续访问可能不同，因此顺序不能丢失。

对每个具体测试，工具独立统计以下四类结果：

| 类别 | 指标 | 回答的问题 |
| --- | --- | --- |
| A | 各 Target 的 count 和 share | 总访问量是否均匀分配 |
| B | 最大负载率 $R_{\max}$ | 长期最繁忙的 Target 偏离理想值多少 |
| C | 短时负载率 $R_{\mathrm{window}}(W)$ | 任意连续窗口内是否出现局部拥塞 |
| D | 最长连续访问 $L_{\mathrm{run}}$ | 请求是否连续成团落到同一 Target |

下面依次给出四类结果的正式定义。设：

- $Q>0$ 为该具体测试的总访问数；
- $N$ 为 Target 数量；
- $y_i\in\{0,\ldots,N-1\}$ 为第 $i$ 次访问对应的 Target。

### 3.2 A：各 Target 访问量

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

### 3.3 B：最大负载率

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

### 3.4 C：短时拥塞

窗口大小 $W$ 的单位是**访问次数**，不是 byte、时间或 cycle。短时拥塞会随观察尺度变化，因此一个具体测试可以同时指定多个 $W$。

例如 `window_sizes: [4, 16, 64]` 表示对同一条 Target 序列分别计算：

- 任意连续 4 次访问中的最差分布；
- 任意连续 16 次访问中的最差分布；
- 任意连续 64 次访问中的最差分布。

三个窗口大小各自产生一条短时拥塞结果。它们不会把 case 展开成三个具体测试，也不会改变原始地址或 Target 序列。

对于每个窗口大小 $W$，必须满足 $1\le W\le Q$。

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

### 3.5 D：最长连续访问

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

### 3.6 sweep 的指标边界

sweep 的每个 `(base, stride)` 组合都是独立的具体测试，各自计算 $Q$、$C_j$、$R_{\max}$、$R_{\mathrm{window}}$ 和 $L_{\mathrm{run}}$。

v1 不定义跨组合聚合指标。

### 3.7 运行限制与确定完成

v1 只接受处于以下闭区间限制内的工作量：

| 资源 | 限制 |
| --- | --- |
| 每个 Mapping 或 Scenario 来源的原始字节数，无论来自文件还是标准输入 | `16 MiB` |
| 一条 `map` 命令的地址操作数 | `1,000,000` |
| `targets.count`（$N$） | `65,536` |
| `address.granule_bytes`（$G$） | $2^{52}$ |
| 单个具体测试的访问数（$Q$） | `10,000,000` |
| 一条 `run` 展开的具体测试数 | `10,000` |
| 一条 `run` 的总生成访问数 $\sum Q$ | `100,000,000` |
| 单个 `multi_stream` case 的 stream 数 | `4,096` |
| 单个 case 的有效窗口大小数 | `1,024` |
| 一份完整报告中跨具体测试合计的 Target 行数 | `1,000,000` |
| 一份完整报告中跨具体测试合计的 window 行数 | `1,000,000` |
| 一条 `run` 的精确窗口工作量 $\sum(Q\cdot K_{\mathrm{effective}})$ | `100,000,000` |

预检使用的每个乘积、求和、$2^A$ 边界、sweep 基数、stream 总数和末地址表达式，都必须先用 checked `u128` 算术求值，再进行已经证明安全的窄化。checked 算术失败与超出相应边界属于同一种确定的限制失败；绝不允许回绕、截断或开始部分分析。

Mapping 限制失败使用 `mapping.unsupported`，退出码为 2。Scenario 或展开后的 run 限制失败使用 `scenario.invalid`，退出码为 3。原始输入过大使用 `input.invalid_value`：Mapping 来源退出 2，Scenario 来源退出 3。`map` 操作数超限也使用 `input.invalid_value`，退出码为 3。只有在所有适用的输入、范围、地址和资源预检都通过后发生的意外失败，才使用 `analysis.failed` 并退出 4。

这些限制属于输入契约，不是尽力而为的目标。处于所有适用限制以内的输入必须确定完成，除非发生 I/O 失败或报告上述限制以内的异常 `analysis.failed`。不得输出部分 Mapping 查询、具体测试或报告。

## 4. 用户输入格式

### 4.1 格式选择

v1 只接受 YAML 1.2 配置文件：

- Mapping 使用一个 YAML 文件；
- Scenario 使用另一个 YAML 文件；
- JSON 只用于结构化输出；
- TOML 和 JSON 输入不属于 v1。

选择 YAML 是因为 XOR tap 列表、场景列表和注释在 YAML 中更便于人工编写和评审。

每个来源必须是 UTF-8，并且只包含一个 YAML document，其根节点必须是块样式（block-style）mapping。即使值符合 schema，流样式根节点也必须拒绝；因此无论文件扩展名是什么、是否使用标准输入，JSON object 都不能作为配置输入。schema 允许 sequence 的位置仍可使用嵌套流样式 sequence，例如本规格中的 tap 和 window 列表。

只允许在来源最开头的三个字节出现一次 UTF-8 BOM。UTF-16 和 UTF-32 BOM、第二个 BOM，以及 byte zero 之后任意位置的 U+FEFF 都必须拒绝。YAML anchor、alias、merge key、所有显式 tag、重复 key、非 string mapping key、多 document 和未知字段都必须拒绝。key 区分大小写，`schema_version` 必须为整数 `1`。

scalar 解析遵循 YAML 1.2。特别地，plain `yes` 和 `on` 是 string：用于 `name` 等 string 字段时有效，用于 `enabled` 等 boolean 字段时属于类型错误。只有 plain `true` 和 `false` 满足 boolean 字段类型。

以 plain YAML scalar 表示的地址字段以及每个命令行地址操作数，只接受以下 lexeme：

```text
decimal: 0|[1-9][0-9]*(?:_[0-9]+)*
hex:     0x[0-9A-Fa-f]+(?:_[0-9A-Fa-f]+)*
```

下划线只能位于同一数值的两个数字之间。开头、结尾、连续两个以及紧邻前缀的下划线均无效。canonical 输出为小写十六进制且不含下划线。

以 plain YAML scalar 表示的通用 integer 字段只接受：

```text
0|[1-9][0-9]*|0x[0-9A-Fa-f]+
```

因此通用 integer 不接受正负号、十进制前导零或下划线。带引号的 numeric text 对地址字段和通用 integer 字段都属于类型错误。

验证遵循以下固定阶梯：

1. 验证命令行语法；
2. 按命令顺序 bounded-read 每个输入，直到 EOF 或读到第 `16 MiB + 1` 个 byte；
3. 读到第 `16 MiB + 1` 个 byte 时立即停止读取，在检查 UTF-8 或 YAML 之前拒绝该来源；
4. 预检输出目的地；
5. 验证 UTF-8 和 YAML 语法/document 规则；
6. 解码并验证整个 document schema，包括未选中的 Scenario case；
7. 验证 Mapping scalar、关系和 v1 cap；
8. 验证矩阵尺寸和 tap；
9. 计算全部三个数学检查；
10. 选择 Scenario case；
11. 解析继承，并验证已选 case 的语义；
12. 预检 checked 展开、资源以及每个查询或生成地址；
13. 执行分析；
14. 渲染完整报告；
15. 原子提交文件输出。

“完整读取”只表示在 16 MiB envelope 内读到 EOF。普通文件、标准输入、FIFO 和 device-like 命名输入都使用同一个 bounded reader，最多保留 `16 MiB + 1` byte。只有观察到 EOF 时，处于限制以内的来源才算完整。读到第 `16 MiB + 1` 个 byte 时，命令立即停止本次读取，不再读取后续输入，并在考虑畸形 UTF-8、BOM、YAML 或输出目的地之前报告 size issue。Mapping 输入始终先于 Scenario 输入。对于大小合格的 snapshot，输出预检或内容解析前必须先取得全部输入；此后目的地错误在内容解析前以退出码 1 结束。

原始大小检查之后，语法、编码、document 和 prohibited-YAML 失败恰好产生一个 `input.yaml_parse` issue。选择来源 byte position 最早的 violation。同一 byte 开始的失败使用以下从高到低的总优先级：invalid encoding/BOM；scanner/parser syntax；flow-style root；explicit tag；anchor；alias；merge key；non-string key；duplicate key；second-document start。non-string key 在 duplicate 处理之前被拒绝，不加入 duplicate-key set，也绝不执行 duplicate equality 比较。缺失 document 的位置定义为 EOF。不得报告更晚或优先级更低的 violation。

schema 解码时，在每个 mapping 中按来源顺序遍历已经出现的 entry。一个已出现 field 或 sequence entry 最多产生第 7.2 节顺序中的首个 failing constraint。有效 container 必须递归处理完毕，然后才移动到下一个已出现 sibling；缺失或类型错误的 parent 抑制虚构的 descendant issue。处理完 mapping 中全部已出现 entry 后，按以下 canonical order 产生缺失 required field：

- Mapping root：`schema_version`、`name`、`address`、`targets`、`mapping`；
- `address`：`width_bits`、`granule_bytes`；
- `targets`：`count`；
- `mapping`：`m`、`l`；
- `m`：`rows`；
- `l`：`mode`、`rows`，其中 `rows` 只在 `mode` 为 `explicit` 时 required；
- Scenario root：`schema_version`、`defaults`、`cases`；
- `defaults`：`accesses`、`window_sizes`；
- `stride` 或 `sweep` case：`name`、`enabled`、`kind`、`window_sizes`、`base_bytes`、`stride_bytes`、`accesses`；只有 `name`、`kind`、`base_bytes` 和 `stride_bytes` intrinsically required，`accesses` 可以继承；
- `multi_stream` case：`name`、`enabled`、`kind`、`window_sizes`、`schedule`、`streams`；
- stream：`name`、`base_bytes`、`stride_bytes`、`accesses`。

optional 和 conditional field 绝不产生 missing issue，除非其 stated condition 使其 required。缺失 container field 在其列出位置使用 container 自身 path，并抑制全部 descendant。数组路径使用从零开始的下标，例如 `cases[2].streams[1].accesses`、`mapping.m.rows[2][1]` 和 `addresses[3]`；命令或命令行选项 issue 使用空路径 `""`。

对于每个 Scenario case，即使 `kind` 缺失、类型错误或不是受支持的 literal，也必须按来源顺序处理 common field `name`、`enabled`、`kind` 和 `window_sizes`。在 `kind` 通过 string 与 allowed-value 两个 gate 之前，recognized-key set 必须是 common name 与全部已声明 kind-specific name 的并集：`base_bytes`、`stride_bytes`、`accesses`、`schedule` 和 `streams`。该并集只能用于判断 key 是否 unknown；必须抑制所有依赖 case kind 的 required、forbidden、type、value、shape、uniqueness、inheritance 和 resource check。common-plus-union set 之外的 key 仍必须产生 `input.unknown_field`。`kind` 有效后，只能使用该 kind 的精确 allowed field 和 constraint。因此，缺失、类型错误或不受支持的 `kind` 不得产生虚构的 `base_bytes`、`stride_bytes`、`schedule`、`streams` 或 forbidden-`accesses` issue。

### 4.2 Mapping 文件

#### 4.2.1 完整示例

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

#### 4.2.2 字段定义

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

#### 4.2.3 LA 模式

`preserve_high` 表示：

$$
L =
\begin{bmatrix}
0_{s\times r} & I_s
\end{bmatrix}
$$

该模式下不允许同时出现 `mapping.l.rows`。

`explicit` 表示使用者逐行给出 $L$。每一行仍然是 GF(2) 上的 XOR 规则，并不是普通整数加法。

配置格式允许 explicit $L$ 使用任意合法 tap。是否可接受由第 2 章的秩与自然顺序检查决定。

以下写法只是把 `preserve_high` 等价地展开为 explicit rows，因此仍然通过自然 LA 检查：

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

下面是一个使用 XOR 重排、但仍保持双射的 explicit $L$：

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

该例只把 LA 的最低两位改为：

$$
\ell_0=x_2\oplus x_3,\qquad
\ell_1=x_3
$$

其余 LA bit 仍直接保留对应高位。LA 高位部分的变换矩阵是可逆的，因此组合矩阵 $F$ 仍满秩：

- Target 可达检查通过；
- Mapping 双射检查通过；
- 因为 $L\ne[0\ I_s]$，LA 自然顺序检查产生 warning；
- 最终分类为 `valid_non_natural`，validate 退出码为 0，map 和 run 可以继续，但必须保留该 warning。

因此，explicit 可以描述并使用自定义 XOR 变换；任何不等于 $[0\ I_s]$ 的 $L$ 都会失去自然 LA，但只要 Target 可达且 $F$ 满秩，Mapping 仍然有效。

### 4.3 Scenario 文件

Scenario 文件描述如何生成第 3.1 节定义的具体测试。文件中的一个 `case` 是使用者声明的场景；case 经过默认值继承和必要的组合展开后，会得到一个或多个具体测试。

一个具体测试必须只有一条确定、有序的 byte-address 序列。第 3 章的所有性能指标都以这条序列为独立计算单位，不跨测试合并。

#### 4.3.1 一个具体测试是什么

以“从 `0x0` 开始、每次增加 64 byte、共访问 4 次”为例，测试过程为：

```text
场景参数
  base = 0x0
  stride = 64
  accesses = 4

生成有序地址序列
  [0x0, 0x40, 0x80, 0xc0]

经过 Mapping 得到有序 Target 序列
  [Target(0x0), Target(0x40), Target(0x80), Target(0xc0)]

对该 Target 序列计算
  各 Target count、R_max、各窗口 R_window、最长 run
```

地址的先后顺序是测试的一部分。同一组地址以不同顺序出现，长期 count 可能相同，但短时窗口和最长 run 可能不同。

工具处理 Scenario 的固定流程为：

1. 根据 `enabled` 和 `--case` 选择 case；
2. 解析每个 case 的默认值；
3. 将 case 展开为一个或多个具体测试；
4. 为每个具体测试生成有序 byte-address 序列；
5. 使用 Mapping 生成对应的 Target 序列；
6. 独立计算并报告每个具体测试的指标。

#### 4.3.2 三类场景如何展开

| kind | 使用者描述的内容 | 展开结果 | 主要用途 |
| --- | --- | --- | --- |
| `stride` | 一个 base、一个 stride、访问次数 | 1 个具体测试 | 观察一种固定线性访问 |
| `sweep` | 多个 base 和多个 stride | 每个 `(base, stride)` 组合各 1 个具体测试 | 比较相位和步长变化 |
| `multi_stream` | 多个 stream 及合并顺序 | 合并为 1 个具体测试 | 观察多主设备或多请求源叠加 |

stride 和 multi-stream 的 `case_id` 等于 case 名称。sweep 的每个组合都有独立 `case_id`，规则见第 4.3.5 节。

#### 4.3.3 公共字段

| 路径 | 类型 | 必填 | 含义 |
| --- | --- | --- | --- |
| `schema_version` | integer | 是 | 固定为 `1` |
| `defaults.accesses` | integer | 是 | stride 和 sweep 的默认访问次数，必须大于 0 |
| `defaults.window_sizes` | integer array | 是 | 默认短时拥塞窗口列表；单位为访问次数 |
| `cases` | case array | 是 | 至少包含一个场景 |
| `cases[].name` | string | 是 | 在文件内唯一，格式见下文 |
| `cases[].enabled` | boolean | 否 | 默认 `true` |
| `cases[].kind` | string | 是 | `stride`、`sweep` 或 `multi_stream` |
| `cases[].window_sizes` | integer array | 否 | 覆盖默认短时拥塞窗口列表 |

窗口列表必须非空、元素唯一且均大于 0。列表中的每个值都作为第 3.4 节公式里的一个独立 $W$；列表本身不增加具体测试数量。

stride 和 sweep 的有效访问次数为 case 自身的 `accesses`，若省略则使用 `defaults.accesses`。所有 case 的有效窗口列表为 case 自身的 `window_sizes`，若省略则使用 `defaults.window_sizes`。窗口合法性针对继承完成后的最终列表检查。

case 名称必须匹配：

```text
[A-Za-z0-9][A-Za-z0-9._-]*
```

该限制保证名称可以直接用于 `--case`，并且不会与 sweep 自动生成的组合 ID 冲突。

#### 4.3.4 stride

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

一个 stride case 只生成一个具体测试，其 `case_id` 等于 `name`。

#### 4.3.5 sweep

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

#### 4.3.6 multi_stream

`schedule` 定义多个 stream 的地址以什么顺序合并成一条最终激励。调度顺序会直接影响短时拥塞和最长连续访问，因此必须显式指定并保证结果可重复。

字段：

| 字段 | 类型 | 必填 | 含义 |
| --- | --- | --- | --- |
| `schedule` | string | 是 | stream 合并策略；v1 只支持 `round_robin` |
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

例如：

```text
master0: [A0, A1, A2]
master1: [B0, B1]

round_robin 结果:
[A0, B0, A1, B1, A2]
```

因此 multi-stream 场景的总访问数为：

$$
Q=\sum_h Q_h
$$

其中 $Q_h$ 是第 $h$ 个 stream 的访问次数。

multi-stream case 不接受 case 级 `accesses` 字段。

一个 multi-stream case 合并后只生成一个具体测试，其 `case_id` 等于 `name`。

#### 4.3.7 完整示例

理解上述测试模型后，一份包含三类场景的完整文件如下：

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
    schedule: round_robin  # 每轮按 streams 声明顺序各取一个地址
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

在三个 case 全部启用时，该文件展开为：

- `sequential`：1 个具体测试；
- `stride-and-phase-sweep`：$4\times3=12$ 个具体测试；
- `two-master`：2 个 stream 合并成 1 个具体测试。

因此工具按确定顺序运行并报告 14 个彼此独立的具体测试。

## 5. 命令行接口

二进制命令名固定为 `interleave`。

### 5.1 通用约定

- `--help` 显示帮助并以退出码 0 结束；
- `--version` 恰好显示 `interleave 0.1.0` 并以退出码 0 结束；
- v1 初始 package version 固定为 `0.1.0`；
- 输入路径 `-` 表示从标准输入读取；
- 输出路径 `-` 表示写入标准输出；
- 未指定 `--output` 时写入标准输出；
- 未指定 `--format` 时使用 `text`；
- `--format` 只接受 `text` 或 `json`；
- 输出文件已存在时默认拒绝覆盖，使用 `--force` 才允许覆盖；
- `--force` 要求 `--output` 是 path-valued，不能省略也不能为 `-`；该路径可以不存在，但已存在的目标必须是普通文件且不能是 symlink；
- 同一条命令最多只能有一个输入文件来自标准输入。

### 5.2 生成模板

```text
interleave template mapping  --output <FILE> [--force]
interleave template scenario --output <FILE> [--force]
```

行为：

- 生成带注释的 YAML；
- 生成结果必须能直接被对应命令读取；
- template 命令不支持 `--format`；
- `--output` 必填。

Mapping template body 恰好如下：

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

Scenario template body 恰好如下：

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

每个 template 都按照 code fence 内展示的 body 原样输出：UTF-8、LF line ending、无 BOM，并且恰好包含一个末尾换行。

### 5.3 验证 Mapping

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

### 5.4 查询地址

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
- `valid_natural` 和 `valid_non_natural` Mapping 都可以查询；
- `valid_non_natural` 的查询报告必须保留 `mapping.non_natural` warning；
- 无效 Mapping 或任一地址越界时不输出部分查询结果。

### 5.5 运行场景

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
- `valid_natural` 和 `valid_non_natural` Mapping 都可以运行；
- `valid_non_natural` 的场景报告必须保留 `mapping.non_natural` warning；
- 运行前验证所有选中场景；任一场景无效时不产生部分分析结果。

每个不存在且被请求的不同名称都产生一个 `scenario.case_not_found` issue，按其在命令行中首次出现的顺序排列，path 为 `""`；重复的不存在名称去重。如果存在任一缺失名称，则不再产生 `scenario.no_case_selected`。否则，最终选择为空时恰好产生一个 `scenario.no_case_selected`。已选 case 中的 issue 按 Scenario 声明顺序排列，再按字段/来源顺序排列。

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

完整的目的地规则如下：

| 结果 | Text | JSON |
| --- | --- | --- |
| 命令行或文件系统失败 | 诊断写入标准错误；不产生报告 | 诊断写入标准错误；不产生报告 |
| 业务成功 | 完整报告写入所选报告目的地 | 一个完整 envelope 写入所选报告目的地 |
| parse、schema、数学检查、预检或分析阶段的业务失败 | 完整失败报告写入标准错误；标准输出为空，输出文件保持不变 | 一个完整失败 envelope 写入所选报告目的地 |
| `output.exists`、无效输出目标或原子输出失败 | 诊断写入标准错误，退出 1；被拒绝的目的地保持不变 | 诊断写入标准错误，退出 1；被拒绝的目的地无法接收 envelope |

本表中的所选报告目的地是：省略 `--output` 或指定 `-` 时为标准输出，否则为命名文件。渲染或报告写入失败属于文件系统/输出失败，绝不产生部分业务报告。

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

使用 `--verbose` 时，完整矩阵插在派生参数与检查结果之间。矩阵中的列始终按
`x0, x1, ..., x(n-1)` 排列；$F$ 的行依次为 $M$ 的所有行和 $L$ 的所有行；
$M_p$ 只包含 $M$ 的前 $r$ 列。

以下是第 4.2.1 节 Mapping 通过验证时的完整 `--verbose` text 输出：

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

如果验证失败，整个失败报告写入标准错误，标准输出为空；不得创建、截断或替换
`--output` 文件。
三个数学层次仍分别展示，以便使用者区分失败发生在哪里。以下示例表示 Target
仍然可达，但 $L$ 中的错误使组合矩阵缺秩，并且 LA 也不再自然：

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

报告先展示 Mapping 信息和验证分类，再按命令行输入顺序输出地址表。每个输入地址
恰好对应一行；`Offset` 是原 byte address 的粒度内偏移，`LA byte` 等于
`G * LA line + Offset`。

例如：

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

所有地址默认使用 canonical lowercase hex：带小写 `0x` 前缀，十六进制数字
使用小写，除 `0x0` 外没有前导零。

#### run

每个展开后的场景单独成节，至少包含：

- case ID；
- 总访问数；
- 每个 Target 的 count 和 share；
- $R_{\max}$；
- 每个窗口的 $R_{\mathrm{window}}$、Target、起点和 count；
- 最长 run 的长度、Target 和起点；
- Mapping 或场景 warning。

报告开头展示 Mapping 信息和验证分类。每个场景节中的 Target 表按 Target ID
升序排列，并包含 count 为 0 的 Target；窗口表按有效 `window_sizes` 的声明
顺序排列。

以下 text 输出与第 6.6 节 JSON 示例表达同一组 `sequential` 分析结果：

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

`path` 使用第 4.1 节定义的从零开始字段路径；问题不对应输入字段时使用空字符串。

JSON object 的 key 顺序不属于契约；本规格明确规定的 array 顺序属于契约。

issue array 首先按验证阶段排列。schema issue 随后遵循第 4.1 节的递归 present-entry 顺序，以及缺失字段的 documented-table fallback 顺序。已选 case 的语义 issue 按 Scenario 声明顺序排列，再按字段/来源顺序排列。

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

`numerator/denominator` 是权威值且不约分；`decimal` 使用精确 integer round-half-up 四舍五入保留 6 位小数，任何 ratio 计算都不使用 floating point。

每个 JSON numeric field 的边界如下：

| Numeric field | 边界 |
| --- | --- |
| 顶层 `schema_version` | 恰好为 `1` |
| `derived.address_width_bits`、`derived.offset_bits`、`derived.line_bits`、`derived.target_bits`、`derived.local_address_bits`；所有 `rank_m`、`rank_f` 和 `rank_m_low` observed/expected | 最大 `64` |
| `derived.granule_bytes` | 最大 $2^{52}$ |
| `derived.target_count` | 最大 `65,536` |
| `target` 中的每个 Target ID，包括 map row、Target row、max-load row、window 和 longest run | 最大 `65,535` |
| 每个 case 的 `accesses`；Target/window `count`；window `size` 和 `start_index`；longest-run `length` 和 `start_index`；share 和 ratio denominator | 最大 `10,000,000` |
| Target-share numerator | 最大 `10,000,000` |
| max-load 和 window-ratio numerator | 最大 `65,536 * 10,000,000 = 655,360,000,000` |

以上枚举了 envelope、validate result、map result 和 run result 中的全部 numeric field。报告行数、测试数和总生成访问数在渲染前受第 3.7 节限制。地址、line address、offset 和 local address 始终为 canonical string，绝不表示为 JSON number。因此每个 JSON integer 都不超过 $2^{53}-1$。

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

精确的数学检查契约如下：

| `id` | `observed` | `expected` | Pass message | Failure message |
| --- | --- | --- | --- | --- |
| `target_reachable` | `{"rank_m":actual}` | `{"rank_m":r}` | `all targets are reachable` | `rank(M)=<actual>, expected <r>` |
| `bijective` | `{"rank_f":actual}` | `{"rank_f":n}` | `mapping is bijective` | `rank(F)=<actual>, expected <n>` |
| `natural_local_address` | `{"rank_m_low":actual,"l_matches_preserve_high":bool}` | `{"rank_m_low":r,"l_matches_preserve_high":true}` | `local address is naturally ordered` | 见下文 |

对于 `natural_local_address`，仅 rank predicate 失败时，failure message 恰好为 `rank(Mp)=<actual>, expected <r>`；仅 $L$ predicate 失败时，恰好为 `rank(Mp)=<actual>; L != [0 I]`；两者都失败时，恰好为 `rank(Mp)=<actual>, expected <r>; L != [0 I]`。predicate 通过时 status 为 `pass`；predicate 失败时，只有 `target_reachable` 和 `bijective` 都通过才为 `warning`，否则为 `fail`。

无效 Mapping 保留全部三个 check object，但恰好只产生一个 primary error。`mapping.target_unreachable` 的优先级高于 `mapping.non_bijective`，path 为 `mapping.m.rows`。primary `mapping.non_bijective` issue 在 explicit $L$ 下使用 `mapping.l.rows`，在 `preserve_high` 下使用 `mapping.m.rows`。

有效但非自然的 Mapping 恰好产生一个 `mapping.non_natural` warning。仅低 $M_p$ rank 失败时 path 为 `mapping.m.rows`；仅 $L$ 与 preserve-high 不一致时为 `mapping.l.rows`；两个 predicate 都失败时为 `mapping`。无效 Mapping 不产生额外的非自然 warning。

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

`targets` 必须包含从 0 到 $N-1$ 的所有 Target，并按 ID 升序排列，包括 count 为 0 的 Target。

`windows` 按 Scenario 中 `window_sizes` 的声明顺序排列。

sweep 的 `case_id` 使用第 4.3.5 节定义的组合 ID，`source_case` 保留原始 case 名称。

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

第 3.7 节的限制分类是穷尽的：Mapping cap 使用 `mapping.unsupported`/2；Scenario 和展开后的 run cap 使用 `scenario.invalid`/3；原始输入和 `map` 操作数 cap 使用 `input.invalid_value`，并根据命令退出 2 或 3。`analysis.failed`/4 只保留给完整预检后发生的限制以内意外失败。

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
| `output.invalid_target` | error | 输出路径已存在，但不是可接受的普通文件目标 |
| `output.atomic_unsupported` | error | 无法提供原子 no-clobber rename |
| `output.io` | error | 报告输出未能完成 |
| `analysis.failed` | error | 输入有效但分析未能完成 |

实现可以增加更具体的问题码，但不能改变上述 code 的含义。

以下 issue message 是稳定 template；尖括号中的项替换为下文定义的 canonical representation：

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
analysis could not be completed
```

`<quoted-key>`、`<quoted-name>` 和 `<quoted-lexeme>` 是完整 JSON string literal，包括开头和结尾的双引号。canonical escaping 为：`"` 变为 `\"`，`\` 变为 `\\`，backspace/form-feed/newline/carriage-return/tab 变为 `\b`、`\f`、`\n`、`\r`、`\t`，其余 U+0000 到 U+001F scalar 变为小写 `\u00xx`。其他所有 Unicode scalar 以 UTF-8 literal 原样输出；`/` 不转义。

`<canonical-value>` 恰好是以下之一：`missing`；使用上述 escaping 的完整 JSON string literal；无分组符、无前导零且负数恰好带一个前导 `-` 的 decimal integer；`true`；`false`；`null`；`sequence`；或 `mapping`。`<count>` 和 `<A>` 是非负、无分组符的 decimal integer。`<canonical>` 是第 6.3 节的 canonical lowercase hexadecimal address。所有 placeholder 都不使用 locale-dependent formatting，也不增加额外引号。

在 `unsupported` message 中，`<field>` 是不带引号的精确 issue `path`。因此对于 `mapping.unsupported`，它恰好是 `targets.count` 或 `address.granule_bytes`。在 conditional constraint 中，`<field>` 是不带引号、使用从零开始语法的 controlling field 完整 path。缺失字段的 issue path 是该缺失字段的完整 path，observed `<canonical-value>` 为 `missing`。

`<constraint>` 是有限词汇，必须恰好为以下形式之一：

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
string matching <quoted-regex>
non-empty string without control or line-separator characters
field absent when <field>=<canonical-value>
field present when <field>=<canonical-value>
at most <n> raw bytes
at most <n> query addresses
sum(Q*K) <= 100000000
```

constraint 中代入的所有数字都使用无分组符 decimal。`<compact-JSON-string-array>` 不含空格，每个元素都使用 canonical JSON string 形式，例如 `["preserve_high","explicit"]`、`["stride","sweep","multi_stream"]` 或 `["round_robin"]`。`<quoted-regex>` 是完整 canonical JSON string literal，例如 `"[A-Za-z0-9][A-Za-z0-9._-]*"`。

对于每个已出现 field 或 sequence entry，适用 constraint 及其顺序必须由下文 normative emitter matrix 决定，而不是由 vocabulary 的文本顺序决定。parent 和 type gate 之后，每个 field 只能产生首个 failure。intrinsically required field 缺失时使用 `required field`；conditionally forbidden 或 required field 必须使用 matrix 中精确的 `field absent when ...` 或 `field present when ...` constraint。只有 container gate 通过后，才能按来源顺序处理 sequence element。

`targets.count` 和 `address.granule_bytes` 必须使用一个覆盖所有 message family 的顺序，并覆盖其他 constraint/reason 顺序。`integer` 和 `plain integer` 成功后，依次求值：(1) 非 2 的幂，使用 field-specific not-a-power-of-two reason 产生 `mapping.unsupported`；(2) intrinsic relation，即 `targets.count <= 2^n` 或 `address.granule_bytes <= 2^A`，使用 `integer <= <max>` 产生 `input.invalid_value`；(3) v1 cap，使用 field-specific exceeds-limit reason 产生 `mapping.unsupported`。必须在首个 result 停止。`expected power of two, observed ...` 分支不适用于这两个 field。

`<reason>` 是有限词汇，必须恰好为以下之一：

```text
target count is not a power of two
granule size is not a power of two
target count exceeds v1 limit 65536
granule size exceeds v1 limit 4503599627370496
```

两个 target-count reason 的 `<field>` 始终为 `targets.count`；两个 granule-size reason 的 `<field>` 始终为 `address.granule_bytes`。如果一个 field 同时适用多个 reason，则按上述 reason list 顺序选择首个。constraint 或 reason 都不包含用户提供的 text。

#### 规范性 Validation Emitter Matrix

本 matrix 穷举 v1 的 input 与 validation diagnostic。每一行都是一条 emitter rule；scope cell 列出多个精确 path 或 path pattern 时，该行必须独立应用于每个 match。同一 path 的顺序只能由 gate/order column 决定。prerequisite 失败必须抑制全部 dependent row。类型错误的 container 使用其 actual canonical value；collection content、length 和 uniqueness failure 使用 `sequence`；scalar failure 使用该 scalar 的 canonical value；缺失使用 `missing`。derived-count row 使用其命名的无分组 decimal count。checked-arithmetic failure 使用对应 limit 加一作为 observed count，并使用同一行。`2^A`、`2^n`、`r`、`s`、`n-1` 及其他 dynamic substitution 必须使用已经验证的无分组 decimal value。

YAML 与 common input emitter：

| scope/path pattern | gate/order | failure | code | message template | constraint or reason | observed value |
| --- | --- | --- | --- | --- | --- | --- |
| Mapping 或 Scenario source，`""` | raw-size gate，第一 | 读到 byte `16777217` | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `at most 16777216 raw bytes` | `16777217` |
| source，`""` | earliest byte，priority 1 | invalid encoding 或 prohibited BOM | `input.yaml_parse` | `invalid YAML syntax` | — | — |
| source，`""` | earliest byte，priority 2 | scanner/parser syntax | `input.yaml_parse` | `invalid YAML syntax` | — | — |
| source，`""` | earliest byte，priority 3 | flow-style root | `input.yaml_parse` | `invalid YAML syntax` | — | — |
| source，`""` | earliest byte，priority 4 | explicit tag | `input.yaml_parse` | `invalid YAML syntax` | — | — |
| source，`""` | earliest byte，priority 5 | anchor | `input.yaml_parse` | `invalid YAML syntax` | — | — |
| source，`""` | earliest byte，priority 6 | alias | `input.yaml_parse` | `invalid YAML syntax` | — | — |
| source，`""` | earliest byte，priority 7 | merge key | `input.yaml_parse` | `invalid YAML syntax` | — | — |
| source，`""` | earliest byte，priority 8 | non-string key | `input.yaml_parse` | `invalid YAML syntax` | — | — |
| 第二次出现的完整 path | earliest byte，priority 9 | duplicate string key | `input.yaml_parse` | `duplicate key <quoted-key>` | — | quoted duplicate key |
| source，`""` | earliest byte，priority 10 | 第二个或后续 document | `input.yaml_parse` | `expected exactly one YAML document, found <count>` | — | document count |
| source，`""` | EOF position | 无 document | `input.yaml_parse` | `expected exactly one YAML document, found <count>` | — | `0` |
| unrecognized key 的精确完整 path | YAML gate 后，按 containing mapping 来源顺序 | key 不在 allowed set | `input.unknown_field` | `unknown field <quoted-key>` | — | quoted final key |

Mapping schema emitter：

| scope/path pattern | gate/order | failure | code | message template | constraint or reason | observed value |
| --- | --- | --- | --- | --- | --- | --- |
| `""` | Mapping schema，1 | document root 不是 mapping | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `mapping` | actual canonical value |
| `schema_version`、`name`、`address`、`targets`、`mapping`、`mapping.m`、`mapping.l`、`mapping.m.rows` | parent gate，canonical missing order | intrinsically required field 缺失 | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `required field` | `missing` |
| `address.width_bits`、`address.granule_bytes`、`targets.count`、`mapping.l.mode` | parent gate，canonical missing order | intrinsically required field 缺失 | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `required field` | `missing` |
| `address`、`targets`、`mapping`、`mapping.m`、`mapping.l` | field 已出现后 | value 不是 mapping | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `mapping` | actual canonical value |
| `schema_version` | 1 | value 不是 integer | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `integer` | actual canonical scalar |
| `schema_version` | 2 | scalar 不是 plain generic-integer lexeme | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `plain integer` | actual canonical scalar |
| `schema_version` | 3 | value 不是 `1` | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `integer in [1,1]` | actual integer |
| `name` | 1 | value 不是 string | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `string` | actual canonical scalar |
| `name` | 2 | value 为空或包含 control/line-separator character | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `non-empty string without control or line-separator characters` | actual JSON string |
| `address.width_bits` | 1 | value 不是 integer | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `integer` | actual canonical scalar |
| `address.width_bits` | 2 | scalar 不是 plain generic-integer lexeme | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `plain integer` | actual canonical scalar |
| `address.width_bits` | 3 | value 不在 `1..64` | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `integer in [1,64]` | actual integer |
| `address.granule_bytes` | scalar gate 1 | value 不是 integer | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `integer` | actual canonical scalar |
| `address.granule_bytes` | scalar gate 2 | scalar 不是 plain generic-integer lexeme | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `plain integer` | actual canonical scalar |
| `address.granule_bytes` | cross-family 1 | integer 不是 2 的幂 | `mapping.unsupported` | `unsupported <field>: <reason>` | `granule size is not a power of two` | actual integer |
| `address.granule_bytes` | cross-family 2；要求有效 `address.width_bits` | $G>2^A$ | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `integer <= <2^A>` | actual integer |
| `address.granule_bytes` | cross-family 3 | $G>4503599627370496$ | `mapping.unsupported` | `unsupported <field>: <reason>` | `granule size exceeds v1 limit 4503599627370496` | actual integer |
| `targets.count` | scalar gate 1 | value 不是 integer | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `integer` | actual canonical scalar |
| `targets.count` | scalar gate 2 | scalar 不是 plain generic-integer lexeme | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `plain integer` | actual canonical scalar |
| `targets.count` | cross-family 1 | integer 不是 2 的幂 | `mapping.unsupported` | `unsupported <field>: <reason>` | `target count is not a power of two` | actual integer |
| `targets.count` | cross-family 2；要求有效 $n$ | $N>2^n$ | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `integer <= <2^n>` | actual integer |
| `targets.count` | cross-family 3 | $N>65536$ | `mapping.unsupported` | `unsupported <field>: <reason>` | `target count exceeds v1 limit 65536` | actual integer |
| `mapping.l.mode` | 1 | value 不是 string | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `string` | actual canonical scalar |
| `mapping.l.mode` | 2 | value 不是 supported mode | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `one of ["preserve_high","explicit"]` | actual JSON string |
| `mapping.l.rows` | valid mode，conditional 1 | mode 为 `preserve_high` 时出现 | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `field absent when mapping.l.mode="preserve_high"` | actual canonical value |
| `mapping.l.rows` | valid mode，conditional 1 | mode 为 `explicit` 时缺失 | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `field present when mapping.l.mode="explicit"` | `missing` |
| `mapping.m.rows`、`mapping.l.rows` | presence/conditional gate 后 | value 不是 sequence | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `sequence` | actual canonical value |
| `mapping.m.rows` | 有效 $r$，sequence gate 后 | row count 不是 $r$ | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `sequence length <r>` | `sequence` |
| `mapping.l.rows` | 有效 $s$、explicit mode、sequence gate 后 | row count 不是 $s$ | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `sequence length <s>` | `sequence` |
| `mapping.m.rows[i]`、`mapping.l.rows[i]` | row 来源顺序，1 | row 不是 sequence | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `sequence` | actual canonical value |
| `mapping.m.rows[i]`、`mapping.l.rows[i]` | row 来源顺序，2 | row 重复 tap | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `unique values` | `sequence` |
| `mapping.m.rows[i][j]`、`mapping.l.rows[i][j]` | tap 来源顺序，1 | tap 不是 integer | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `integer` | actual canonical scalar |
| `mapping.m.rows[i][j]`、`mapping.l.rows[i][j]` | tap 来源顺序，2 | tap 不是 plain generic-integer lexeme | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `plain integer` | actual canonical scalar |
| `mapping.m.rows[i][j]`、`mapping.l.rows[i][j]` | 有效 $n$，tap 来源顺序，3 | tap 不在 `0..n-1` | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `integer in [0,<n-1>]` | actual integer |

Scenario schema emitter：

| scope/path pattern | gate/order | failure | code | message template | constraint or reason | observed value |
| --- | --- | --- | --- | --- | --- | --- |
| `""` | Scenario schema，1 | document root 不是 mapping | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `mapping` | actual canonical value |
| `schema_version`、`defaults`、`cases`、`defaults.accesses`、`defaults.window_sizes` | parent gate，canonical missing order | intrinsically required field 缺失 | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `required field` | `missing` |
| `cases[i].name`、`cases[i].kind` | common-field canonical missing order | intrinsically required field 缺失 | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `required field` | `missing` |
| `cases[i].base_bytes`、`cases[i].stride_bytes` | valid `stride` 或 `sweep`，kind-specific missing order | intrinsically required field 缺失 | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `required field` | `missing` |
| `cases[i].schedule`、`cases[i].streams` | valid `multi_stream`，kind-specific missing order | intrinsically required field 缺失 | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `required field` | `missing` |
| `cases[i].streams[j].name`、`cases[i].streams[j].base_bytes`、`cases[i].streams[j].stride_bytes`、`cases[i].streams[j].accesses` | valid `multi_stream`，stream missing order | intrinsically required field 缺失 | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `required field` | `missing` |
| `defaults`、`cases[i]`、`cases[i].streams[j]` | presence 与 valid parent shape 后 | value 不是 mapping | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `mapping` | actual canonical value |
| `schema_version` | 1 | value 不是 integer | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `integer` | actual canonical scalar |
| `schema_version` | 2 | scalar 不是 plain generic-integer lexeme | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `plain integer` | actual canonical scalar |
| `schema_version` | 3 | value 不是 `1` | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `integer in [1,1]` | actual integer |
| `defaults.accesses`、`stride`/`sweep` 的 `cases[i].accesses`、`cases[i].streams[j].accesses` | 1 | value 不是 integer | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `integer` | actual canonical scalar |
| 相同 accesses path | 2 | scalar 不是 plain generic-integer lexeme | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `plain integer` | actual canonical scalar |
| 相同 accesses path | 3 | value 不在 `1..10000000` | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `integer in [1,10000000]` | actual integer |
| `defaults.window_sizes`、`cases[i].window_sizes` | 1 | value 不是 sequence | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `sequence` | actual canonical value |
| 相同 window-list path | 2 | sequence 为空 | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `non-empty sequence` | `sequence` |
| 相同 window-list path | 3 | sequence 重复 value | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `unique values` | `sequence` |
| `defaults.window_sizes[j]`、`cases[i].window_sizes[j]` | 1 | entry 不是 integer | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `integer` | actual canonical scalar |
| 相同 window-entry path | 2 | entry 不是 plain generic-integer lexeme | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `plain integer` | actual canonical scalar |
| 相同 window-entry path | 3 | entry 不在 `1..10000000` | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `integer in [1,10000000]` | actual integer |
| effective window-entry source path | selected case，inheritance 后且有效 effective $Q$ | $W>Q$ | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `integer <= <Q>` | actual $W$ |
| `cases` | 1 | value 不是 sequence | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `sequence` | actual canonical value |
| `cases` | 2 | sequence 为空 | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `non-empty sequence` | `sequence` |
| `cases[i]` | case 来源顺序 | entry 不是 mapping | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `mapping` | actual canonical value |
| `cases[i].name`、`cases[i].streams[j].name` | 1 | value 不是 string | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `string` | actual canonical scalar |
| 相同 name path | 2 | value 不符合 name grammar | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `string matching "[A-Za-z0-9][A-Za-z0-9._-]*"` | actual JSON string |
| `cases` | 所有有效 case name 后 | case name 不唯一 | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `unique values` | `sequence` |
| `cases[i].streams` | 所有有效 stream name 后 | stream name 不唯一 | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `unique values` | `sequence` |
| `cases[i].enabled` | common field，出现时 | value 不是 boolean | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `boolean` | actual canonical scalar |
| `cases[i].kind` | common field，1 | value 不是 string | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `string` | actual canonical scalar |
| `cases[i].kind` | common field，2 | value 不是 supported kind | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `one of ["stride","sweep","multi_stream"]` | actual JSON string |
| valid `stride` 的 `cases[i].base_bytes`、`cases[i].stride_bytes` | kind gate，field 来源顺序 | scalar 不是 plain address | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `plain address` | actual canonical scalar |
| valid `sweep` 的 `cases[i].base_bytes`、`cases[i].stride_bytes` | kind gate，1 | value 不是 sequence | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `sequence` | actual canonical value |
| 相同 sweep path | kind gate，2 | sequence 为空 | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `non-empty sequence` | `sequence` |
| 相同 sweep path | kind gate，3 | sequence 重复 value | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `unique values` | `sequence` |
| valid `sweep` 的 `cases[i].base_bytes[j]`、`cases[i].stride_bytes[j]` | entry 来源顺序 | entry 不是 plain address | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `plain address` | actual canonical scalar |
| `cases[i].schedule` | valid `multi_stream`，1 | value 不是 string | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `string` | actual canonical scalar |
| `cases[i].schedule` | valid `multi_stream`，2 | value 不是 `round_robin` | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `one of ["round_robin"]` | actual JSON string |
| `cases[i].streams` | valid `multi_stream`，1 | value 不是 sequence | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `sequence` | actual canonical value |
| `cases[i].streams` | valid `multi_stream`，2 | sequence 为空 | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `non-empty sequence` | `sequence` |
| `cases[i].streams[j].base_bytes`、`cases[i].streams[j].stride_bytes` | valid stream，field 来源顺序 | scalar 不是 plain address | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `plain address` | actual canonical scalar |
| `cases[i].accesses` | valid `multi_stream`，field 来源顺序 | forbidden case-level field 出现 | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `field absent when cases[i].kind="multi_stream"` | actual canonical value |

command、semantic 与 resource emitter：

| scope/path pattern | gate/order | failure | code | message template | constraint or reason | observed value |
| --- | --- | --- | --- | --- | --- | --- |
| `addresses[i]` | command operand 来源顺序，1 | operand 不是 plain address | `address.invalid` | `invalid address <quoted-lexeme>` | `plain address` | quoted original lexeme |
| `addresses[i]` | command operand 来源顺序，2；有效 $A$ | address 不小于 $2^A$ | `address.out_of_range` | `address <canonical> is outside the <A>-bit range` | `integer <= <2^A-1>` | canonical address |
| `""` | CLI grammar 后 | query-address count 超过 `1000000` | `input.invalid_value` | `expected <constraint>, observed <canonical-value>` | `at most 1000000 query addresses` | actual query count |
| 从 `cases[i]` 或 `cases[i].streams[j]` 产生的 address | checked expansion 后，按 source test/address 顺序 | address 不小于 $2^A$ | `address.out_of_range` | `address <canonical> is outside the <A>-bit range` | `integer <= <2^A-1>` | canonical address |
| effective-window source path | selected case，inheritance 后 | effective window count 超过 `1024` | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `integer <= 1024` | actual window count |
| `cases[i].streams` | selected case 且 valid `multi_stream` shape | stream count 超过 `4096` | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `integer <= 4096` | actual stream count |
| `cases[i]` | selected case，inheritance/stream sum 后 | concrete-test $Q$ 超过 `10000000` | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `integer <= 10000000` | actual $Q$ |
| `cases` | selected-case expansion 后 | concrete-test count 超过 `10000` | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `integer <= 10000` | actual test count |
| `cases` | selected-case expansion 后 | $\sum Q$ 超过 `100000000` | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `integer <= 100000000` | actual $\sum Q$ |
| `cases` | selected-case expansion 后 | Target report row 超过 `1000000` | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `integer <= 1000000` | actual Target-row count |
| `cases` | selected-case expansion 后 | window report row 超过 `1000000` | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `integer <= 1000000` | actual window-row count |
| `cases` | selected-case expansion 后 | $\sum(Q\cdot K_{\mathrm{effective}})>100000000$ | `scenario.invalid` | `expected <constraint>, observed <canonical-value>` | `sum(Q*K) <= 100000000` | actual checked sum |
| `mapping.m.rows` | mathematical check 后 | $\operatorname{rank}(M)<r$ | `mapping.target_unreachable` | `rank(M)=<actual>, expected <r>` | — | 第 6.4 节精确 check object |
| explicit L 使用 `mapping.l.rows`；preserve-high 使用 `mapping.m.rows` | target reachable 后再进行 mathematical check | $\operatorname{rank}(F)<n$ | `mapping.non_bijective` | `rank(F)=<actual>, expected <n>` | — | 第 6.4 节精确 check object |
| 第 6.4 节指定的 `mapping.m.rows`、`mapping.l.rows` 或 `mapping` | 前两个 check 通过 | natural-order predicate 失败 | `mapping.non_natural` | 第 6.4 节适用的精确 natural failure message | — | 第 6.4 节精确 check object |
| `""` | requested name 的首次 CLI occurrence 顺序 | distinct requested case name 缺失 | `scenario.case_not_found` | `case <quoted-name> was not found` | — | quoted requested name |
| `""` | case-name lookup 后 | final selection 为空且没有 missing-name issue | `scenario.no_case_selected` | `no scenario case was selected` | — | — |

对于 Scenario case unknown-key detection，在 `kind` 有效前，allowed-name union 必须恰好为 `name`、`enabled`、`kind`、`window_sizes`、`base_bytes`、`stride_bytes`、`accesses`、`schedule` 和 `streams`，并且必须抑制 kind-dependent matrix row。`kind` 有效后，`stride` 和 `sweep` 允许四个 common name 加 `base_bytes`、`stride_bytes` 与 `accesses`；`multi_stream` 允许 common name 加 `schedule` 与 `streams`，同时只为显式 forbidden-field emitter 识别 `accesses`。其他 key 必须使用 common `input.unknown_field` row。

畸形 YAML 语法以及每种 prohibited syntax/document form 按第 4.1 节的 earliest-byte 规则竞争，并且只产生一个 `input.yaml_parse`。重复 key 和 document 数量 winner 使用上面的专用 template；其他 syntax/document winner 使用 `invalid YAML syntax`。未知字段使用 `input.unknown_field` 和 `unknown field` template。G/N cross-family row 对非 2 的幂与 v1-cap failure 都必须使用 `unsupported` template；其他 field、range 和 constraint row 必须使用 matrix 指定的 template。

### 7.3 原子性

Linux `x86_64-unknown-linux-gnu` 是 v1 文件系统 baseline。输入和输出 transaction 遵循以下全部规则：

1. 对每个普通文件、标准输入、FIFO 或 device-like 命名输入使用第 4.1 节的 bounded reader。读到第 `16 MiB + 1` 个 byte 时停止；size error 先于 UTF-8/YAML 和输出预检。对于已读到 EOF 的合格普通文件输入 snapshot，保留 device 和 inode identity。
2. `--force` 要求 path-valued output；省略 output 或指定 `-` 属于 usage error。在不跟随最终 symlink 的情况下检查最终路径。不存在的路径允许使用。无论是否有 `--force`，已存在的 symlink 或非普通文件都使用 `output.invalid_target`/1 拒绝。已存在的普通输出与每个普通文件输入通过 device 和 inode 比较，而不是比较路径拼写；同一文件或 hard-link alias 只有在 bounded input snapshot 已读到 EOF 后配合 `--force` 才允许。
3. 创建目的地 transaction 之前渲染完整报告。在输出所在目录以 `O_CREAT|O_EXCL`、mode `0666 & umask` 创建普通临时文件；写入完整字节，flush userspace buffer，然后 close。
4. 不带 `--force` 时，用 `renameat2(RENAME_NOREPLACE)` 提交。如果目的地此时已存在，报告 `output.exists`/1。如果 syscall 或文件系统无法提供原子 no-clobber rename，报告 `output.atomic_unsupported`/1；不允许 link/unlink 或其他较弱 fallback。
5. 带 `--force` 时，在不跟随最终 symlink 的情况下重新检查最终路径。如果仍不存在，则用 atomic rename 创建；如果是已存在的普通文件，则用 atomic rename 替换。重新检查发现 symlink 或非普通目标时拒绝。
6. commit 前任何失败都删除本次唯一临时文件。每个拒绝或失败操作都使旧目的地逐字节保持不变，并且不留下临时残留。

新文件获得临时文件的 `0666 & umask` mode。替换不保留旧目的地的权限、ownership 或其他 metadata。v1 不承诺 `fsync` crash-durability，也不保证输出目录遭到敌对并发修改时的正确性。

`map` 在计算前验证 Mapping 和全部查询地址。`run` 在分析前验证 Mapping、已选场景、展开资源及所有将生成的地址。任一预检失败都不产生部分地址或场景结果，任一失败的文件 transaction 都不留下截断目标。

## 8. Corner case 的确定行为

### 8.1 Mapping

| 场景 | 行为 |
| --- | --- |
| `targets.count = 1` | 合法；$r=0$，`mapping.m.rows` 必须为空 |
| $N=2^n$ | 合法；$s=0$，LA line 恒为 0 |
| `granule_bytes = 1` | 合法；$g=0$，byte offset 恒为 0 |
| `granule_bytes > 2^A` | Mapping 输入错误 |
| `granule_bytes > 2^52` 但仍满足数学关系 | `mapping.unsupported`，退出码 2 |
| Target 数量、粒度不是 2 的幂 | 报告 `mapping.unsupported`，退出码 2 |
| `targets.count > 65,536` 但在其他方面有意义 | `mapping.unsupported`，退出码 2 |
| Target 数量超过 line-address 组合数 | Mapping 输入错误 |
| tap 为负数或 `tap >= n` | Mapping 输入错误，并给出准确字段路径 |
| 同一行出现重复 tap | Mapping 输入错误，不按 XOR 抵消处理 |
| `M` 或 explicit `L` 行数错误 | Mapping 输入错误 |
| `preserve_high` 同时提供 rows | Mapping 输入错误 |
| `explicit` 未提供 rows | Mapping 输入错误 |
| 矩阵合法但 Target 不可达 | `invalid_target_unreachable` |
| Target 可达但 $F$ 不满秩 | `invalid_non_bijective` |
| $F$ 满秩但 LA 不自然 | `valid_non_natural` warning，退出码 0，允许 map 和 run |

### 8.2 地址和数值

| 场景 | 行为 |
| --- | --- |
| 地址恰好为 $2^A-1$ | 合法 |
| 地址等于或大于 $2^A$ | 越界错误 |
| 负地址 | 输入错误 |
| 地址文本含正负号、十进制前导零或无效下划线位置 | `address.invalid`；不产生部分查询结果 |
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
| 超出第 3.7 节任一 case 级或 run 级资源 cap | 分析前报告 `scenario.invalid`，退出码 3 |
| 单个具体测试中 $Q=10,000,000$ 且 $K_{\mathrm{effective}}=1,024$ | 因 $QK=10,240,000,000>100,000,000$ 而拒绝 |

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
- 双射但 LA bit 被置换时，分类为 `valid_non_natural`，validate、map 和 run 均保留 warning；
- byte offset 在所有 Mapping 中原样保留。

### 9.2 输入与命令

- 两类 template 输出均可直接被对应命令读取；
- Mapping 和 Scenario 的未知字段、重复字段和错误类型不会被静默忽略；
- `validate`、`map`、`run` 的选项、选择顺序和退出码符合第 5、7 章；
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

这些能力未来可以扩展，但不得影响当前范围内结果的准确性、确定性和可解释性。
